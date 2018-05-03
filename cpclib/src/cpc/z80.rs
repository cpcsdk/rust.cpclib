///! Manage z80 CPU
///! Could be used to simulate or generate code

use std::mem::swap;
use std::fmt::Debug;
use std::fmt;

pub trait HasValue {
    type ValueType;

    /// Retreive the stored value
    fn value(&self) -> Self::ValueType;

    fn set(&mut self, value:Self::ValueType);
}

/// Represents an 8 bit register
#[derive(Copy,Clone,Debug)]
pub struct Register8{
    val: u8
}


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
}



/// Represents a 16 bits register
#[derive(Copy,Clone)]
#[repr(C)]
pub union Register16 {
    val: u16,
    units: [Register8;2]
}



impl Debug for Register16 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {write!(f, "({:?}, {:?})", &self.units[0], &self.units[1])}
    }

}

impl Default for Register16 {
    fn default() -> Self {
        Self{units: [Register8::default(), Register8::default()]}
    }
}

// XXX strict copy paste of Register8
impl HasValue for Register16 {
    type ValueType=u16;

    fn value(&self) -> Self::ValueType{
        unsafe{self.val}
    }

    fn set(&mut self, value:Self::ValueType) {
        self.val = value;
    }
}


impl Register16 {

    pub fn low(& mut self) -> & mut Register8 {
        unsafe{& mut self.units[0]}
    }

    pub fn high(& mut self) -> &mut Register8 {
        unsafe{& mut self.units[1]}
    }
}


/// Z80 CPU model
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
    pub fn pc(&mut self)-> &mut Register16 {&mut self.reg_pc}
    pub fn sp(&mut self)-> &mut Register16 {&mut self.reg_sp}

    pub fn af(&mut self)-> &mut Register16 {&mut self.reg_af}

    pub fn bc(&mut self)-> &mut Register16 {&mut self.reg_bc}
    pub fn de(&mut self)-> &mut Register16 {&mut self.reg_de}
    pub fn hl(&mut self)-> &mut Register16 {&mut self.reg_hl}

    pub fn ix(&mut self)-> &mut Register16 {&mut self.reg_ix}
    pub fn iy(&mut self)-> &mut Register16 {&mut self.reg_iy}

    pub fn a(&mut self)-> &mut Register8 {
       let tmp = self.af();
       tmp.high()
    }
    pub fn f(&mut self)-> &mut Register8 {
       let tmp = self.af();
       tmp.low()
    }


    pub fn b(&mut self)-> &mut Register8 {
       let tmp = self.bc();
       tmp.high()
    }
    pub fn c(&mut self)-> &mut Register8 {
       let tmp = self.bc();
       tmp.low()
    }


    pub fn d(&mut self)-> &mut Register8 {
       let tmp = self.de();
       tmp.high()
    }
    pub fn e(&mut self)-> &mut Register8 {
       let tmp = self.de();
       tmp.low()
    }


    pub fn h(&mut self)-> &mut Register8 {
       let tmp = self.hl();
       tmp.high()
    }
    pub fn l(&mut self)-> &mut Register8 {
       let tmp = self.hl();
       tmp.low()
    }




    pub fn ex_af_af_prime(&mut self) {
        swap(& mut self.reg_af_prime, & mut self.reg_af);
    }

    pub fn exx(&mut self) {
        swap(& mut self.reg_hl_prime, & mut self.reg_hl);
        swap(& mut self.reg_de_prime, & mut self.reg_de);
        swap(& mut self.reg_bc_prime, & mut self.reg_bc);
    }
}
