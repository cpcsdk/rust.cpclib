#![allow(dead_code)]


// trick to not be distirb by Register8/16
use std::fmt;
use std::fmt::Debug;
/// ! Manage z80 CPU
/// ! Could be used to simulate or generate code
use std::mem::swap;

use cpclib_asm::assembler::Env;
use cpclib_common::num::integer::Integer;
use cpclib_common::num::traits::{WrappingAdd, WrappingSub};
use cpclib_common::num::One;

use crate::preamble::*;

/// Common trait for Register 8 and 6 bits
#[allow(missing_docs)]
pub trait HasValue {
    /// The type that encodes the value of interest
    type ValueType: Integer + One + WrappingAdd + WrappingSub;

    /// Retreive the stored value
    fn value(&self) -> Self::ValueType;

    #[inline]
    fn get(&self) -> Self::ValueType {
        self.value()
    }
    /// Change the stored value
    fn set(&mut self, value: Self::ValueType);

    fn add(&mut self, value: Self::ValueType) {
        self.set(self.value().wrapping_add(&value));
    }

    fn sub(&mut self, value: Self::ValueType) {
        self.set(self.value().wrapping_sub(&value));
    }

    fn inc(&mut self) {
        self.add(Self::ValueType::one());
    }

    fn dec(&mut self) {
        self.sub(Self::ValueType::one());
    }
}

/// Represents an 8 bit register
#[derive(Copy, Clone, Debug)]
pub struct Register8 {
    val: u8
}

/// By default a Register8 is set to 0
/// TODO use an Unknown value ?
impl Default for Register8 {
    fn default() -> Self {
        Self { val: 0 }
    }
}

// TODO use macro for that
impl HasValue for Register8 {
    type ValueType = u8;

    fn value(&self) -> Self::ValueType {
        self.val
    }

    fn set(&mut self, value: Self::ValueType) {
        self.val = value;
    }
}

impl Register8 {
    pub fn res_bit(&mut self, bit: u8) {
        let new = self.value() & (!(1 << bit));
        self.set(new);
    }

    pub fn set_bit(&mut self, bit: u8) {
        let new = self.value() | (1 << bit);
        self.set(new);
    }

    pub fn and(&mut self, value: u8) {
        let new = self.value() & value;
        self.set(new);
    }

    pub fn or(&mut self, value: u8) {
        let new = self.value() | value;
        self.set(new);
    }

    pub fn xor(&mut self, value: u8) {
        let new = self.value() ^ value;
        self.set(new);
    }
}

/// Represents a 16 bits register by decomposing it in 2 8 bits registers
#[derive(Copy, Clone, Default)]
pub struct Register16 {
    low: Register8,
    high: Register8
}

impl Debug for Register16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?}, {:?})", &self.high, &self.low)
    }
}

impl HasValue for Register16 {
    type ValueType = u16;

    fn value(&self) -> Self::ValueType {
        256 * u16::from(self.high().value()) + u16::from(self.low().value())
    }

    fn set(&mut self, value: Self::ValueType) {
        self.low_mut().set((value % 256) as _);
        self.high_mut().set((value / 256) as _);
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

/// Contains all needed stuff to make the emulation that does not belong to the CPU
/// This enable the execution of tokens that contains symbolic values
#[derive(Default, Debug, Clone)]
pub struct EmulationContext {
    /// The symbol table that can be used when ev
    pub(crate) env: Env
}

impl EmulationContext {
    pub fn symbols(&mut self) -> &SymbolsTableCaseDependent {
        self.env.symbols()
    }

    pub fn symbols_mut(&mut self) -> &mut SymbolsTableCaseDependent {
        self.env.symbols_mut()
    }
}

/// Highly simplify z80 model.
/// TODO Add memory
#[derive(Default, Debug, Clone)]
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

    pub(crate) context: EmulationContext
}

#[allow(missing_docs)]
impl Z80 {
    // Immutable accessors
    pub fn pc(&self) -> &Register16 {
        &self.reg_pc
    }

    pub fn sp(&self) -> &Register16 {
        &self.reg_sp
    }

    pub fn af(&self) -> &Register16 {
        &self.reg_af
    }

    pub fn bc(&self) -> &Register16 {
        &self.reg_bc
    }

    pub fn de(&self) -> &Register16 {
        &self.reg_de
    }

    pub fn hl(&self) -> &Register16 {
        &self.reg_hl
    }

    pub fn ix(&self) -> &Register16 {
        &self.reg_ix
    }

    pub fn iy(&self) -> &Register16 {
        &self.reg_iy
    }

    pub fn a(&self) -> &Register8 {
        let tmp = self.af();
        tmp.high()
    }

    pub fn f(&self) -> &Register8 {
        let tmp = self.af();
        tmp.low()
    }

    pub fn b(&self) -> &Register8 {
        let tmp = self.bc();
        tmp.high()
    }

    pub fn c(&self) -> &Register8 {
        let tmp = self.bc();
        tmp.low()
    }

    pub fn d(&self) -> &Register8 {
        let tmp = self.de();
        tmp.high()
    }

    pub fn e(&self) -> &Register8 {
        let tmp = self.de();
        tmp.low()
    }

    pub fn h(&self) -> &Register8 {
        let tmp = self.hl();
        tmp.high()
    }

    pub fn l(&self) -> &Register8 {
        let tmp = self.hl();
        tmp.low()
    }

    pub fn ixh(&self) -> &Register8 {
        let tmp = self.ix();
        tmp.high()
    }

    pub fn ixl(&self) -> &Register8 {
        let tmp = self.ix();
        tmp.low()
    }

    pub fn iyh(&self) -> &Register8 {
        let tmp = self.iy();
        tmp.high()
    }

    pub fn iyl(&self) -> &Register8 {
        let tmp = self.iy();
        tmp.low()
    }

    // Mutable accessors
    pub fn pc_mut(&mut self) -> &mut Register16 {
        &mut self.reg_pc
    }

    pub fn sp_mut(&mut self) -> &mut Register16 {
        &mut self.reg_sp
    }

    pub fn af_mut(&mut self) -> &mut Register16 {
        &mut self.reg_af
    }

    pub fn bc_mut(&mut self) -> &mut Register16 {
        &mut self.reg_bc
    }

    pub fn de_mut(&mut self) -> &mut Register16 {
        &mut self.reg_de
    }

    pub fn hl_mut(&mut self) -> &mut Register16 {
        &mut self.reg_hl
    }

    pub fn ix_mut(&mut self) -> &mut Register16 {
        &mut self.reg_ix
    }

    pub fn iy_mut(&mut self) -> &mut Register16 {
        &mut self.reg_iy
    }

    pub fn a_mut(&mut self) -> &mut Register8 {
        let tmp = self.af_mut();
        tmp.high_mut()
    }

    pub fn f_mut(&mut self) -> &mut Register8 {
        let tmp = self.af_mut();
        tmp.low_mut()
    }

    pub fn b_mut(&mut self) -> &mut Register8 {
        let tmp = self.bc_mut();
        tmp.high_mut()
    }

    pub fn c_mut(&mut self) -> &mut Register8 {
        let tmp = self.bc_mut();
        tmp.low_mut()
    }

    pub fn d_mut(&mut self) -> &mut Register8 {
        let tmp = self.de_mut();
        tmp.high_mut()
    }

    pub fn e_mut(&mut self) -> &mut Register8 {
        let tmp = self.de_mut();
        tmp.low_mut()
    }

    pub fn h_mut(&mut self) -> &mut Register8 {
        let tmp = self.hl_mut();
        tmp.high_mut()
    }

    pub fn l_mut(&mut self) -> &mut Register8 {
        let tmp = self.hl_mut();
        tmp.low_mut()
    }

    pub fn ixh_mut(&mut self) -> &mut Register8 {
        let tmp = self.ix_mut();
        tmp.high_mut()
    }

    pub fn ixl_mut(&mut self) -> &mut Register8 {
        let tmp = self.ix_mut();
        tmp.low_mut()
    }

    pub fn iyh_mut(&mut self) -> &mut Register8 {
        let tmp = self.iy_mut();
        tmp.high_mut()
    }

    pub fn iyl_mut(&mut self) -> &mut Register8 {
        let tmp = self.iy_mut();
        tmp.low_mut()
    }

    pub fn ex_af_af_prime(&mut self) {
        swap(&mut self.reg_af_prime, &mut self.reg_af);
    }

    pub fn exx(&mut self) {
        swap(&mut self.reg_hl_prime, &mut self.reg_hl);
        swap(&mut self.reg_de_prime, &mut self.reg_de);
        swap(&mut self.reg_bc_prime, &mut self.reg_bc);
    }

    pub fn ex_de_hl(&mut self) {
        swap(&mut self.reg_hl, &mut self.reg_de);
    }

    // To reduce copy paste/implementation errors, all manipulation are translated as token usage
    pub fn copy_to_from(&mut self, to: tokens::Register8, from: tokens::Register8) {
        self.execute(&Token::OpCode(
            Mnemonic::Ld,
            Some(DataAccess::Register8(to)),
            Some(DataAccess::Register8(from)),
            None
        ));
    }
}

// https://www.msx.org/wiki/Assembler_for_Dummies_%28Z80%29#Flags
#[allow(unused)]
pub enum FlagPos {
    // bit 7, SF, Sign flag. This is copy of the results most significant bit. If the bit is set (= 1 = "M") "Minus" the 2-complement value is negative other ways the result is positive (= 0 = "P") "Plus". Note that this flag can be used only with conditional JP-instruction.
    Sign = 7,

    // bit 6, ZF, Zero flag. If the result of mathematical operation is zero the bit is set (= 1 = "Z") other ways the bit is reset (= 0 = "NZ") "Not zero"
    Zero = 6,

    // Bit 5, YF, copy of the results 5th bit.
    Y = 5,

    // Bit 4, HF, "Half-carry" from bit 3 to 4. Z80 uses this internally for BCD correction.
    HalfCarry = 4,

    // Bit 3, XF, copy of the results 3rd bit.
    X = 3,

    // Bit 2, PF/VF, Parity flag. This is copy of the results least significant bit. If the bit is set (= 1 = "PO") the parity is odd otherways the result is even. (= 0 = "PE") On some cases this bit might be used to indicate 2-compliment signed overflow.(VF) Note that this flag can be used only with conditional JP-instruction.
    Parity = 2,

    // Bit 1, NF, this bit is used by Z80 internally to indicate if last operation was addition or subtraction (needed for BCD correction)
    N = 1,

    // Bit 0, CF, Carry flag = Overflow bit. This bit is the most high bit that did not fit to the result. In addition to large calculations and bit transfers it is mostly used in comparisons to see if the subtraction result was smaller or greater than zero. If the carry flag was set (= 1 = "C") the result did overflow. Other ways the flag is reset (= 0 = "NC") "No carry". Please note that 8bit INC/DEC commands do not update this flag.
    Carry = 0
}

#[allow(unused)]
struct ExtraFlags {
    iff1: u8,
    iff2: u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_z80() {
        let _z80 = Z80::default();
    }

    #[test]
    fn test_register8() {
        let mut b = Register8::default();

        assert_eq!(b.value(), 0);

        b.set(22);
        assert_eq!(b.value(), 22);
    }

    #[test]
    fn test_register16() {
        let mut bc = Register16::default();

        assert_eq!(bc.value(), 0);

        bc.set(22);
        assert_eq!(bc.low().value(), 22);
        assert_eq!(bc.high().value(), 0);
        assert_eq!(bc.value(), 22);

        bc.set(50 * 256);
        assert_eq!(bc.low().value(), 0);
        assert_eq!(bc.high().value(), 50);
        assert_eq!(bc.value(), 50 * 256);

        bc.set(0xFFFF);
        bc.add(1);
        assert_eq!(bc.value(), 0);

        bc.set(0x4000);
        bc.add(1);
        assert_eq!(bc.value(), 0x4001);
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

        z80.a_mut().set(0);
        assert_eq!(0, z80.a().value());
        z80.a_mut().add(1);
        assert_eq!(1, z80.a().value());

        z80.a_mut().set(0);
        assert_eq!(0, z80.a().value());
        z80.a_mut().inc();
        assert_eq!(1, z80.a().value());
        z80.a_mut().dec();
        assert_eq!(0, z80.a().value());
        z80.a_mut().dec();
        assert_eq!(0xFF, z80.a().value());
        z80.a_mut().inc();
        assert_eq!(0, z80.a().value());
    }

    #[test]
    fn eval() {
        let mut z80 = Z80::default();
        z80.pc_mut().set(0x4000);
        z80.hl_mut().set(0x8000);
        z80.de_mut().set(0xC000);
        z80.a_mut().set(0);

        let pop_bc = Token::OpCode(
            Mnemonic::Pop,
            Some(DataAccess::Register16(tokens::Register16::Bc)),
            None,
            None
        );
        let ld_l_a = Token::OpCode(
            Mnemonic::Ld,
            Some(DataAccess::Register8(tokens::Register8::L)),
            Some(DataAccess::Register8(tokens::Register8::A)),
            None
        );
        let add_a_b = Token::OpCode(
            Mnemonic::Add,
            Some(DataAccess::Register8(tokens::Register8::A)),
            Some(DataAccess::Register8(tokens::Register8::B)),
            None
        );
        let ldi = Token::OpCode(Mnemonic::Ldi, None, None, None);

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
        assert_eq!(z80.de().value(), 0xC001);
    }
}
