#[cfg(test)]
mod tests {
    use cpclib_asm::assembler::processed_token::build_processed_token;
    use cpclib_asm::preamble::*;

    use std::sync::{Arc, RwLock};

    fn visit_token(token: &Token, env: Arc<RwLock<&mut Env>>) -> Result<(), AssemblerError> {
        let processed = build_processed_token(token, env.clone());
        processed?.visited(&mut *env.write().unwrap())
    }

    fn visit_tokens(tokens: &[Token]) -> Result<Env, AssemblerError> {
        let mut env = Env::default();
        let env_arc = Arc::new(RwLock::new(&mut env));
        for t in tokens {
            visit_token(t, env_arc.clone())?;
        }
        Ok(env)
    }

    #[test]
    pub fn test_visit() {
        let mut env = Env::default();
        let env_arc = Arc::new(RwLock::new(&mut env));

        visit_token(
            &Token::Org {
                val1: Expr::Value(10),
                val2: None
            },
            env_arc.clone()
        )
        .unwrap();
        visit_token(
            &Token::Defb(vec![Expr::Value(10), Expr::Value(5)]),
            env_arc.clone()
        )
        .unwrap();
        visit_token(
            &Token::OpCode(
                Mnemonic::Ld,
                Some(DataAccess::Register8(Register8::A)),
                Some(DataAccess::Register8(Register8::L)),
                None
            ),
            env_arc.clone()
        )
        .unwrap();
    }

    #[test]
    pub fn test_ld() {
        let mut env = Env::default();
        let env_arc = Arc::new(RwLock::new(&mut env));
        assert_ne!(env_arc.read().unwrap().peek(&0x0000.into()), 0x7F);

        visit_token(
            &Token::OpCode(
                Mnemonic::Ld,
                Some(DataAccess::Register8(Register8::A)),
                Some(DataAccess::Register8(Register8::A)),
                None
            ),
            env_arc.clone()
        )
        .unwrap();
        assert_eq!(env_arc.read().unwrap().peek(&0x0000.into()), 0x7F);

        visit_token(
            &Token::OpCode(
                Mnemonic::Ld,
                Some(DataAccess::Register8(Register8::A)),
                Some(DataAccess::Register8(Register8::L)),
                None
            ),
            env_arc.clone()
        )
        .unwrap();
        assert_eq!(env_arc.read().unwrap().peek(&0x0001.into()), 0x7D);

        visit_token(
            &Token::OpCode(
                Mnemonic::Ld,
                Some(DataAccess::Register8(Register8::C)),
                Some(DataAccess::Register8(Register8::L)),
                None
            ),
            env_arc.clone()
        )
        .unwrap();
        assert_eq!(env_arc.read().unwrap().peek(&0x0002.into()), 0x49);
    }

    #[test]
    pub fn test_assemble() {
        let tokens = vec![
            Token::Org {
                val1: Expr::Value(10),
                val2: None
            },
            Token::OpCode(
                Mnemonic::Ld,
                Some(DataAccess::Register8(Register8::A)),
                Some(DataAccess::Register8(Register8::L)),
                None
            ),
        ];

        let _count = visit_tokens(&tokens).unwrap().size();
        //       assert_eq!(count, 2);
    }

    #[test]
    pub fn test_listing_size() {
        let listing = parse_z80_str(
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
    jr nz, .other_lines ; 3"
        )
        .expect("Unable to assemble");

        let size = listing.number_of_bytes();
        eprintln!("{:?}", size);
        assert!(size.is_ok());
        assert_eq!(size.ok().unwrap(), 1 + 1 + 1 + 3 + 60 + 1 + 2);
    }
}
