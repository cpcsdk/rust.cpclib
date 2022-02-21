#![feature(assert_matches)]
#![feature(in_band_lifetimes)]
#![feature(specialization)]
#![feature(exact_size_is_empty)]
#![feature(exclusive_range_pattern)]
#![feature(let_chains)]
#![feature(box_syntax)]
#![feature(box_patterns)]
#![feature(string_extend_from_within)]
#![recursion_limit = "256"]

/// Implementation of various behvior for the tokens of cpclib_tokens
pub mod implementation;

/// All the stuff to parse z80 code.
pub mod parser;

/// Production of the bytecodes from the tokens.
pub mod assembler;

pub mod disass;

pub mod preamble;

pub mod error;

mod crunchers;

#[cfg(feature = "basm")]
pub mod basm_utils;

use std::fmt::Debug;
use std::io::Write;
use std::sync::{Arc, RwLock};

use cpclib_disc::amsdos::*;
use preamble::*;

use self::listing_output::ListingOutput;
use crate::processed_token::AsSimpleToken;

/// Configuration of the assembler. By default the assembler is case sensitive and has no symbol
#[derive(Debug)]
pub struct AssemblingOptions {
    /// Set to true to consider that the assembler pay attention to the case of the labels
    case_sensitive: bool,
    /// Contains some symbols that could be used during assembling
    symbols: cpclib_tokens::symbols::SymbolsTable,
    output_builder: Option<Arc<RwLock<ListingOutput>>>
}

impl Default for AssemblingOptions {
    fn default() -> Self {
        Self {
            case_sensitive: true,
            symbols: cpclib_tokens::symbols::SymbolsTable::default(),
            output_builder: None
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
        options.case_sensitive = false;
        options
    }

    /// Creation an option object with the given symbol table
    pub fn new_with_table(symbols: &cpclib_tokens::symbols::SymbolsTable) -> Self {
        let mut options = Self::default();
        options.set_symbols(symbols);
        options
    }

    /// Specify if the assembler must be case sensitive or not
    pub fn set_case_sensitive(&mut self, val: bool) -> &mut Self {
        self.case_sensitive = val;
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
        self.case_sensitive
    }

    pub fn write_listing_output<W: 'static + Write + Send + Sync>(
        &mut self,
        writer: W
    ) -> &mut Self {
        self.output_builder = Some(Arc::new(RwLock::new(ListingOutput::new(writer))));
        self.output_builder
            .as_mut()
            .map(|b| b.write().unwrap().on());
        self
    }
}

/// Assemble a piece of code and returns the associated list of bytes.
pub fn assemble(code: &str, ctx: &ParserContext) -> Result<Vec<u8>, AssemblerError> {
    let options = AssemblingOptions::default();
    // let options = AssemblingOptions::new_with_table(table);
    assemble_with_options(code, &options, ctx).map(|(bytes, _symbols)| bytes)
}

/// Assemble a piece of code and returns the associates liste of bytes as well as the generated reference table.
pub fn assemble_with_options(
    code: &str,
    options: &AssemblingOptions,
    ctx: &ParserContext
) -> Result<(Vec<u8>, cpclib_tokens::symbols::SymbolsTable), AssemblerError> {
    let tokens = parser::parse_z80_str(code)?;
    assemble_tokens_with_options(&tokens, &options, ctx)
}

/// Assemble the predifined list of tokens
pub fn assemble_tokens_with_options<'tokens, T: 'static + Visited + AsSimpleToken + Clone + ListingElement + Sync>(
    tokens: &'tokens [T],
    options: &AssemblingOptions,
    ctx: &ParserContext
) -> Result<(Vec<u8>, cpclib_tokens::symbols::SymbolsTable), AssemblerError> {
    let env = assembler::visit_tokens_all_passes_with_options(tokens, &options, ctx)?;
    Ok((env.produced_bytes(), env.symbols().as_ref().clone()))
}

/// Build the code and store it inside a file supposed to be injected in a dsk
/// XXX probably crash if filename is not coherent
/// //
pub fn assemble_to_amsdos_file(
    code: &str,
    amsdos_filename: &str,
    ctx: &ParserContext
) -> Result<AmsdosFile, AssemblerError> {

    let amsdos_filename = AmsdosFileName::try_from(amsdos_filename)?;

    let tokens = parser::parse_z80_str(code)?;
    let options = AssemblingOptions::default();

    let env = assembler::visit_tokens_all_passes_with_options(&tokens, &options, ctx)?;

    Ok(AmsdosFile::binary_file_from_buffer(
        &amsdos_filename,
        env.loading_address().unwrap() as u16,
        env.execution_address().unwrap() as u16,
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

        let ctx = ParserContext{
            ..Default::default()
        };

        let options = AssemblingOptions::new_case_sensitive();
        println!("{:?}", assemble_with_options(code, &options, &ctx));
        assert!(assemble_with_options(code, &options, &ctx).is_err());

        let options = AssemblingOptions::new_case_insensitive();
        println!("{:?}", assemble_with_options(code, &options, &ctx));
        assert!(assemble_with_options(code, &options, &ctx).is_ok());
    }

    #[test]
    fn test_size() {
        let mut env = Default::default();
        dbg!(assemble_call_jr_or_jp(
            Mnemonic::Jp,
            None,
            &DataAccess::Expression(Expr::Value(0)),
            &mut env
        )
        .unwrap());
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
        let options = AssemblingOptions::new_case_insensitive();
        let res = assemble_with_options(code, &options);
        res.unwrap();
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
        let code = "    RUN 0x50, 0xc0";
        code_test(code);

        let code = "    if {bank}$ == 0
            RUN 0x50, 0xc0
        endif
        ";
        code_test(code);
    }
}
