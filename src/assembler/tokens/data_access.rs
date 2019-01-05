use std::fmt;
use crate::assembler::tokens::expression::*;
use crate::assembler::tokens::registers::*;


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


impl From<&str> for DataAccess {
    fn from(txt: &str) -> DataAccess {
        DataAccess::Expression(txt.into())
    }
}

impl From<Register8> for DataAccess {
    fn from(reg: Register8) -> DataAccess {
        DataAccess::Register8(reg)
    }
}

impl From<Register16> for DataAccess {
    fn from(reg: Register16) -> DataAccess {
        DataAccess::Register16(reg)
    }
}


impl From<IndexRegister8> for DataAccess {
    fn from(reg: IndexRegister8) -> DataAccess {
        DataAccess::IndexRegister8(reg)
    }
}

impl From<IndexRegister16> for DataAccess {
    fn from(reg: IndexRegister16) -> DataAccess {
        DataAccess::IndexRegister16(reg)
    }
}

impl From<FlagTest> for DataAccess {
    fn from(test: FlagTest) -> DataAccess {
        DataAccess::FlagTest(test)
    }
}



impl fmt::Display for DataAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
