#![deny(deprecated)]
#![feature(assert_matches)]
#![feature(specialization)]
#![feature(exact_size_is_empty)]
#![feature(exclusive_range_pattern)]
#![feature(let_chains)]
#![feature(box_patterns)]
#![feature(box_into_inner)]
#![feature(string_extend_from_within)]
#![recursion_limit = "256"]
#![feature(map_try_insert)]
#![feature(get_mut_unchecked)]
#![feature(stmt_expr_attributes)]
#![feature(slice_take)]
#![feature(write_all_vectored)]

// mod rewrite;
/// Implementation of various behavior for the tokens of cpclib_tokens
pub mod implementation;

/// All the stuff to parse z80 code.
pub mod parser;

/// Production of the bytecodes from the tokens.
pub mod assembler;

pub mod disass;

pub mod preamble;

pub mod error;

mod crunchers;

pub mod orgams;
pub mod progress;

use std::fmt::Debug;
use std::io::Write;
use std::sync::{Arc, RwLock};

use cpclib_disc::amsdos::*;
use cpclib_sna::Snapshot;
use enumflags2::{BitFlags, bitflags};
use preamble::function::FunctionBuilder;
use preamble::processed_token::ProcessedToken;
pub use preamble::*;

use self::listing_output::ListingOutput;

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AssemblingOptionFlags {
    /// Set to consider that the assembler pay attention to the case of the labels
    CaseSensitive,
    // Set to include SYMB in sna chunks
    SnaSymb,
    // Set to include BRKS in sna chunks
    SnaBrks,
    // Set to include BRKC in sna chunks
    SnaBrkc,
    // Set to include REMU in sna chunks
    SnaRemu,
    // Save remu chunk in a file
    RemuInFile,
    // Save wabp chunck in a file
    WabpInFile,
    // generate breakpoint as code
    BreakpointAsOpcode
}

impl AssemblingOptionFlags {
    pub fn from_chunk(chunk: &str) -> Option<Self> {
        match chunk {
            "SYMB" => Some(Self::SnaSymb),
            "BRKS" => Some(Self::SnaBrks),
            "BRKC" => Some(Self::SnaBrkc),
            "REMU" => Some(Self::SnaRemu),
            _ => None
        }
    }
}

/// Configuration of the assembler. By default the assembler is case sensitive and has no symbol
#[derive(Debug, Clone)]
pub struct AssemblingOptions {
    flags: BitFlags<AssemblingOptionFlags>,

    /// Contains some symbols that could be used during assembling
    symbols: cpclib_tokens::symbols::SymbolsTable,
    output_builder: Option<Arc<RwLock<ListingOutput>>>,
    /// The snapshot may be prefiled with a dedicated snapshot
    snapshot_model: Option<Snapshot>,
    amsdos_behavior: AmsdosAddBehavior,
    enable_warnings: bool,
    force_void: bool,
    debug: bool
}

impl Default for AssemblingOptions {
    fn default() -> Self {
        Self {
            flags: AssemblingOptionFlags::CaseSensitive
                | AssemblingOptionFlags::SnaBrkc
                | AssemblingOptionFlags::SnaBrks
                | AssemblingOptionFlags::SnaSymb
                | AssemblingOptionFlags::SnaRemu,
            symbols: cpclib_tokens::symbols::SymbolsTable::default(),
            output_builder: None,
            snapshot_model: None,
            amsdos_behavior: AmsdosAddBehavior::FailIfPresent,
            enable_warnings: true,
            force_void: true,
            debug: false
        }
    }
}

#[allow(missing_docs)]
impl AssemblingOptions {
    pub fn new_case_sensitive() -> Self {
        Self::default()
    }

    pub fn new_case_insensitive() -> Self {
        let mut options = Self::new_case_sensitive();
        options.set_case_sensitive(false);
        options
    }

    /// Creation an option object with the given symbol table
    pub fn new_with_table(symbols: &cpclib_tokens::symbols::SymbolsTable) -> Self {
        let mut options = Self::default();
        options.set_symbols(symbols);
        options
    }

    pub fn set_flag(&mut self, flag: AssemblingOptionFlags, val: bool) -> &mut Self {
        self.flags.set(flag, val);
        self
    }

    pub fn get_flag(&self, flag: AssemblingOptionFlags) -> bool {
        self.flags.contains(flag)
    }

    pub fn disable_warnings(&mut self) -> &mut Self {
        self.enable_warnings = false;
        self
    }

    /// Specify if the assembler must be case sensitive or not
    pub fn set_case_sensitive(&mut self, val: bool) -> &mut Self {
        self.set_flag(AssemblingOptionFlags::CaseSensitive, val);
        self
    }

    pub fn set_save_behavior(&mut self, behavior: AmsdosAddBehavior) -> &mut Self {
        self.amsdos_behavior = behavior;
        self
    }

    pub fn set_snapshot_model(&mut self, mut sna: Snapshot) -> &mut Self {
        sna.unwrap_memory_chunks();
        self.snapshot_model = Some(sna);
        self
    }

    /// Specify a symbol table to copy
    pub fn set_symbols(&mut self, val: &cpclib_tokens::symbols::SymbolsTable) -> &mut Self {
        self.symbols = val.clone();
        self
    }

    pub fn symbols(&self) -> &cpclib_tokens::symbols::SymbolsTable {
        &self.symbols
    }

    pub fn symbols_mut(&mut self) -> &mut cpclib_tokens::symbols::SymbolsTable {
        &mut self.symbols
    }

    pub fn case_sensitive(&self) -> bool {
        self.get_flag(AssemblingOptionFlags::CaseSensitive)
    }

    pub fn debug(&self) -> bool {
        self.debug
    }

    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    pub fn snapshot_model(&self) -> Option<&Snapshot> {
        self.snapshot_model.as_ref()
    }

    pub fn save_behavior(&self) -> AmsdosAddBehavior {
        self.amsdos_behavior
    }

    pub fn force_void(&self) -> bool {
        self.force_void
    }

    pub fn set_force_void(&mut self, force_void: bool) -> &mut Self {
        self.force_void = force_void;
        self
    }

    pub fn write_listing_output<W: 'static + Write + Send + Sync>(
        &mut self,
        writer: W
    ) -> &mut Self {
        self.output_builder = Some(Arc::new(RwLock::new(ListingOutput::new(writer))));
        if let Some(b) = self.output_builder.as_mut() {
            b.write().unwrap().on()
        }
        self
    }
}

/// Assemble a piece of code and returns the associated list of bytes.
pub fn assemble(code: &str) -> Result<Vec<u8>, AssemblerError> {
    let options = EnvOptions::default();
    // let options = AssemblingOptions::new_with_table(table);
    assemble_with_options(code, options).map(|(bytes, _symbols)| bytes)
}

/// Assemble a piece of code and returns the associates liste of bytes as well as the generated reference table.
pub fn assemble_with_options(
    code: &str,
    options: EnvOptions
) -> Result<(Vec<u8>, cpclib_tokens::symbols::SymbolsTable), AssemblerError> {
    let builder = options.parse_options().clone().context_builder();
    let tokens = parser::parse_z80_with_context_builder(code, builder)?;
    assemble_tokens_with_options(&tokens, options)
}

/// Assemble the predifined list of tokens
pub fn assemble_tokens_with_options<
    'tokens,
    T: 'static + Visited + ToSimpleToken + Clone + ListingElement + Sync + MayHaveSpan
>(
    tokens: &'tokens [T],
    options: EnvOptions
) -> Result<(Vec<u8>, cpclib_tokens::symbols::SymbolsTable), AssemblerError>
where
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
    <<T as cpclib_tokens::ListingElement>::TestKind as cpclib_tokens::TestKindElement>::Expr:
        implementation::expression::ExprEvaluationExt,
    ProcessedToken<'tokens, T>: FunctionBuilder
{
    let (_tok, env) = assembler::visit_tokens_all_passes_with_options(tokens, options)
        .map_err(|(_, _, e)| AssemblerError::AlreadyRenderedError(e.to_string()))?;
    Ok((env.produced_bytes(), env.symbols().into()))
}

/// Build the code and store it inside a file supposed to be injected in a dsk
/// XXX probably crash if filename is not coherent
/// //
pub fn assemble_to_amsdos_file(
    code: &str,
    amsdos_filename: &str,
    options: EnvOptions
) -> Result<AmsdosFile, AssemblerError> {
    let amsdos_filename = AmsdosFileName::try_from(amsdos_filename)?;

    let tokens = parser::parse_z80_str(code)?;

    let (_, env) = assembler::visit_tokens_all_passes_with_options(&tokens, options)
        .map_err(|(_, _, e)| AssemblerError::AlreadyRenderedError(e.to_string()))?;

    Ok(AmsdosFile::binary_file_from_buffer(
        &amsdos_filename,
        env.loading_address().unwrap(),
        env.execution_address().unwrap(),
        &env.produced_bytes()
    )?)
}

#[cfg(test)]
mod test_super {

    use super::*;

    #[test]
    fn simple_test_assemble() {
        let code = "
		org 0
		db 1, 2
		db 3, 4
		";

        let bytes = assemble(code).unwrap_or_else(|e| panic!("Unable to assemble {}: {}", code, e));
        assert_eq!(bytes.len(), 4);
        assert_eq!(bytes, vec![1, 2, 3, 4]);
    }

    #[test]
    fn located_test_assemble() {
        let code = "
		org 0x100
		db 1, 2
		db 3, 4
		";

        let bytes = assemble(code).unwrap_or_else(|e| panic!("Unable to assemble {}: {}", code, e));
        assert_eq!(bytes, vec![1, 2, 3, 4]);
    }

    #[test]
    fn case_verification() {
        let code = "
		ld hl, TruC
Truc
		";

        let options = AssemblingOptions::new_case_sensitive();
        let options = EnvOptions::from(options);
        println!("{:?}", assemble_with_options(code, options.clone()));
        assert!(assemble_with_options(code, options).is_err());

        let options = AssemblingOptions::new_case_insensitive();
        let options = EnvOptions::from(options);
        println!("{:?}", assemble_with_options(code, options.clone()));
        assert!(assemble_with_options(code, options).is_ok());
    }

    #[test]
    fn test_size() {
        let mut env = Default::default();
        dbg!(
            assemble_call_jr_or_jp(
                Mnemonic::Jp,
                None,
                &DataAccess::Expression(Expr::Value(0)),
                &mut env
            )
            .unwrap()
        );
        assert_eq!(
            Token::OpCode(
                Mnemonic::Jp,
                None,
                Some(DataAccess::Expression(Expr::Value(0))),
                None
            )
            .number_of_bytes(),
            Ok(3)
        );

        assert_eq!(
            Token::OpCode(
                Mnemonic::Jr,
                None,
                Some(DataAccess::Expression(Expr::Value(0))),
                None
            )
            .number_of_bytes(),
            Ok(2)
        );

        assert_eq!(
            Token::OpCode(
                Mnemonic::Jr,
                Some(DataAccess::FlagTest(FlagTest::NC)),
                Some(DataAccess::Expression(Expr::Value(0))),
                None
            )
            .number_of_bytes(),
            Ok(2)
        );

        assert_eq!(
            Token::OpCode(
                Mnemonic::Push,
                Some(DataAccess::Register16(Register16::De)),
                None,
                None
            )
            .number_of_bytes(),
            Ok(1)
        );

        assert_eq!(
            Token::OpCode(
                Mnemonic::Dec,
                Some(DataAccess::Register8(Register8::A)),
                None,
                None
            )
            .number_of_bytes(),
            Ok(1)
        );
    }

    #[test]
    fn test_listing() {
        let mut listing = Listing::from_str("   nop").expect("unable to assemble");
        assert_eq!(listing.estimated_duration().unwrap(), 1);
        listing.set_duration(100);
        assert_eq!(listing.estimated_duration().unwrap(), 100);
    }

    fn code_test(code: &'static str) {
        let asm_options = AssemblingOptions::new_case_insensitive();
        let env_options = EnvOptions::new(ParserOptions::default(), asm_options, Arc::new(()));
        let res = assemble_with_options(code, env_options);
        res.map_err(|e| eprintln!("{e}")).unwrap();
    }

    /// Test stolen to rasm
    #[test]
    fn rasm_pagetag1() {
        let code = "  
        bankset 0
        org #5000
label1
        bankset 1
        org #9000
label2
        bankset 2
        assert {page}label1==0xC0
        assert {page}label2==0xC6 
        assert {pageset}label1==#C0
        assert {pageset}label2==#C2
        assert $ == 0x0000
        assert $$ == 0x0000
        nop";
        code_test(code);
    }
    // /// This test currently does not pass
    // #[test]
    // fn rasm_pagetag2() {
    // let code = "
    // bankset 0
    // call maroutine
    //
    // bank 4
    // org #C000
    // autreroutine
    // nop
    // ret
    //
    // bank 5
    // org #8000
    // maroutine
    // ldir
    // ret
    //
    // bankset 2
    // org #9000
    // troize
    // nop
    // assert {page}maroutine==#7FC5
    // assert {pageset}maroutine==#7FC2
    // assert {page}autreroutine==#7FC4
    // assert {pageset}autreroutine==#7FC2
    // assert {page}troize==#7FCE
    // assert {pageset}troize==#7FCA";
    // rasm_test(code);
    //
    // }
    // #define AUTOTEST_PAGETAG3	"buildsna:bank 2:assert {bank}$==2:assert {page}$==0x7FC0:assert {pageset}$==#7FC0:" \
    // "bankset 1:org #4000:assert {bank}$==5:assert {page}$==0x7FC5:assert {pageset}$==#7FC2"

    #[test]
    fn test_duration() {
        let listing = Listing::from_str(
            "
            pop de      ; 3
        "
        )
        .expect("Unable to assemble this code");
        println!("{}", listing.to_string());
        assert_eq!(listing.estimated_duration().unwrap(), 3);

        let listing = Listing::from_str(
            "
            inc l       ; 1
        "
        )
        .expect("Unable to assemble this code");
        println!("{}", listing.to_string());
        assert_eq!(listing.estimated_duration().unwrap(), 1);

        let listing = Listing::from_str(
            "
            ld (hl), e  ; 2
        "
        )
        .expect("Unable to assemble this code");
        println!("{}", listing.to_string());
        assert_eq!(listing.estimated_duration().unwrap(), 2);

        let listing = Listing::from_str(
            "
            ld (hl), d  ; 2
        "
        )
        .expect("Unable to assemble this code");
        println!("{}", listing.to_string());
        assert_eq!(listing.estimated_duration().unwrap(), 2);

        let listing = Listing::from_str(
            "
            pop de      ; 3
            inc l       ; 1
            ld (hl), e  ; 2
            inc l       ; 1
            ld (hl), d  ; 2
        "
        )
        .expect("Unable to assemble this code");
        println!("{}", listing.to_string());
        assert_eq!(listing.estimated_duration().unwrap(), (3 + 1 + 2 + 1 + 2));
    }

    #[test]
    fn test_real1() {
        let code = "RUN 0x50, 0xc0";
        code_test(code);

        let code = r"    if {bank}$ == 0
            RUN 0x50, 0xc0
        endif
        ";
        code_test(code);
    }
}
