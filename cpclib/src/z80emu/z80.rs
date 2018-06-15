///! Manage z80 CPU
///! Could be used to simulate or generate code

use std::mem::swap;
use std::fmt::Debug;
use std::fmt;
use num::integer::Integer;
use assembler::tokens::*;

/// Common trait for Register 8 and 6 bits
pub trait HasValue {
    type ValueType: Integer;

    /// Retreive the stored value
    fn value(&self) -> Self::ValueType;

    #[inline]
    fn get(&self) -> Self::ValueType {
        self.value()
    }

    /// Change the stored value
    fn set(&mut self, value:Self::ValueType);

    /// Add value to register
    // TODO return flags
    fn add(&mut self, value:Self::ValueType);

    // TODO find a way to implement it here
    // TODO return flags
    fn inc(&mut self);
}

/// Represents an 8 bit register
#[derive(Copy,Clone,Debug)]
pub struct Register8{
    val: u8
}


/// By default a Register8 is set to 0
/// TODO use an Unknown value ?
impl Default for Register8 {
    fn default() -> Self {
        Self{val:0}
    }
}


// TODO use macro for that
impl HasValue for Register8 {
    type ValueType=u8;

    fn value(&self) -> Self::ValueType{
        self.val
    }

    fn set(&mut self, value:Self::ValueType) {
        self.val = value;
    }


    fn add(&mut self, value:Self::ValueType) {
        self.val = ((self.val as u16 + value as u16) & 256) as u8;
    }

    fn inc(&mut self) {
        self.add(1);
    }
}



/// Represents a 16 bits register
#[derive(Copy, Clone, Default)]
pub struct Register16 {
    low: Register8,
    high: Register8
}



impl Debug for Register16 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {write!(f, "({:?}, {:?})", &self.high, &self.low)}
    }

}


impl HasValue for Register16 {
    type ValueType=u16;

    fn value(&self) -> Self::ValueType{
        256*self.high().value() as u16 + self.low().value() as u16
    }

    fn set(&mut self, value:Self::ValueType) {
        self.low_mut().set((value%256) as _);
        self.high_mut().set((value/256) as _);
    }

    fn add(&mut self, value:Self::ValueType) {
        let val = ((self.value() as u32 + value as u32) & 0xffff) as u16;
        self.set(val);
    }

    fn inc(&mut self) {
        self.add(1);
    }
}


impl Register16 {

    fn low(&self) -> &Register8 {
        &self.low
    }

    fn high(&self) -> &Register8 {
        &self.high
    }


    fn low_mut(&mut self) -> &mut Register8 {
        &mut self.low
    }

    fn high_mut(&mut self) -> &mut Register8 {
        &mut self.high
    }


}


/// Z80 CPU model
/// TODO Add memory
#[derive(Default, Debug)]
pub struct Z80 {
    reg_pc: Register16,
    reg_sp: Register16,

    reg_af: Register16,

    reg_bc: Register16,
    reg_de: Register16,
    reg_hl: Register16,

    reg_ix: Register16,
    reg_iy: Register16,

    reg_i: Register8,
    reg_r: Register8,

    reg_af_prime: Register16,

    reg_bc_prime: Register16,
    reg_de_prime: Register16,
    reg_hl_prime: Register16,
}


impl Z80 {

    // Immutable accessors
    pub fn pc(& self)-> & Register16 {& self.reg_pc}
    pub fn sp(& self)-> & Register16 {& self.reg_sp}

    pub fn af(& self)-> & Register16 {& self.reg_af}

    pub fn bc(& self)-> & Register16 {& self.reg_bc}
    pub fn de(& self)-> & Register16 {& self.reg_de}
    pub fn hl(& self)-> & Register16 {& self.reg_hl}

    pub fn ix(& self)-> & Register16 {& self.reg_ix}
    pub fn iy(& self)-> & Register16 {& self.reg_iy}

    pub fn a(& self)-> & Register8 {
       let tmp = self.af();
       tmp.high()
    }
    pub fn f(& self)-> & Register8 {
       let tmp = self.af();
       tmp.low()
    }


    pub fn b(& self)-> & Register8 {
       let tmp = self.bc();
       tmp.high()
    }
    pub fn c(& self)-> & Register8 {
       let tmp = self.bc();
       tmp.low()
    }


    pub fn d(& self)-> & Register8 {
       let tmp = self.de();
       tmp.high()
    }
    pub fn e(& self)-> & Register8 {
       let tmp = self.de();
       tmp.low()
    }


    pub fn h(& self)-> & Register8 {
       let tmp = self.hl();
       tmp.high()
    }
    pub fn l(& self)-> & Register8 {
       let tmp = self.hl();
       tmp.low()
    }

    pub fn ixh(& self)-> & Register8 {
       let tmp = self.ix();
       tmp.high()
    }
    pub fn ixl(& self)-> & Register8 {
       let tmp = self.ix();
       tmp.low()
    }

    pub fn iyh(& self)-> & Register8 {
       let tmp = self.iy();
       tmp.high()
    }
    pub fn iyl(& self)-> & Register8 {
       let tmp = self.iy();
       tmp.low()
    }


    // Mutable accessors
    pub fn pc_mut(&mut self)-> &mut Register16 {&mut self.reg_pc}
    pub fn sp_mut(&mut self)-> &mut Register16 {&mut self.reg_sp}

    pub fn af_mut(&mut self)-> &mut Register16 {&mut self.reg_af}

    pub fn bc_mut(&mut self)-> &mut Register16 {&mut self.reg_bc}
    pub fn de_mut(&mut self)-> &mut Register16 {&mut self.reg_de}
    pub fn hl_mut(&mut self)-> &mut Register16 {&mut self.reg_hl}

    pub fn ix_mut(&mut self)-> &mut Register16 {&mut self.reg_ix}
    pub fn iy_mut(&mut self)-> &mut Register16 {&mut self.reg_iy}

    pub fn a_mut(&mut self)-> &mut Register8 {
       let tmp = self.af_mut();
       tmp.high_mut()
    }
    pub fn f_mut(&mut self)-> &mut Register8 {
       let tmp = self.af_mut();
       tmp.low_mut()
    }


    pub fn b_mut(&mut self)-> &mut Register8 {
       let tmp = self.bc_mut();
       tmp.high_mut()
    }
    pub fn c_mut(&mut self)-> &mut Register8 {
       let tmp = self.bc_mut();
       tmp.low_mut()
    }


    pub fn d_mut(&mut self)-> &mut Register8 {
       let tmp = self.de_mut();
       tmp.high_mut()
    }
    pub fn e_mut(&mut self)-> &mut Register8 {
       let tmp = self.de_mut();
       tmp.low_mut()
    }


    pub fn h_mut(&mut self)-> &mut Register8 {
       let tmp = self.hl_mut();
       tmp.high_mut()
    }
    pub fn l_mut(&mut self)-> &mut Register8 {
       let tmp = self.hl_mut();
       tmp.low_mut()
    }

    pub fn ixh_mut(&mut self)-> &mut Register8 {
       let tmp = self.ix_mut();
       tmp.high_mut()
    }
    pub fn ixl_mut(&mut self)-> &mut Register8 {
       let tmp = self.ix_mut();
       tmp.low_mut()
    }

    pub fn iyh_mut(&mut self)-> &mut Register8 {
       let tmp = self.iy_mut();
       tmp.high_mut()
    }
    pub fn iyl_mut(&mut self)-> &mut Register8 {
       let tmp = self.iy_mut();
       tmp.low_mut()
    }




    pub fn ex_af_af_prime(&mut self) {
        swap(& mut self.reg_af_prime, & mut self.reg_af);
    }

    pub fn exx(&mut self) {
        swap(& mut self.reg_hl_prime, & mut self.reg_hl);
        swap(& mut self.reg_de_prime, & mut self.reg_de);
        swap(& mut self.reg_bc_prime, & mut self.reg_bc);
    }



    // To reduce copy paste/implementation errors, all manipulation are translated as token usage
    pub fn copy_to_from(&mut self, to: ::assembler::tokens::Register8, from: ::assembler::tokens::Register8) {
        self.execute(&Token::OpCode(
                        Mnemonic::Ld,
                        Some(DataAccess::Register8(to)),
                        Some(DataAccess::Register8(from)),
                )
        );
    }
}


#[cfg(test)]
mod tests{
    use z80emu::z80::*;

    #[test]
    fn build_z80() {
        let z80 = Z80::default();
    }

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
        assert_eq!(BC.value(), 22);

        BC.set(50*256);
        assert_eq!(BC.low().value(), 0);
        assert_eq!(BC.high().value(), 50);
        assert_eq!(BC.value(), 50*256);


        BC.set(0xffff);
        BC.add(1);
        assert_eq!(BC.value(), 0);


        BC.set(0x4000);
        BC.add(1);
        assert_eq!(BC.value(), 0x4001);
    }


    #[test]
    fn z80_registers() {
        let mut z80 = Z80::default();

        z80.bc_mut().set(0x1234);
        z80.af_mut().set(0x4567);

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


    #[test]
    fn eval() {
        use assembler::tokens::*;

        let mut z80 = Z80::default();
        z80.pc_mut().set(0x4000);
        z80.hl_mut().set(0x8000);
        z80.de_mut().set(0xc000);
        z80.a_mut().set(0);


        let pop_bc = Token::OpCode(Mnemonic::Pop, Some(DataAccess::Register16(Register16::Bc)), None);
        let ld_l_a = Token::OpCode(Mnemonic::Ld, Some(DataAccess::Register8(Register8::L)), Some(DataAccess::Register8(Register8::A)));
        let add_a_b = Token::OpCode(Mnemonic::Add,Some(DataAccess::Register8(Register8::A)), Some(DataAccess::Register8(Register8::B)));
        let ldi = Token::OpCode(Mnemonic::Ldi, None, None);

        assert_eq!(z80.pc().value(), 0x4000);

        z80.execute(&pop_bc);
        assert_eq!(z80.pc().value(), 0x4001);

        z80.execute(&add_a_b);
        assert_eq!(z80.pc().value(), 0x4002);

        z80.execute(&ld_l_a);
        assert_eq!(z80.pc().value(), 0x4003);
        assert_eq!(z80.a().value(), z80.l().value());

        z80.execute(&ldi);
        assert_eq!(z80.pc().value(), 0x4005);
        assert_eq!(z80.de().value(), 0xc001);
    }
}
