pub(crate) mod data_access;
pub(crate) mod expression;
pub(crate) mod instructions;
pub(crate) mod listing;
pub(crate) mod registers;
pub(crate) mod tokens;

pub use self::data_access::*;
pub use self::expression::*;
pub use self::instructions::*;
pub use self::listing::*;
pub use self::registers::*;
pub use self::tokens::*;

#[cfg(test)]
mod test {
    use crate::assembler::tokens::{
        DataAccess, Expr, FlagTest, Listing, ListingElement, Mnemonic, Register16, Register8, Token,
    };
    use std::str::FromStr;
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

    #[test]
    fn test_duration() {
        let listing = Listing::from_str(
            "
            pop de      ; 3
        ",
        )
        .expect("Unable to assemble this code");
        println!("{}", listing);
        assert_eq!(listing.estimated_duration().unwrap(), 3);

        let listing = Listing::from_str(
            "
            inc l       ; 1
        ",
        )
        .expect("Unable to assemble this code");
        println!("{}", listing);
        assert_eq!(listing.estimated_duration().unwrap(), 1);

        let listing = Listing::from_str(
            "
            ld (hl), e  ; 2
        ",
        )
        .expect("Unable to assemble this code");
        println!("{}", listing);
        assert_eq!(listing.estimated_duration().unwrap(), 2);

        let listing = Listing::from_str(
            "
            ld (hl), d  ; 2
        ",
        )
        .expect("Unable to assemble this code");
        println!("{}", listing);
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
        println!("{}", listing);
        assert_eq!(listing.estimated_duration().unwrap(), (3 + 1 + 2 + 1 + 2));
    }
}
