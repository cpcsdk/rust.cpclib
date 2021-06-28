#[macro_use]
extern crate matches;

#[cfg(test)]
mod tests {
    use cpclib_asm::preamble::*;

    #[test]
    pub fn test_visit() {
        let mut env = Env::default();

        visit_token(&Token::Org(Expr::Value(10), None), &mut env);
        visit_token(
            &Token::Defb(vec![Expr::Value(10), Expr::Value(5)]),
            &mut env,
        );
        visit_token(
            &Token::OpCode(
                Mnemonic::Ld,
                Some(DataAccess::Register8(Register8::A)),
                Some(DataAccess::Register8(Register8::L)),None
            ),
            &mut env,
        );
    }

    #[test]
    pub fn test_ld() {
        let mut env = Env::default();
        assert_eq!(env.byte(0x0000), 0x00);

        visit_token(
            &Token::OpCode(
                Mnemonic::Ld,
                Some(DataAccess::Register8(Register8::A)),
                Some(DataAccess::Register8(Register8::A)),None
            ),
            &mut env,
        );
        assert_eq!(env.byte(0x0000), 0x7f);

        visit_token(
            &Token::OpCode(
                Mnemonic::Ld,
                Some(DataAccess::Register8(Register8::A)),
                Some(DataAccess::Register8(Register8::L)),None
            ),
            &mut env,
        );
        assert_eq!(env.byte(0x0001), 0x7d);

        visit_token(
            &Token::OpCode(
                Mnemonic::Ld,
                Some(DataAccess::Register8(Register8::C)),
                Some(DataAccess::Register8(Register8::C)), None
            ),
            &mut env,
        );
        assert_eq!(env.byte(0x0002), 0x49);
    }

    #[test]
    pub fn test_assemble() {
        let tokens = vec![
            Token::Org(Expr::Value(10), None),
            Token::OpCode(
                Mnemonic::Ld,
                Some(DataAccess::Register8(Register8::A)),
                Some(DataAccess::Register8(Register8::L)),None
            ),
        ];

        let _count = visit_tokens(&tokens).unwrap().size();
        //       assert_eq!(count, 2);
    }

    #[test]
    pub fn test_listing_size() {
        let listing = Listing::from_str(
            "
.first_line
                    ; end code : 9 nops
    pop de              ; 4
    ld a, e             ; 1
    dec sp              ; 2
    jp  .other_lines + (9+4+1+2)                ; 4


.other_lines
    defs 64 - 4
    dec a               ; 1
    jr nz, .other_lines ; 3",
        )
        .expect("Unable to assemble");

        let size = listing.number_of_bytes();
        eprintln!("{:?}", size);
        assert!(size.is_ok());
        assert_eq!(size.ok().unwrap(), 1 + 1 + 1 + 3 + 60 + 1 + 2);
    }
}
