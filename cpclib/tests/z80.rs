extern crate cpc;

#[cfg(test)]
mod tests {
    use cpc::z80::*;
    use cpc::z80::HasValue;

    #[test]
    fn test_register8() {
        let mut B = Register8::default();

        assert_eq!(B.value(), 0);

        B.set(22);
        assert_eq!(B.value(), 22);
    }



    #[test]
    fn test_register16() {
        let mut BC = Register16::default();

        assert_eq!(BC.value(), 0);

        BC.set(22);
        assert_eq!(BC.low().value(), 22);
        assert_eq!(BC.high().value(), 0);

        BC.set(50*256);
        assert_eq!(BC.low().value(), 0);
        assert_eq!(BC.high().value(), 50);
    }


    #[test]
    fn z80_registers() {
        let mut z80 = Z80::default();

        z80.bc().set(0x1234);
        z80.af().set(0x4567);

        assert_eq!(z80.b().value(), 0x12);
        assert_eq!(z80.c().value(), 0x34);

        assert_eq!(z80.a().value(), 0x45);
        assert_eq!(z80.f().value(), 0x67);

        z80.ex_af_af_prime();
        assert_eq!(z80.a().value(), 0x00);
        assert_eq!(z80.b().value(), 0x12);
        assert_eq!(z80.c().value(), 0x34);

        z80.ex_af_af_prime();
        assert_eq!(z80.a().value(), 0x45);

    }
}
