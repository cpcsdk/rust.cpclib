use std::borrow::Cow;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;

use paste;

use crate::tokens::expression::*;
use crate::tokens::registers::*;

#[derive(Debug, PartialEq, Eq, Clone)]
/// Encode the way mnemonics access to data
#[allow(missing_docs)]
pub enum DataAccess {
    /// We are using an indexed register associated to its index
    IndexRegister16WithIndex(IndexRegister16, BinaryOperation, Expr),
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
    PortN(Expr)
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
            DataAccess::IndexRegister16WithIndex(ref reg, ref op, ref delta) => {
                write!(f, "({} {} {})", reg, op, delta)
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
            DataAccess::PortN(n) => write!(f, "({})", n)
        }
    }
}

pub trait DataAccessElem: Sized + Debug + Display {
    type Expr: ExprElement;

    fn get_expression(&self) -> Option<&Self::Expr>;
    fn get_flag_test(&self) -> Option<FlagTest>;
    fn get_index(&self) -> Option<(BinaryOperation, &Self::Expr)>;
    fn get_indexregister16(&self) -> Option<IndexRegister16>;
    fn get_indexregister8(&self) -> Option<IndexRegister8>;
    fn get_register16(&self) -> Option<Register16>;
    fn get_register8(&self) -> Option<Register8>;
    fn is_address_in_indexregister16(&self) -> bool;
    fn is_address_in_register16(&self) -> bool;
    #[inline]
    fn is_address_in_hl(&self) -> bool {
        self.is_address_in_register16() && 
            self.get_register16() == Some(Register16::Hl) 
    }
    fn is_indexregister_with_index(&self) -> bool;
    fn is_indexregister16(&self) -> bool;
    fn is_indexregister8(&self) -> bool;
    fn is_expression(&self) -> bool;
    fn is_memory(&self) -> bool;
    fn is_port_c(&self) -> bool;
    fn is_port_n(&self) -> bool;
    fn is_register_a(&self) -> bool;
    fn is_register_af(&self) -> bool;
    fn is_register_b(&self) -> bool;
    fn is_register_bc(&self) -> bool;
    fn is_register_c(&self) -> bool;
    fn is_register_d(&self) -> bool;
    fn is_register_de(&self) -> bool;
    fn is_register_e(&self) -> bool;
    fn is_register_h(&self) -> bool;
    fn is_register_hl(&self) -> bool;
    fn is_register_i(&self) -> bool;
    fn is_register_ix(&self) -> bool;
    fn is_register_ixh(&self) -> bool;
    fn is_register_ixl(&self) -> bool;
    fn is_register_iy(&self) -> bool;
    fn is_register_iyh(&self) -> bool;
    fn is_register_iyl(&self) -> bool;
    fn is_register_l(&self) -> bool;
    fn is_register_r(&self) -> bool;
    fn is_register_sp(&self) -> bool;
    fn is_register16(&self) -> bool;
    fn is_register8(&self) -> bool;
    fn to_data_access_for_high_register(&self) -> Option<Self>;
    fn to_data_access_for_low_register(&self) -> Option<Self>;
    fn to_data_access(&self) -> Cow<DataAccess>;
}

#[macro_export]
macro_rules! data_access_is_any_indexregister8 {
    ($($reg:ident)*) => {$(
        paste::paste! {
           // impl DataAccess {
                /// Check if this DataAccess corresonds to $reg
                fn [<is_register_ $reg:lower>] (&self) -> bool {
                    match self {
                        Self::IndexRegister8(IndexRegister8::$reg, ..) => true,
                        _ => false,
                    }
                }
            }
      //  }
    )*}
}

#[macro_export]
macro_rules! data_access_is_any_register8 {
    ($($reg:ident)*) => {$(
        paste::paste! {
          //  impl DataAccess {
                /// Check if this DataAccess corresonds to $reg
                fn [<is_register_ $reg:lower>] (&self) -> bool {
                    match self {
                        Self::Register8(Register8::$reg, ..) => true,
                        _ => false,
                    }
                }
            }
     //   }
    )*}
}

#[macro_export]
macro_rules! data_access_is_any_register16 {
    ($($reg:ident)*) => {$(
        paste::paste! {
         //   impl DataAccess {
                /// Check if this DataAccess corresonds to $reg
                fn [<is_register_ $reg:lower>] (&self) -> bool {
                    match self {
                        Self::Register16(Register16::$reg, ..) => true,
                        _ => false,
                    }
                }
       //     }
        }
    )*}
}

#[macro_export]
macro_rules! data_access_is_any_indexregister16 {
    ($($reg:ident)*) => {$(
        paste::paste! {
       //     impl DataAccess {
                /// Check if this DataAccess corresonds to $reg
                fn [<is_register_ $reg:lower>] (&self) -> bool {
                    match self {
                        Self::IndexRegister16(IndexRegister16::$reg, ..) => true,
                        _ => false,
                    }
                }
         //   }
        }
    )*}
}

#[macro_export]
macro_rules! data_access_impl_most_methods {
    () => {

    fn get_flag_test(&self) -> Option<FlagTest> {
        match self {
            Self::FlagTest(test, ..) => Some(*test),
            _ => None
        }
    }

    fn is_register8(&self) -> bool {
        match self {
            Self::Register8(_, ..) => true,
            _ => false
        }
    }

    fn is_register16(&self) -> bool {
        match self {
            Self::Register16(_, ..) => true,
            _ => false
        }
    }

    fn is_indexregister8(&self) -> bool {
        match self {
            Self::IndexRegister8(_, ..) => true,
            _ => false
        }
    }

    fn is_indexregister16(&self) -> bool {
        match self {
            Self::IndexRegister16(_, ..) => true,
            _ => false
        }
    }

    fn is_indexregister_with_index(&self) -> bool {
        match self {
            Self::IndexRegister16WithIndex(..) => true,
            _ => false
        }
    }

    fn is_memory(&self) -> bool {
        match self {
            Self::Memory(_, ..) => true,
            _ => false
        }
    }

    fn is_address_in_register16(&self) -> bool {
        match self {
            Self::MemoryRegister16(_, ..) => true,
            _ => false
        }
    }

    fn is_address_in_indexregister16(&self) -> bool {
        match self {
            Self::MemoryIndexRegister16(_, ..) => true,
            _ => false
        }
    }

    fn get_register16(&self) -> Option<Register16> {
        match self {
            Self::Register16(ref reg, ..) => Some(*reg),
            Self::MemoryRegister16(ref reg, ..) => Some(*reg),
            _ => None
        }
    }

    fn get_indexregister16(&self) -> Option<IndexRegister16> {
        match self {
            Self::IndexRegister16(ref reg, ..) => Some(*reg),
            Self::MemoryIndexRegister16(ref reg, ..) => Some(*reg),
            Self::IndexRegister16WithIndex(reg, .., _) => Some(*reg),
            _ => None
        }
    }


    fn get_register8(&self) -> Option<Register8> {
        match self {
            Self::Register8(ref reg, ..) => Some(*reg),
            _ => None
        }
    }

    fn get_indexregister8(&self) -> Option<IndexRegister8> {
        match self {
            Self::IndexRegister8(ref reg, ..) => Some(*reg),
            _ => None
        }
    }

    fn get_expression(&self) -> Option<&Self::Expr> {
        match self {
            Self::Expression(exp, ..) |
            Self::Memory(exp, ..) |
            Self::PortN(exp,..) => Some(exp),
            
            _ => None
        }
    }

    fn is_port_n(&self) -> bool {
        match self {
            Self::PortN(..) => true,
            _ => false
        }
    }

    fn is_expression(&self) -> bool {
        match self {
            Self::Expression(..) => true,
            _ => false
        }
    }

    fn get_index(&self) -> Option<(BinaryOperation, &Self::Expr)> {
        match self {
            Self::IndexRegister16WithIndex(_, op, exp, ..) => Some((*op, exp)),
            _ => None
        }
    }



    data_access_is_any_indexregister8!(Ixh Ixl Iyh Iyl);
    data_access_is_any_register8!(A B C D E H L);
    data_access_is_any_register16!(Af Bc De Hl Sp);
    data_access_is_any_indexregister16!(Ix Iy);
    }
}

#[allow(missing_docs)]
impl DataAccessElem for DataAccess {
    type Expr = Expr;

    data_access_impl_most_methods!();

    fn to_data_access_for_low_register(&self) -> Option<Self> {
        match self {
            Self::IndexRegister16(ref reg, ..) => Some(reg.low().into()),
            Self::Register16(ref reg, ..) => Some(reg.low().unwrap().into()),
            _ => None
        }
    }

    fn to_data_access_for_high_register(&self) -> Option<Self> {
        match self {
            Self::IndexRegister16(ref reg, ..) => Some(reg.high().into()),
            Self::Register16(ref reg, ..) => Some(reg.high().unwrap().into()),
            _ => None
        }
    }

    fn is_port_c(&self) -> bool {
        match self {
            Self::PortC => true,
            _ => false
        }
    }



    fn is_register_i(&self) -> bool {
        match self {
            Self::SpecialRegisterI => true,
            _ => false
        }
    }

    fn is_register_r(&self) -> bool {
        match self {
            Self::SpecialRegisterR => true,
            _ => false
        }
    }

    fn to_data_access(&self) -> Cow<DataAccess> {
        Cow::Borrowed(self)
    }
}

impl DataAccess {
    fn expression_mut(&mut self) -> Option<&mut Expr> {
        match self {
            DataAccess::Expression(ref mut exp) => Some(exp),
            _ => None
        }
    }
}

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
