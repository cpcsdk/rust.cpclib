#[cfg(test)]
mod test {
    use cpclib_asm::preamble::*;

    #[test]
    pub fn test_push() {
        let de_res = push_de();
        let de_expected = Token::OpCode(
            Mnemonic::Push,
            Some(DataAccess::Register16(Register16::De)),
            None,
        );

        assert_eq!(de_res, de_expected);
    }
}