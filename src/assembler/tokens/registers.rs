use std::fmt;


#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub enum Register16 {
    Af,
    Hl,
    De,
    Bc,
    Sp
}
impl fmt::Display for Register16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            &Register16::Af => "AF",
            &Register16::Bc => "BC",
            &Register16::De => "DE",
            &Register16::Hl => "HL",
            &Register16::Sp => "SP"
        };
        write!(f, "{}", code)
    }
}

impl Register16 {
    /// Return the high 8bit register if exists
    pub fn high(&self) -> Option<Register8> {
        match self {
            Register16::Af => Some(Register8::A),
            Register16::Hl => Some(Register8::H),
            Register16::De => Some(Register8::D),
            Register16::Bc => Some(Register8::B),
            Register16::Sp => None
        }
    }

    /// Return the low 8bit register if exists
    pub fn low(&self) -> Option<Register8> {
        match self {
            Register16::Af => None,
            Register16::Hl => Some(Register8::L),
            Register16::De => Some(Register8::E),
            Register16::Bc => Some(Register8::C),
            Register16::Sp => None
        }
    }

    pub fn split(&self) -> (Option<Register8>, Option<Register8>) {
        (
            self.low(),
            self.high()
        )
    }
}


#[derive(Debug, PartialEq, Eq, Clone)]
pub enum IndexRegister16{
    Ix,
    Iy
}

impl fmt::Display for IndexRegister16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::*;
        let code = match self {
            &IndexRegister16::Ix => "IX",
            &IndexRegister16::Iy => "IY"
        };
        write!(f, "{}", code)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Hash)]
pub enum Register8 {
    A,
    B,
    C,
    D,
    E,
    H,
    L

}

impl Register8 {

    pub fn is_high(&self) -> bool {
        match self {
            &Register8::A | &Register8::B | &Register8::D | &Register8::H => true,
            _ => false
        }
    }


    pub fn is_low(&self) -> bool  {
        !self.is_high()
    }

    pub fn neighbourg(&self) -> Option<Register8> {
        match self {
            &Register8::A => None,
            &Register8::B => Some(Register8::C),
            &Register8::C => Some(Register8::B),
            &Register8::D => Some(Register8::E),
            &Register8::E => Some(Register8::D),
            &Register8::H => Some(Register8::L),
            &Register8::L => Some(Register8::H),
        }
    }


    /// Return the 16bit register than contains this one
    pub fn complete(&self) -> Register16 {
        match self {
            &Register8::A => Register16::Af,
            &Register8::B => Register16::Bc,
            &Register8::C => Register16::Bc,
            &Register8::D => Register16::De,
            &Register8::E => Register16::De,
            &Register8::H => Register16::Hl,
            &Register8::L => Register16::Hl,
        }
    }
}

impl fmt::Display for Register8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::*;
        let code = match self {
            &Register8::A => "A",
            &Register8::B => "B",
            &Register8::C => "C",
            &Register8::D => "D",
            &Register8::E => "E",
            &Register8::H => "H",
            &Register8::L => "L"
        };
        write!(f, "{}", code)
    }
}



#[derive(Debug, PartialEq, Eq, Clone)]
pub enum IndexRegister8 {
    Ixh,
    Ixl,
    Iyh,
    Iyl
}


impl fmt::Display for IndexRegister8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::*;
        let code = match self {
            &IndexRegister8::Ixh => "IXH",
            &IndexRegister8::Ixl => "IXL",
            &IndexRegister8::Iyh => "IYH",
            &IndexRegister8::Iyl => "IYL"
        };
        write!(f, "{}", code)
    }
}

        /*
#[derive(Debug, PartialEq, Eq)]
pub struct Label;

#[derive(Debug, PartialEq, Eq)]
pub enum Value{
    Label(),
    Constant
}
*/

// TODO add missing flags
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FlagTest {
    NZ,
    Z,
    NC,
    C,
    PO,
    PE,
    P,
    M
}

impl fmt::Display for FlagTest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            &FlagTest::NZ => "NZ",
            &FlagTest::Z => "Z",
            &FlagTest::NC => "NC",
            &FlagTest::C => "C",
            &FlagTest::PO => "PO",
            &FlagTest::PE => "PE",
            &FlagTest::P => "P",
            &FlagTest::M => "M"
        };
        write!(f, "{}", code)
    }
}


