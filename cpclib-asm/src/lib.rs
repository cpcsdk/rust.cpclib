

/// Implementation of various behvior for the tokens of cpclib_tokens
pub mod implementation;

/// All the stuff to parse z80 code.
pub mod parser;


/// Production of the bytecodes from the tokens.
pub mod assembler;

pub mod disass;

pub mod preamble;

pub mod error;

use crate::parser::ParserContext;


use error::*;
use preamble::*;

/// Configuration of the assembler. By default the assembler is case sensitive and has no symbol
#[derive(Clone, Debug)]
pub struct AssemblingOptions {
    /// Set to true to consider that the assembler pay attention to the case of the labels
    case_sensitive: bool,
    /// Contains some symbols that could be used during assembling
    symbols: cpclib_tokens::symbols::SymbolsTable,
}

impl Default for AssemblingOptions {
    fn default() -> Self {
        Self {
            case_sensitive: true,
            symbols: cpclib_tokens::symbols::SymbolsTable::default(),
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

    pub fn case_sensitive(&self) -> bool {
        self.case_sensitive
    }
}

/// Assemble a piece of code and returns the associated list of bytes.
pub fn assemble(code: &str) -> Result<Vec<u8>, AssemblerError> {
    let options = AssemblingOptions::default();
    //let options = AssemblingOptions::new_with_table(table);
    assemble_with_options(code, &options).map(|(bytes, _symbols)| bytes)
}

/// Assemble a piece of code and returns the associates liste of bytes as well as the generated reference table.
pub fn assemble_with_options(
    code: &str,
    options: &AssemblingOptions,
) -> Result<(Vec<u8>, cpclib_tokens::symbols::SymbolsTable), AssemblerError> {
    let tokens = parser::parse_str(code)?;
    assemble_tokens_with_options(&tokens, options)
}

/// Assemble the predifined list of tokens
pub fn assemble_tokens_with_options(
    tokens: &[Token],
    options: &AssemblingOptions
) -> Result<(Vec<u8>, cpclib_tokens::symbols::SymbolsTable), AssemblerError> {
    let env = assembler::visit_tokens_all_passes_with_options(&tokens, &options)?;
    Ok((env.produced_bytes(), env.symbols().as_ref().clone()))
}

#[cfg(test)]
mod test_super {
    use super::*;
    use crate::preamble::*;

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
        println!("{:?}", assemble_with_options(code, &options));
        assert!(assemble_with_options(code, &options).is_err());

        let options = AssemblingOptions::new_case_insensitive();
        println!("{:?}", assemble_with_options(code, &options));
        assert!(assemble_with_options(code, &options).is_ok());
    }



    #[test]
    fn test_size() {
        assert_eq!(
            Token::OpCode(
                Mnemonic::Jp,
                None,
                Some(DataAccess::Expression(Expr::Value(0)))
            )
            .number_of_bytes(),
            Ok(3)
        );

        assert_eq!(
            Token::OpCode(
                Mnemonic::Jr,
                None,
                Some(DataAccess::Expression(Expr::Value(0)))
            )
            .number_of_bytes(),
            Ok(2)
        );

        assert_eq!(
            Token::OpCode(
                Mnemonic::Jr,
                Some(DataAccess::FlagTest(FlagTest::NC)),
                Some(DataAccess::Expression(Expr::Value(0)))
            )
            .number_of_bytes(),
            Ok(2)
        );

        assert_eq!(
            Token::OpCode(
                Mnemonic::Push,
                Some(DataAccess::Register16(Register16::De)),
                None
            )
            .number_of_bytes(),
            Ok(1)
        );

        assert_eq!(
            Token::OpCode(
                Mnemonic::Dec,
                Some(DataAccess::Register8(Register8::A)),
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


    

    /// Test stolen to rasm
    #[test]
    fn test_pagetag1() {
        Listing::from_str(" bankset 0").expect("unable to assemble");
        Listing::from_str(" org #5000").expect("unable to assemble");
        Listing::from_str(" assert label1==0x7FC0").expect("unable to assemble");
        Listing::from_str(" ld a, label").expect("unable to assemble");
        Listing::from_str(" ld a, {page}label").expect("unable to assemble");

        Listing::from_str(" assert {page}label1==0x7FC0").expect("unable to assemble");

        let code = "  bankset 0
                      org #5000
label1
                    bankset 1
                    org #9000
label2
                    bankset 2
                    assert {page}label1==0x7FC0
                    assert {page}label2==0x7FC6
                    assert {pageset}label1==#7FC0
                    assert {pageset}label2==#7FC2
                    nop";
        
        let mut listing = Listing::from_str(code).expect("unable to assemble");
    }
    
    /*
#define AUTOTEST_PAGETAG2	"bankset 0:call maroutine:bank 4:org #C000:autreroutine:nop:" \
							"ret:bank 5:org #8000:maroutine:ldir:ret:bankset 2:org #9000:troize:nop:" \
							"assert {page}maroutine==#7FC5:assert {pageset}maroutine==#7FC2:assert {page}autreroutine==#7FC4:" \
							"assert {pageset}autreroutine==#7FC2:assert {page}troize==#7FCE:assert {pageset}troize==#7FCA"							
							
#define AUTOTEST_PAGETAG3	"buildsna:bank 2:assert {bank}$==2:assert {page}$==0x7FC0:assert {pageset}$==#7FC0:" \
							"bankset 1:org #4000:assert {bank}$==5:assert {page}$==0x7FC5:assert {pageset}$==#7FC2"
    */	
    


    #[test]
    fn test_duration() {
        let listing = Listing::from_str(
            "
            pop de      ; 3
        ",
        )
        .expect("Unable to assemble this code");
        println!("{}", listing.to_string());
        assert_eq!(listing.estimated_duration().unwrap(), 3);

        let listing = Listing::from_str(
            "
            inc l       ; 1
        ",
        )
        .expect("Unable to assemble this code");
        println!("{}", listing.to_string());
        assert_eq!(listing.estimated_duration().unwrap(), 1);

        let listing = Listing::from_str(
            "
            ld (hl), e  ; 2
        ",
        )
        .expect("Unable to assemble this code");
        println!("{}", listing.to_string());
        assert_eq!(listing.estimated_duration().unwrap(), 2);

        let listing = Listing::from_str(
            "
            ld (hl), d  ; 2
        ",
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
        ",
        )
        .expect("Unable to assemble this code");
        println!("{}", listing.to_string());
        assert_eq!(listing.estimated_duration().unwrap(), (3 + 1 + 2 + 1 + 2));
    }
}
