use std::fmt;
use assembler::tokens::expression::*;
use assembler::tokens::registers::*;


#[derive(Debug, PartialEq, Eq, Clone)]
/// Encode the way mnemonics access to data
pub enum DataAccess {
    /// We are using an indexed register associated to its index
    IndexRegister16WithIndex(IndexRegister16, Oper, Expr),
    IndexRegister16(IndexRegister16),
    IndexRegister8(IndexRegister8),
    /// Represents a standard 16 bits register
    Register16(Register16),
    /// Represents a standard 8 bits register
    Register8(Register8),
    /// Represents a memory access indexed by a register
    MemoryRegister16(Register16),
    /// Represents any expression
    Expression(Expr),
    /// Represents an address
    Memory(Expr),
    /// Represnts the test of bit flag
    FlagTest(FlagTest)
}



impl fmt::Display for DataAccess {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &DataAccess::IndexRegister16WithIndex(ref reg, ref op,  ref delta) =>
                write!(f, "({} {} {})", reg, op, delta),
            &DataAccess::IndexRegister16(ref reg) =>
                write!(f, "{}", reg),
            &DataAccess::Register16(ref reg) =>
                write!(f, "{}", reg),
            &DataAccess::IndexRegister8(ref reg) =>
                write!(f, "{}", reg),
            &DataAccess::Register8(ref reg) =>
                write!(f, "{}", reg),
            &DataAccess::MemoryRegister16(ref reg) =>
                write!(f, "({})", reg),
            &DataAccess::Expression(ref exp) =>
                write!(f, "{}", exp),
            &DataAccess::Memory(ref exp) =>
                write!(f, "({})", exp),
            &DataAccess::FlagTest(ref test) =>
                write!(f, "{}", test)
        }
    }
}


impl DataAccess {
    pub fn expr(&self) -> Option<&Expr>{
        match self {
            &DataAccess::Expression(ref expr) => Some(expr),
            _ => None
        }
    }

    pub fn is_register8(&self) -> bool {
        match self {
            &DataAccess::Register8(_) => true,
            _ => false
        }
    }

    pub fn is_register16(&self) -> bool {
        match self {
            &DataAccess::Register16(_) => true,
            _ => false
        }
    }

    pub fn is_indexregister16(&self) -> bool {
        match self {
            &DataAccess::IndexRegister16(_) => true,
            _ => false
        }
    }



    pub fn is_memory(&self) -> bool {
        match self {
            &DataAccess::Memory(_) => true,
            _ => false
        }
    }

    pub fn is_address_in_register16(&self) -> bool {
        match self {
            &DataAccess::MemoryRegister16(_) => true,
            _ => false
        }
    }

    pub fn get_register16(&self) -> Option<Register16> {
        match self {
            &DataAccess::Register16(ref reg) => Some(reg.clone()),
            &DataAccess::MemoryRegister16(ref reg) => Some(reg.clone()),
            _ => None
        }
    }

    pub fn get_indexregister16(&self) -> Option<IndexRegister16> {
        match self {
            &DataAccess::IndexRegister16(ref reg) => Some(reg.clone()),
            _ => None
        }
    }

    pub fn get_register8(&self) -> Option<Register8> {
        match self {
            &DataAccess::Register8(ref reg) => Some(reg.clone()),
            _ => None
        }
    }
}
