use std::fmt;
use std::str::FromStr;

use cpclib_common::itertools::Itertools;

use crate::error::*;

/// Encode a flag of the snaphot
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
#[allow(non_camel_case_types)]
#[allow(missing_docs)]
pub enum SnapshotFlag {
    Z80_AF,
    Z80_F,
    Z80_A,
    Z80_BC,
    Z80_C,
    Z80_B,
    Z80_DE,
    Z80_E,
    Z80_D,
    Z80_HL,
    Z80_L,
    Z80_H,
    Z80_R,
    Z80_I,
    Z80_IFF0,
    Z80_IFF1,
    Z80_IX,
    Z80_IXL,
    Z80_IXH,
    Z80_IY,
    Z80_IYL,
    Z80_IYH,
    Z80_SP,
    Z80_PC,
    Z80_IM,
    Z80_AFX,
    Z80_FX,
    Z80_AX,
    Z80_BCX,
    Z80_CX,
    Z80_BX,
    Z80_DEX,
    Z80_EX,
    Z80_DX,
    Z80_HLX,
    Z80_LX,
    Z80_HX,
    GA_PAL(Option<usize>),
    GA_PEN,
    /// Naming of this one is inapproriate as documentatoin state: This byte in the snapshot represents the multi-configuration register of the Gate-Array. This byte is the last byte written to this register. For CPCEMU compatibility, bit 7 should be set to "1" and bit 6 and bit 5 set to "0".
    /// Use it to select screen mode
    GA_ROMCFG,
    GA_RAMCFG,
    CRTC_REG(Option<usize>),
    CRTC_SEL,
    ROM_UP,
    PPI_A,
    PPI_B,
    PPI_C,
    PPI_CTL,
    PSG_REG(Option<usize>),
    PSG_SEL,
    CPC_TYPE,
    INT_NUM,
    GA_MULTIMODE(Option<usize>),
    FDD_MOTOR,
    FDD_TRACK,
    PRNT_DATA,
    CRTC_TYPE,
    CRTC_HCC,
    CRTC_CLC,
    CRTC_RLC,
    CRTC_VAC,
    CRTC_VSWC,
    CRTC_HSWC,
    CRTC_STATE,
    GA_VSC,
    GA_ISC,
    INT_REQ
}

#[allow(missing_docs)]
impl SnapshotFlag {
    pub fn enumerate() -> &'static [Self; 67] {
        use self::SnapshotFlag::*;

        &[
            Z80_AF,
            Z80_F,
            Z80_A,
            Z80_BC,
            Z80_C,
            Z80_B,
            Z80_DE,
            Z80_E,
            Z80_D,
            Z80_HL,
            Z80_L,
            Z80_H,
            Z80_R,
            Z80_I,
            Z80_IFF0,
            Z80_IFF1,
            Z80_IX,
            Z80_IXL,
            Z80_IXH,
            Z80_IY,
            Z80_IYL,
            Z80_IYH,
            Z80_SP,
            Z80_PC,
            Z80_IM,
            Z80_AFX,
            Z80_FX,
            Z80_AX,
            Z80_BCX,
            Z80_CX,
            Z80_BX,
            Z80_DEX,
            Z80_EX,
            Z80_DX,
            Z80_HLX,
            Z80_LX,
            Z80_HX,
            GA_PAL(None),
            GA_PEN,
            GA_ROMCFG,
            GA_RAMCFG,
            CRTC_REG(None),
            CRTC_SEL,
            ROM_UP,
            PPI_A,
            PPI_B,
            PPI_C,
            PPI_CTL,
            PSG_REG(None),
            PSG_SEL,
            CPC_TYPE,
            INT_NUM,
            GA_MULTIMODE(None),
            FDD_MOTOR,
            FDD_TRACK,
            PRNT_DATA,
            CRTC_TYPE,
            CRTC_HCC,
            CRTC_CLC,
            CRTC_RLC,
            CRTC_VAC,
            CRTC_VSWC,
            CRTC_HSWC,
            CRTC_STATE,
            GA_VSC,
            GA_ISC,
            INT_REQ
        ]
    }

    /// Return the location in the header for the flag (and its potential index)
    pub fn offset(&self) -> usize {
        use self::SnapshotFlag::*;
        match self {
            GA_PAL(ref idx) | CRTC_REG(ref idx) | PSG_REG(ref idx) | &GA_MULTIMODE(ref idx) => {
                self.base() + idx.unwrap_or(0) * self.elem_size()
            }
            _ => self.base()
        }
    }

    pub fn indice(&self) -> Option<usize> {
        match self {
            Self::GA_PAL(ref idx)
            | Self::CRTC_REG(ref idx)
            | Self::PSG_REG(ref idx)
            | &Self::GA_MULTIMODE(ref idx) => *idx,
            _ => Some(0) // For standard stuff indice is considered to be 0
        }
    }

    pub fn set_indice(&mut self, indice: usize) -> Result<(), SnapshotError> {
        match self {
            Self::GA_PAL(ref mut idx)
            | Self::CRTC_REG(ref mut idx)
            | Self::PSG_REG(ref mut idx)
            | Self::GA_MULTIMODE(ref mut idx) => {
                *idx = Some(indice);
                Ok(())
            }
            _ => Err(SnapshotError::InvalidIndex)
        }
    }

    /// Return the header base position that corresponds to the flag
    #[allow(clippy::match_ref_pats)]
    pub fn base(&self) -> usize {
        use self::SnapshotFlag::*;
        match self {
            &Z80_AF | &Z80_F => 0x11,
            &Z80_A => 0x12,
            &Z80_BC | &Z80_C => 0x13,
            &Z80_B => 0x14,
            &Z80_DE | &Z80_E => 0x15,
            &Z80_D => 0x16,
            &Z80_HL | &Z80_L => 0x17,
            &Z80_H => 0x18,
            &Z80_R => 0x19,
            &Z80_I => 0x1A,
            &Z80_IFF0 => 0x1B,
            &Z80_IFF1 => 0x1C,
            &Z80_IX | &Z80_IXL => 0x1D,
            &Z80_IXH => 0x1E,
            &Z80_IY | &Z80_IYL => 0x1F,
            &Z80_IYH => 0x20,
            &Z80_SP => 0x21,
            &Z80_PC => 0x23,
            &Z80_IM => 0x25,
            &Z80_AFX | &Z80_FX => 0x26,
            &Z80_AX => 0x27,
            &Z80_BCX | &Z80_CX => 0x28,
            &Z80_BX => 0x29,
            &Z80_DEX | &Z80_EX => 0x2A,
            &Z80_DX => 0x2B,
            &Z80_HLX | &Z80_LX => 0x2C,
            &Z80_HX => 0x2D,
            &GA_PEN => 0x2E,
            &GA_PAL(_) => 0x2F,
            &GA_ROMCFG => 0x40,
            &GA_RAMCFG => 0x41,
            &CRTC_SEL => 0x42,
            &CRTC_REG(_) => 0x43,
            &ROM_UP => 0x55,
            &PPI_A => 0x56,
            &PPI_B => 0x57,
            &PPI_C => 0x58,
            &PPI_CTL => 0x59,
            &PSG_SEL => 0x5A,
            &PSG_REG(_) => 0x5B,
            &CPC_TYPE => 0x6D,
            &INT_NUM => 0x6E,
            &GA_MULTIMODE(_) => 0x6F,
            &FDD_MOTOR => 0x9C,
            &FDD_TRACK => 0x9D,
            &PRNT_DATA => 0xA1,
            &CRTC_TYPE => 0xA4,
            &CRTC_HCC => 0xA9,
            &CRTC_CLC => 0xAB,
            &CRTC_RLC => 0xAC,
            &CRTC_VAC => 0xAD,
            &CRTC_VSWC => 0xAE,
            &CRTC_HSWC => 0xAF,
            &CRTC_STATE => 0xB0,
            &GA_VSC => 0xB2,
            &GA_ISC => 0xB3,
            &INT_REQ => 0xB4
        }
    }

    /// Return the number of elements the flag can handle
    pub fn nb_elems(&self) -> usize {
        use self::SnapshotFlag::*;
        match self {
            GA_PAL(_) => 17,
            CRTC_REG(_) => 18,
            PSG_REG(_) => 16,
            GA_MULTIMODE(_) => 6,
            _ => 1
        }
    }

    /// Return the size of one unique element
    #[allow(clippy::match_same_arms, clippy::match_ref_pats)]
    pub fn elem_size(&self) -> usize {
        use self::SnapshotFlag::*;
        match self {
            &Z80_AF | &Z80_BC | &Z80_DE | &Z80_HL | &Z80_IX | &Z80_IY | &Z80_SP | &Z80_PC
            | &Z80_AFX | &Z80_BCX | &Z80_DEX | &Z80_HLX | &CRTC_STATE => 2,

            &Z80_F | &Z80_A | &Z80_C | &Z80_B | &Z80_E | &Z80_D | &Z80_L | &Z80_H | &Z80_R
            | &Z80_I | &Z80_IFF0 | &Z80_IFF1 | &Z80_IXL | &Z80_IXH | &Z80_IYL | &Z80_IYH
            | &Z80_IM | &Z80_FX | &Z80_AX | &Z80_CX | &Z80_BX | &Z80_EX | &Z80_DX | &Z80_LX
            | &Z80_HX | &GA_PEN | &GA_ROMCFG | &GA_RAMCFG | &CRTC_SEL | &ROM_UP | &PPI_A
            | &PPI_B | &PPI_C | &PPI_CTL | &PSG_SEL | &CPC_TYPE | &GA_VSC | &GA_ISC | &INT_REQ
            | &INT_NUM | &FDD_MOTOR | &FDD_TRACK | &PRNT_DATA | &CRTC_TYPE | &CRTC_HCC
            | &CRTC_CLC | &CRTC_RLC | &CRTC_VAC | &CRTC_VSWC | &CRTC_HSWC => 1,

            &GA_PAL(_) => 1,
            &CRTC_REG(_) => 1,
            &PSG_REG(_) => 1,
            &GA_MULTIMODE(_) => 1
        }
    }

    pub fn comment(&self) -> &str {
        use self::SnapshotFlag::*;

        match self {
            Z80_AF => "\t\tZ80 register AF",
            Z80_F => "\t\tZ80 register F",
            Z80_A => "\t\tZ80 register A",
            Z80_BC => "\t\tZ80 register BC",
            Z80_C => "\t\tZ80 register C",
            Z80_B => "\t\tZ80 register B",
            Z80_DE => "\t\tZ80 register DE",
            Z80_E => "\t\tZ80 register E",
            Z80_D => "\t\tZ80 register D",
            Z80_HL => "\t\tZ80 register HL",
            Z80_L => "\t\tZ80 register L",
            Z80_H => "\t\tZ80 register H",
            Z80_R => "\t\tZ80 register R",
            Z80_I => "\t\tZ80 register I",
            Z80_IFF0 => "\tZ80 interrupt flip-flop IFF0",
            Z80_IFF1 => "\tZ80 interrupt flip-flop IFF1",
            Z80_IX => "\t\tZ80 register IX",
            Z80_IXL => "\t\tZ80 register IX (low)",
            Z80_IXH => "\t\tZ80 register IX (high)",
            Z80_IY => "\t\tZ80 register IY",
            Z80_IYL => "\t\tZ80 register IY (low)",
            Z80_IYH => "\t\tZ80 register IY (high)",
            Z80_SP => "\t\tZ80 register SP",
            Z80_PC => "\t\tZ80 register PC",
            Z80_IM => "\t\tZ80 interrupt mode (0,1,2)",
            Z80_AFX => "\t\tZ80 register AF'",
            Z80_FX => "\t\tZ80 register F'",
            Z80_AX => "\t\tZ80 register A'",
            Z80_BCX => "\t\tZ80 register BC'",
            Z80_CX => "\t\tZ80 register C'",
            Z80_BX => "\t\tZ80 register B'",
            Z80_DEX => "\t\tZ80 register DE'",
            Z80_EX => "\t\tZ80 register E'",
            Z80_DX => "\t\tZ80 register D'",
            Z80_HLX => "\t\tZ80 register HL'",
            Z80_LX => "\t\tZ80 register L'",
            Z80_HX => "\t\tZ80 register H'",
            GA_PEN => "\t\tGA: index of selected pen",
            GA_PAL(_) => "\t\tGA: current palette (0..16)",
            GA_ROMCFG => "\tGA: multi configuration",
            GA_RAMCFG => "\tCurrent RAM configuration",
            CRTC_SEL => "\tCRTC: index of selected register",
            CRTC_REG(_) => "\tCRTC: register data (0..17)",
            ROM_UP => "\t\tCurrent ROM selection",
            PPI_A => "\t\tPPI: port A",
            PPI_B => "\t\tPPI: port B",
            PPI_C => "\t\tPPI: port C",
            PPI_CTL => "\t\tPPI: control port",
            PSG_SEL => "\t\tPSG: index of selected register",
            PSG_REG(_) => "\t\tPSG: register data (0..15)",
            CPC_TYPE => "\tCPC type: \n\t\t\t0 = CPC464\n\t\t\t1 = CPC664\n\t\t\t2 = CPC6128\n\t\t\t3 = unknown\n\t\t\t4 = 6128 Plus\n\t\t\t5 = 464 Plus\n\t\t\t6 = GX4000",
            INT_NUM => "\tinterrupt number (0..5)",
            GA_MULTIMODE(_) => "\t6 mode bytes (one for each halt)",
            FDD_MOTOR => "\tFDD motor drive state (0=off, 1=on)",
            FDD_TRACK => "\tFDD current physical track",
            PRNT_DATA => "\tPrinter Data/Strobe Register",
            CRTC_TYPE => "\tCRTC type:\n\t\t\t0 = HD6845S/UM6845\n\t\t\t1 = UM6845R\n\t\t\t2 = MC6845\n\t\t\t3 = 6845 in CPC+ ASIC\n\t\t\t4 = 6845 in Pre-ASIC",
            CRTC_HCC => "\tCRTC horizontal character counter register",
            CRTC_CLC => "\tCRTC character-line counter register",
            CRTC_RLC => "\tCRTC raster-line counter register",
            CRTC_VAC => "\tCRTC vertical total adjust counter register",
            CRTC_VSWC => "\tCRTC horizontal sync width counter",
            CRTC_HSWC => "\tCRTC vertical sync width counter",
            CRTC_STATE => "\tCRTC state flags. \n\t\t\t0 if '1'/'0' VSYNC active/inactive\n\t\t\t1 if '1'/'0' HSYNC active/inactive\n\t\t\t2-7 reserved\n\t\t\t7 if '1'/'0' Vert Total Adjust active/inactive\n\t\t\t8-15 reserved (0)",
            GA_VSC => "\t\tGA vsync delay counter",
            GA_ISC => "\t\tGA interrupt scanline counter",
            INT_REQ => "\t\tInterrupt request flag\n\t\t\t0=no interrupt requested\n\t\t\t1=interrupt requested",
        }
    }
}

impl FromStr for SnapshotFlag {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_uppercase();

        if s.contains(':') {
            let elems = s.split(':').collect::<Vec<_>>();
            let idx = match elems[1].parse::<usize>() {
                Ok(idx) => idx,
                Err(_) => return Err(String::from("Unable to parse index"))
            };

            let indexed_flag = match elems[0] {
                "GA_PAL" => SnapshotFlag::GA_PAL(Some(idx)),
                "CRTC_REG" => SnapshotFlag::CRTC_REG(Some(idx)),
                "PSG_REG" => SnapshotFlag::PSG_REG(Some(idx)),
                "GA_MULTIMODE" => SnapshotFlag::GA_MULTIMODE(Some(idx)),
                _ => {
                    return Err(String::from("Unable to convert string to a flag"));
                }
            };

            if indexed_flag.indice().unwrap() < indexed_flag.nb_elems() {
                Ok(indexed_flag)
            }
            else {
                Err(format!("Wrong index size {:?}", indexed_flag))
            }
        }
        else {
            match s.as_str() {
                //           "GA_PAL" => Ok(SnapshotFlag::GA_PAL(None)),
                // "CRTC_REG" => Ok(SnapshotFlag::CRTC_REG(None)),
                // "PSG_REG" => Ok(SnapshotFlag::PSG_REG(None)),
                // "GA_MULTIMODE" => Ok(SnapshotFlag::GA_MULTIMODE(None)),
                "Z80_AF" => Ok(SnapshotFlag::Z80_AF),
                "Z80_F" => Ok(SnapshotFlag::Z80_F),
                "Z80_A" => Ok(SnapshotFlag::Z80_A),
                "Z80_BC" => Ok(SnapshotFlag::Z80_BC),
                "Z80_C" => Ok(SnapshotFlag::Z80_C),
                "Z80_B" => Ok(SnapshotFlag::Z80_B),
                "Z80_DE" => Ok(SnapshotFlag::Z80_DE),
                "Z80_E" => Ok(SnapshotFlag::Z80_E),
                "Z80_D" => Ok(SnapshotFlag::Z80_D),
                "Z80_HL" => Ok(SnapshotFlag::Z80_HL),
                "Z80_L" => Ok(SnapshotFlag::Z80_L),
                "Z80_H" => Ok(SnapshotFlag::Z80_H),
                "Z80_R" => Ok(SnapshotFlag::Z80_R),
                "Z80_I" => Ok(SnapshotFlag::Z80_I),
                "Z80_IFF0" => Ok(SnapshotFlag::Z80_IFF0),
                "Z80_IFF1" => Ok(SnapshotFlag::Z80_IFF1),
                "Z80_IX" => Ok(SnapshotFlag::Z80_IX),
                "Z80_IXL" => Ok(SnapshotFlag::Z80_IXL),
                "Z80_IXH" => Ok(SnapshotFlag::Z80_IXH),
                "Z80_IY" => Ok(SnapshotFlag::Z80_IY),
                "Z80_IYL" => Ok(SnapshotFlag::Z80_IYL),
                "Z80_IYH" => Ok(SnapshotFlag::Z80_IYH),
                "Z80_SP" => Ok(SnapshotFlag::Z80_SP),
                "Z80_PC" => Ok(SnapshotFlag::Z80_PC),
                "Z80_IM" => Ok(SnapshotFlag::Z80_IM),
                "Z80_AFX" => Ok(SnapshotFlag::Z80_AFX),
                "Z80_FX" => Ok(SnapshotFlag::Z80_FX),
                "Z80_AX" => Ok(SnapshotFlag::Z80_AX),
                "Z80_BCX" => Ok(SnapshotFlag::Z80_BCX),
                "Z80_CX" => Ok(SnapshotFlag::Z80_CX),
                "Z80_BX" => Ok(SnapshotFlag::Z80_BX),
                "Z80_DEX" => Ok(SnapshotFlag::Z80_DEX),
                "Z80_EX" => Ok(SnapshotFlag::Z80_EX),
                "Z80_DX" => Ok(SnapshotFlag::Z80_DX),
                "Z80_HLX" => Ok(SnapshotFlag::Z80_HLX),
                "Z80_LX" => Ok(SnapshotFlag::Z80_LX),
                "Z80_HX" => Ok(SnapshotFlag::Z80_HX),
                "GA_PEN" => Ok(SnapshotFlag::GA_PEN),
                "GA_ROMCFG" => Ok(SnapshotFlag::GA_ROMCFG),
                "GA_RAMCFG" => Ok(SnapshotFlag::GA_RAMCFG),
                "CRTC_SEL" => Ok(SnapshotFlag::CRTC_SEL),
                "ROM_UP" => Ok(SnapshotFlag::ROM_UP),
                "PPI_A" => Ok(SnapshotFlag::PPI_A),
                "PPI_B" => Ok(SnapshotFlag::PPI_B),
                "PPI_C" => Ok(SnapshotFlag::PPI_C),
                "PPI_CTL" => Ok(SnapshotFlag::PPI_CTL),
                "PSG_SEL" => Ok(SnapshotFlag::PSG_SEL),
                "CPC_TYPE" => Ok(SnapshotFlag::CPC_TYPE),
                "INT_NUM" => Ok(SnapshotFlag::INT_NUM),
                "FDD_MOTOR" => Ok(SnapshotFlag::FDD_MOTOR),
                "FDD_TRACK" => Ok(SnapshotFlag::FDD_TRACK),
                "PRNT_DATA" => Ok(SnapshotFlag::PRNT_DATA),
                "CRTC_TYPE" => Ok(SnapshotFlag::CRTC_TYPE),
                "CRTC_HCC" => Ok(SnapshotFlag::CRTC_HCC),
                "CRTC_CLC" => Ok(SnapshotFlag::CRTC_CLC),
                "CRTC_RLC" => Ok(SnapshotFlag::CRTC_RLC),
                "CRTC_VAC" => Ok(SnapshotFlag::CRTC_VAC),
                "CRTC_VSWC" => Ok(SnapshotFlag::CRTC_VSWC),
                "CRTC_HSWC" => Ok(SnapshotFlag::CRTC_HSWC),
                "CRTC_STATE" => Ok(SnapshotFlag::CRTC_STATE),
                "GA_VSC" => Ok(SnapshotFlag::GA_VSC),
                "GA_ISC" => Ok(SnapshotFlag::GA_ISC),
                "INT_REQ" => Ok(SnapshotFlag::INT_REQ),

                "GA_PAL" | "CRTC_REG" | "PSG_REG" | "GA_MULTIMODE" => {
                    Err(format!("{} requires an indice", s))
                }
                _ => Err(String::from("Unable to convert string to a flag"))
            }
        }
    }
}

/// Encode the type of the flag values
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum FlagValue {
    /// The flag is a byte
    Byte(u8),
    /// The flag is a word
    Word(u16),
    /// The flag is a list of bytes or words
    Array(Vec<FlagValue>) // Restr$icted to Byte or Word
}

impl fmt::Display for FlagValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            FlagValue::Byte(ref val) => write!(f, "0x{:.2x}", val),
            FlagValue::Word(ref val) => write!(f, "0x{:.4x}", val),
            FlagValue::Array(ref array) => {
                write!(f, "[")
                    .and_then(|_x| {
                        write!(
                            f,
                            "{}",
                            &array.iter().map(|b| format!("{}", b)).join(", ")
                        )
                    })
                    .and_then(|_x| write!(f, "]"))
            }
        }
    }
}

impl FlagValue {
    pub fn as_u16(&self) -> Option<u16> {
        match self {
            Self::Byte(b) => Some(*b as u16),
            Self::Word(w) => Some(*w),
            _ => None
        }
    }
}
