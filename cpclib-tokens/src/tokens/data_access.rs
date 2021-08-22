use crate::tokens::expression::*;
use crate::tokens::registers::*;

use paste;
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone)]
/// Encode the way mnemonics access to data
#[allow(missing_docs)]
pub enum DataAccess {
    /// We are using an indexed register associated to its index
    IndexRegister16WithIndex(IndexRegister16, Expr),
    IndexRegister16(IndexRegister16),
    IndexRegister8(IndexRegister8),
    /// Represents a standard 16 bits register
    Register16(Register16),
    /// Represents a standard 8 bits register
    Register8(Register8),
    /// Represents a memory access indexed by a register
    MemoryRegister16(Register16),
    MemoryIndexRegister16(IndexRegister16),
    /// Represents any expression
    Expression(Expr),
    /// Represents an address
    Memory(Expr),
    /// Represnts the test of bit flag
    FlagTest(FlagTest),
    /// Special register I
    SpecialRegisterI,
    /// Special register R
    SpecialRegisterR,
    /// Used for in/out instructions
    PortC,
    /// Used for in/out instructions
    PortN(Expr),
}

impl From<u8> for DataAccess {
    fn from(val: u8) -> Self {
        DataAccess::Expression(Expr::from(val))
    }
}

impl From<Expr> for DataAccess {
    fn from(exp: Expr) -> Self {
        DataAccess::Expression(exp)
    }
}

impl From<&str> for DataAccess {
    fn from(txt: &str) -> Self {
        DataAccess::Expression(Expr::from(txt))
    }
}

impl From<Register8> for DataAccess {
    fn from(reg: Register8) -> Self {
        DataAccess::Register8(reg)
    }
}

impl From<Register16> for DataAccess {
    fn from(reg: Register16) -> Self {
        DataAccess::Register16(reg)
    }
}

impl From<IndexRegister8> for DataAccess {
    fn from(reg: IndexRegister8) -> Self {
        DataAccess::IndexRegister8(reg)
    }
}

impl From<IndexRegister16> for DataAccess {
    fn from(reg: IndexRegister16) -> Self {
        DataAccess::IndexRegister16(reg)
    }
}

impl From<FlagTest> for DataAccess {
    fn from(test: FlagTest) -> Self {
        DataAccess::FlagTest(test)
    }
}

impl fmt::Display for DataAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataAccess::IndexRegister16WithIndex(ref reg, ref delta) => {
                if delta.is_negated() {
                    write!(f, "({} - {})", reg, delta)
                } else {
                    write!(f, "({} + {})", reg, delta)
                }
            }
            DataAccess::IndexRegister16(ref reg) => write!(f, "{}", reg),
            DataAccess::Register16(ref reg) => write!(f, "{}", reg),
            DataAccess::IndexRegister8(ref reg) => write!(f, "{}", reg),
            DataAccess::Register8(ref reg) => write!(f, "{}", reg),
            DataAccess::MemoryRegister16(ref reg) => write!(f, "({})", reg),
            DataAccess::MemoryIndexRegister16(ref reg) => write!(f, "({})", reg),
            DataAccess::Expression(ref exp) => write!(f, "{}", exp.to_simplified_string()),
            DataAccess::Memory(ref exp) => write!(f, "({})", exp.to_simplified_string()),
            DataAccess::FlagTest(ref test) => write!(f, "{}", test),
            DataAccess::SpecialRegisterI => write!(f, "I"),
            DataAccess::SpecialRegisterR => write!(f, "R"),
            DataAccess::PortC => write!(f, "(C)"),
            DataAccess::PortN(n) => write!(f, "({})", n),
        }
    }
}

impl DataAccess {
    /*
    /// Rename the local labels used in macros (with @)
    pub fn fix_local_macro_labels_with_seed(&mut self, seed: usize) {
        match self {
            Self::Expression(e)
            | Self::IndexRegister16WithIndex(_, e)
            | Self::Memory(e)
            | Self::PortN(e) => {
                e.fix_local_macro_labels_with_seed(seed);
            }
            _ => {
                // nothing to do there
            }
        }
    }
    */
}

#[allow(missing_docs)]
impl DataAccess {
    pub fn expr(&self) -> Option<&Expr> {
        match self {
            DataAccess::Expression(ref expr) => Some(expr),
            _ => None,
        }
    }

    pub fn is_portc(&self) -> bool {
        match self {
            DataAccess::PortC => true,
            _ => false,
        }
    }

    pub fn is_register_i(&self) -> bool {
        match self {
            DataAccess::SpecialRegisterI => true,
            _ => false,
        }
    }

    pub fn is_register_r(&self) -> bool {
        match self {
            DataAccess::SpecialRegisterR => true,
            _ => false,
        }
    }

    pub fn is_register8(&self) -> bool {
        match self {
            DataAccess::Register8(_) => true,
            _ => false,
        }
    }

    pub fn is_register16(&self) -> bool {
        match self {
            DataAccess::Register16(_) => true,
            _ => false,
        }
    }

    pub fn is_indexregister8(&self) -> bool {
        match self {
            DataAccess::IndexRegister8(_) => true,
            _ => false,
        }
    }

    pub fn is_indexregister16(&self) -> bool {
        match self {
            DataAccess::IndexRegister16(_) => true,
            _ => false,
        }
    }

    pub fn is_indexregister_with_index(&self) -> bool {
        match self {
            DataAccess::IndexRegister16WithIndex(_, _) => true,
            _ => false,
        }
    }

    pub fn is_memory(&self) -> bool {
        match self {
            DataAccess::Memory(_) => true,
            _ => false,
        }
    }

    pub fn is_address_in_register16(&self) -> bool {
        match self {
            DataAccess::MemoryRegister16(_) => true,
            _ => false,
        }
    }

    pub fn get_register16(&self) -> Option<Register16> {
        match self {
            DataAccess::Register16(ref reg) => Some(*reg),
            DataAccess::MemoryRegister16(ref reg) => Some(*reg),
            _ => None,
        }
    }

    pub fn get_indexregister16(&self) -> Option<IndexRegister16> {
        match self {
            DataAccess::IndexRegister16(ref reg) => Some(*reg),
            DataAccess::MemoryIndexRegister16(ref reg) => Some(*reg),
            &DataAccess::IndexRegister16WithIndex(ref reg, _) => Some(*reg),
            _ => None,
        }
    }

    pub fn to_data_access_for_low_register(&self) -> Option<DataAccess> {
        match self {
            DataAccess::IndexRegister16(ref reg) => Some(reg.low().into()),
            DataAccess::Register16(ref reg) => Some(reg.low().unwrap().into()),
            _ => None,
        }
    }

    pub fn to_data_access_for_high_register(&self) -> Option<DataAccess> {
        match self {
            DataAccess::IndexRegister16(ref reg) => Some(reg.high().into()),
            DataAccess::Register16(ref reg) => Some(reg.high().unwrap().into()),
            _ => None,
        }
    }

    pub fn get_register8(&self) -> Option<Register8> {
        match self {
            DataAccess::Register8(ref reg) => Some(*reg),
            _ => None,
        }
    }

    pub fn get_expression(&self) -> Option<&Expr> {
        match self {
            DataAccess::Expression(ref exp) => Some(&exp),
            _ => None,
        }
    }

    pub fn expression_mut(&mut self) -> Option<&mut Expr> {
        match self {
            DataAccess::Expression(ref mut exp) => Some(exp),
            _ => None,
        }
    }
}

macro_rules! is_any_indexregister8 {
    ($($reg:ident)*) => {$(
        paste::paste! {
            impl DataAccess {
                /// Check if this DataAccess corresonds to $reg
                pub fn [<is_register_ $reg:lower>] (&self) -> bool {
                    match self {
                        DataAccess::IndexRegister8(IndexRegister8::$reg) => true,
                        _ => false,
                    }
                }
            }
        }
    )*}
}
is_any_indexregister8!(Ixh Ixl Iyh Iyl);

macro_rules! is_any_register8 {
    ($($reg:ident)*) => {$(
        paste::paste! {
            impl DataAccess {
                /// Check if this DataAccess corresonds to $reg
                pub fn [<is_register_ $reg:lower>] (&self) -> bool {
                    match self {
                        DataAccess::Register8(Register8::$reg) => true,
                        _ => false,
                    }
                }
            }
        }
    )*}
}
is_any_register8!(A B C D E H L);

macro_rules! is_any_register16 {
    ($($reg:ident)*) => {$(
        paste::paste! {
            impl DataAccess {
                /// Check if this DataAccess corresonds to $reg
                pub fn [<is_register_ $reg:lower>] (&self) -> bool {
                    match self {
                        DataAccess::Register16(Register16::$reg) => true,
                        _ => false,
                    }
                }
            }
        }
    )*}
}
is_any_register16!(Af Bc De Hl Sp);

macro_rules! is_any_indexregister16 {
    ($($reg:ident)*) => {$(
        paste::paste! {
            impl DataAccess {
                /// Check if this DataAccess corresonds to $reg
                pub fn [<is_register_ $reg:lower>] (&self) -> bool {
                    match self {
                        DataAccess::IndexRegister16(IndexRegister16::$reg) => true,
                        _ => false,
                    }
                }
            }
        }
    )*}
}
is_any_indexregister16!(Ix Iy);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn is_indexregister8() {
        assert!(DataAccess::IndexRegister8(IndexRegister8::Ixl).is_indexregister8());
        assert!(DataAccess::IndexRegister8(IndexRegister8::Ixl).is_register_ixl());
        assert!(!DataAccess::IndexRegister8(IndexRegister8::Ixl).is_register_iyl());
        assert!(!DataAccess::Register8(Register8::A).is_register_iyl());
    }
}
