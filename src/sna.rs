use bitsets;

use std::fmt;
use std::fs::File;

use std::io::prelude::*;

use std::path::Path;
use std::str::FromStr;

use num_enum::TryFromPrimitive;
use std::ops::AddAssign;
use std::ops::DerefMut;

///! Reimplementation of createsnapshot by Ramlaid/Arkos
///! in rust by Krusty/Benediction

/**
 * Original options

     {'i',"inSnapshot",0,1,1,"Load <$1> snapshot file"},
    {'l',"loadFileData",0,0,2,"Load <$1> file data at <$2> address in snapshot memory (or use AMSDOS header load address if <$2> is negative)"},
    {'p',"putData",0,0,2,"Put <$2> byte at <$1> address in snapshot memory"},
    {'s',"setToken",0,0,2,"Set snapshot token <$1> to value <$2>\n\t\t"
        "Use <$1>:<val> to set array value\n\t\t"
        "ex '-s CRTC_REG:6 20' : Set CRTC register 6 to 20"},
    {'x',"clearMemory",0,1,0,"Clear snapshot memory"},
    {'r',"romDeconnect",0,1,0,"Disconnect lower and upper rom"},
    {'e',"enableInterrupt",0,1,0,"Enable interrupt"},
    {'d',"disableInterrupt",0,1,0,"Disable interrupt"},
    {'c',"configFile",0,1,1,"Load a config file with createSnapshot option"},
    {'t',"tokenList",0,1,0,"Display setable snapshot token ID"},
    {'o',"output",0,0,3,"Output <$3> bytes of data from address <$2> to file <$1>"},
    {'f',"fillData",0,0,3,"Fill snapshot from <$1> over <$2> bytes, with <$3> datas"},
    {'g',"fillText",0,0,3,"Fill snapshot from <$1> over <$2> bytes, with <$3> text"},
    {'j',"loadIniFile",0,1,1,"Load <$1> init file"},
{'k',"saveIniFile",0,1,1,"Save <$1> init file"},

*/

#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub enum SnapshotVersion {
    /// Version 1 of Snapshsots
    V1 = 1,
    /// Version 2 of Snapshsots
    V2,
    /// Version 3 of Snapshsots (use of chunks)
    V3,
}

impl SnapshotVersion {
    /// Check if snapshot ius V3 version
    pub fn is_v3(self) -> bool {
        if let SnapshotVersion::V3 = self {
            true
        } else {
            false
        }
    }
}

/// Encode a flag of the snaphot
#[derive(PartialEq, Debug, Copy, Clone)]
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
    INT_REQ,
}

#[allow(missing_docs)]
impl SnapshotFlag {
    pub fn enumerate() -> [Self; 67] {
        use self::SnapshotFlag::*;

        [
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
            INT_REQ,
        ]
    }

    /// Return the location in the header for the flag (and its potential index)
    pub fn offset(&self) -> usize {
        use self::SnapshotFlag::*;
        match self {
            GA_PAL(ref idx) | CRTC_REG(ref idx) | PSG_REG(ref idx) | &GA_MULTIMODE(ref idx) => {
                self.base() + idx.unwrap_or(0) * self.elem_size()
            }
            _ => self.base(),
        }
    }

    pub fn indice(&self) -> Option<usize> {
        match self {
            Self::GA_PAL(ref idx)
            | Self::CRTC_REG(ref idx)
            | Self::PSG_REG(ref idx)
            | &Self::GA_MULTIMODE(ref idx) => *idx,
            _ => Some(0), // For standard stuff indice is considered to be 0
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
            _ => Err(SnapshotError::InvalidIndex),
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
            &Z80_I => 0x1a,
            &Z80_IFF0 => 0x1b,
            &Z80_IFF1 => 0x1c,
            &Z80_IX | &Z80_IXL => 0x1d,
            &Z80_IXH => 0x1e,
            &Z80_IY | &Z80_IYL => 0x1f,
            &Z80_IYH => 0x20,
            &Z80_SP => 0x21,
            &Z80_PC => 0x23,
            &Z80_IM => 0x25,
            &Z80_AFX | &Z80_FX => 0x26,
            &Z80_AX => 0x27,
            &Z80_BCX | &Z80_CX => 0x28,
            &Z80_BX => 0x29,
            &Z80_DEX | &Z80_EX => 0x2a,
            &Z80_DX => 0x2b,
            &Z80_HLX | &Z80_LX => 0x2c,
            &Z80_HX => 0x2d,
            &GA_PEN => 0x2e,
            &GA_PAL(_) => 0x2f,
            &GA_ROMCFG => 0x40,
            &GA_RAMCFG => 0x41,
            &CRTC_SEL => 0x42,
            &CRTC_REG(_) => 0x43,
            &ROM_UP => 0x55,
            &PPI_A => 0x56,
            &PPI_B => 0x57,
            &PPI_C => 0x58,
            &PPI_CTL => 0x59,
            &PSG_SEL => 0x5a,
            &PSG_REG(_) => 0x5b,
            &CPC_TYPE => 0x6d,
            &INT_NUM => 0x6e,
            &GA_MULTIMODE(_) => 0x6f,
            &FDD_MOTOR => 0x9c,
            &FDD_TRACK => 0x9d,
            &PRNT_DATA => 0xa1,
            &CRTC_TYPE => 0xa4,
            &CRTC_HCC => 0xa9,
            &CRTC_CLC => 0xab,
            &CRTC_RLC => 0xac,
            &CRTC_VAC => 0xad,
            &CRTC_VSWC => 0xae,
            &CRTC_HSWC => 0xaf,
            &CRTC_STATE => 0xb0,
            &GA_VSC => 0xb2,
            &GA_ISC => 0xb3,
            &INT_REQ => 0xb4,
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
            _ => 1,
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
            &GA_MULTIMODE(_) => 1,
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
        let s = &s.to_uppercase();

        if s.contains(':') {
            let elems = s.split(':').collect::<Vec<_>>();
            let idx = match elems[1].parse::<usize>() {
                Ok(idx) => idx,
                Err(_) => return Err(String::from("Unable to parse index")),
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
            } else {
                Err(format!("Wrong index size {:?}", indexed_flag))
            }
        } else {
            match s.as_str() {
                "GA_PAL" => Ok(SnapshotFlag::GA_PAL(None)),
                "CRTC_REG" => Ok(SnapshotFlag::CRTC_REG(None)),
                "PSG_REG" => Ok(SnapshotFlag::PSG_REG(None)),
                "GA_MULTIMODE" => Ok(SnapshotFlag::GA_MULTIMODE(None)),

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
                _ => Err(String::from("Unable to convert string to a flag")),
            }
        }
    }
}

/// Encode the type of the flag values
#[derive(Debug, Clone)]
pub enum FlagValue {
    /// The flag is a byte
    Byte(u8),
    /// The flag is a word
    Word(u16),
    /// The flag is a list of bytes or words
    Array(Vec<FlagValue>), // Restr$icted to Byte or Word
}

impl fmt::Display for FlagValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            FlagValue::Byte(ref val) => write!(f, "0x{:x}", val),
            FlagValue::Word(ref val) => write!(f, "0x{:x}", val),
            FlagValue::Array(ref array) => write!(f, "[")
                .and_then(|_x| {
                    write!(
                        f,
                        "{:?}",
                        &array.iter().map(|b| format!("{}", b)).collect::<Vec<_>>()
                    )
                })
                .and_then(|_x| write!(f, "]")),
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[allow(missing_docs)]
pub enum SnapshotError {
    FileError,
    NotEnougSpaceAvailable,
    InvalidValue,
    FlagDoesNotExists,
    InvalidIndex,
}

#[derive(Clone, Debug)]
/// Raw chunk data.
pub struct SnapshotChunkData {
    /// Identifier of the chunk
    code: [u8; 4],
    /// Content of the chunk
    data: Vec<u8>,
}

#[allow(missing_docs)]
impl SnapshotChunkData {
    pub fn code(&self) -> &[u8; 4] {
        &(self.code)
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn size_as_array(&self) -> [u8; 4] {
        let mut size = self.size();
        let mut array = [0, 0, 0, 0];

        for item in &mut array {
            *item = (size % 256) as u8;
            size /= 256;
        }

        array
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

#[derive(Clone, Debug)]
/// Memory chunk that superseeds the snapshot memory if any.
pub struct MemoryChunk {
    /// Raw content of the memory chunk (i.e. compressed version)
    data: SnapshotChunkData,
}

#[allow(missing_docs)]
impl MemoryChunk {
    /// Create a memory chunk.
    /// `code` identify with memory block is concerned
    /// `data` contains the crunched version of the code
    pub fn from(code: [u8; 4], data: Vec<u8>) -> Self {
        Self {
            data: SnapshotChunkData { code, data },
        }
    }

    /// Uncrunch the 64kbio of RLE crunched data
    pub fn uncrunched_memory(&self) -> Vec<u8> {
        let mut content = Vec::new();

        let idx = std::rc::Rc::new(std::cell::RefCell::new(0));
        let read_byte = || {
            let byte = self.data.data[*idx.borrow()];
            idx.borrow_mut().deref_mut().add_assign(1);
            byte
        };
        while *idx.borrow() != self.data.data.len() {
            match read_byte() {
                0xe5 => {
                    let amount = read_byte();
                    if amount != 0 {
                        let val = read_byte();
                        for _idx in 0..amount {
                            content.push(val);
                        }
                    }
                }
                val => {
                    content.push(val);
                }
            }
        }

        assert_eq!(content.len(), 64 * 1024);
        content
    }

    /// Returns the address in the memory array
    pub fn abstract_address(&self) -> usize {
        let nb = (self.data.code[3] - b'0') as usize;
        nb * 0x10000
    }
}

#[derive(Clone, Debug)]
/// Unknwon kind of chunk
pub struct UnknownChunk {
    /// Raw data of the chunk
    data: SnapshotChunkData,
}

impl UnknownChunk {
    /// Generate the chunk from raw data
    pub fn from(code: [u8; 4], data: Vec<u8>) -> Self {
        Self {
            data: SnapshotChunkData { code, data },
        }
    }
}

/*
pub struct BreakpointChunk {
    pub fn from(code: [u8;4], content: Vec<u8>) -> Self {
        unimplemented!()
    }
}

pub struct InsertedDiscChunk {
    pub fn from(code: [u8;4], content: Vec<u8>) -> Self {
        unimplemented!()
    }
}

pub struct CPCPlusChunk {
    pub fn from(code: [u8;4], content: Vec<u8>) -> Self {
        unimplemented!()
    }
}
*/

#[derive(Clone, Debug)]
/// Represents any kind of chunks in order to manipulate them easily based on their semantic
pub enum SnapshotChunk {
    /// The chunk is a memory chunk
    Memory(MemoryChunk),
    /// The type of the chunk is unknown
    Unknown(UnknownChunk),
}

#[allow(missing_docs)]
impl SnapshotChunk {
    pub fn is_memory_chunk(&self) -> bool {
        self.memory_chunk().is_some()
    }

    pub fn memory_chunk(&self) -> Option<&MemoryChunk> {
        match self {
            SnapshotChunk::Memory(ref mem) => Some(mem),
            _ => None,
        }
    }

    /// Provides the code of the chunk
    pub fn code(&self) -> &[u8; 4] {
        match self {
            SnapshotChunk::Memory(ref chunk) => chunk.data.code(),

            SnapshotChunk::Unknown(ref chunk) => chunk.data.code(),
        }
    }

    pub fn size(&self) -> usize {
        match self {
            SnapshotChunk::Memory(ref chunk) => chunk.data.size(),

            SnapshotChunk::Unknown(ref chunk) => chunk.data.size(),
        }
    }

    pub fn size_as_array(&self) -> [u8; 4] {
        match self {
            SnapshotChunk::Memory(ref chunk) => chunk.data.size_as_array(),

            SnapshotChunk::Unknown(ref chunk) => chunk.data.size_as_array(),
        }
    }

    pub fn data(&self) -> &[u8] {
        match self {
            SnapshotChunk::Memory(ref chunk) => chunk.data.data(),

            SnapshotChunk::Unknown(ref chunk) => chunk.data.data(),
        }
    }
}

impl From<MemoryChunk> for SnapshotChunk {
    fn from(chunk: MemoryChunk) -> Self {
        SnapshotChunk::Memory(chunk)
    }
}

impl From<UnknownChunk> for SnapshotChunk {
    fn from(chunk: UnknownChunk) -> Self {
        SnapshotChunk::Unknown(chunk)
    }
}

const PAGE_SIZE: usize = 0x4000;
const HEADER_SIZE: usize = 256;

/// 3 different states are possible. No memory, 64kb or 128kb
#[derive(Clone, Copy)]
#[allow(clippy::large_enum_variant)]
pub enum SnapshotMemory {
    /// No memory is stored within the snapshot
    Empty([u8; 0]),
    /// A 64kb page is stored within the snapshot
    SixtyFourKb([u8; PAGE_SIZE * 4]),
    /// A 128kb page is stored within the snapshot
    OneHundredTwentyHeightKb([u8; PAGE_SIZE * 8]),
}

impl Default for SnapshotMemory {
    fn default() -> Self {
        Self::Empty(Default::default())
    }
}

impl std::fmt::Debug for SnapshotMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = match self {
            Self::Empty(_) => "Empty",
            Self::SixtyFourKb(_) => "64kb",
            Self::OneHundredTwentyHeightKb(_) => "128kb",
        };
        write!(f, "SnapshotMemory ({})", code)
    }
}

#[allow(missing_docs)]
impl SnapshotMemory {
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Empty(_) => true,
            _ => false,
        }
    }

    pub fn is_64k(&self) -> bool {
        match self {
            Self::SixtyFourKb(_) => true,
            _ => false,
        }
    }

    pub fn is_128k(&self) -> bool {
        match self {
            Self::OneHundredTwentyHeightKb(_) => true,
            _ => false,
        }
    }

    /// Read only access to the memory
    fn memory(&self) -> &[u8] {
        match self {
            Self::Empty(ref mem) => mem,
            Self::SixtyFourKb(ref mem) => mem,
            Self::OneHundredTwentyHeightKb(ref mem) => mem,
        }
    }

    /// Reda write access to the memory
    fn memory_mut(&mut self) -> &mut [u8] {
        match self {
            Self::Empty(ref mut mem) => mem,
            Self::SixtyFourKb(ref mut mem) => mem,
            Self::OneHundredTwentyHeightKb(ref mut mem) => mem,
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Empty(_x) => 0,
            Self::SixtyFourKb(_) => 64 * 1024,
            Self::OneHundredTwentyHeightKb(_) => 128 * 1024,
        }
    }

    /// Produce a novel memory that is bigger
    fn increased_size(self) -> Self {
        match self {
            Self::Empty(ref _mem) => Self::default_64(),
            Self::SixtyFourKb(ref mem) => {
                let mut new = Self::default_128();
                new.memory_mut()[0..64 * 1024].copy_from_slice(mem);
                new
            }
            Self::OneHundredTwentyHeightKb(ref _mem) => unreachable!(),
        }
    }

    fn new(source: &[u8]) -> Self {
        match source.len() {
            0 => Self::default(),
            0x10000 => Self::new_64(source),
            0x20000 => Self::new_128(source),
            _ => unreachable!(),
        }
    }

    fn new_64(source: &[u8]) -> Self {
        assert_eq!(source.len(), 64 * 1024);
        let mut mem = [0; PAGE_SIZE * 4];
        mem.copy_from_slice(source);
        Self::SixtyFourKb(mem)
    }

    fn new_128(source: &[u8]) -> Self {
        assert_eq!(source.len(), 128 * 1024);
        let mut mem = [0; PAGE_SIZE * 8];
        mem.copy_from_slice(source);
        Self::OneHundredTwentyHeightKb(mem)
    }

    fn default_64() -> Self {
        Self::SixtyFourKb([0; PAGE_SIZE * 4])
    }

    fn default_128() -> Self {
        Self::OneHundredTwentyHeightKb([0; PAGE_SIZE * 8])
    }
}

/// Snapshot V3 representation. Can be saved in snapshot V1 or v2.
#[derive(Clone)]
#[allow(missing_docs)]
pub struct Snapshot {
    header: [u8; HEADER_SIZE],
    memory: SnapshotMemory,
    memory_already_written: bitsets::DenseBitSet,
    chunks: Vec<SnapshotChunk>,

    // nothing to do with the snapshot. Should be moved elsewhere
    pub debug: bool,
}

impl std::fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Snapshot ({{")?;
        write!(f, "\theader: TODO")?;
        write!(f, "memory: {:?}", &self.memory)?;
        write!(f, "chunks: {:?}", &self.chunks)?;
        write!(f, "}})")
    }
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            header: [
                0x4D, 0x56, 0x20, 0x2D, 0x20, 0x53, 0x4E, 0x41, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14,
                0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x0C, 0x8D, 0xC0, 0x00, 0x3F, 0x28, 0x2E,
                0x8E, 0x26, 0x00, 0x19, 0x1E, 0x00, 0x07, 0x00, 0x00, 0x30, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0xFF, 0x1E, 0x00, 0x82, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x3F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x02, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x32, 0x00, 0x08, 0x02, 0x00, 0x04, 0x00, 0x01, 0x00, 0x02, 0x20, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00,
            ],
            memory: SnapshotMemory::default_128(),
            chunks: Vec::new(),
            memory_already_written: bitsets::DenseBitSet::with_capacity_and_state(PAGE_SIZE * 8, 0),
            debug: false,
        }
    }
}

#[allow(missing_docs)]
#[allow(unused)]
impl Snapshot {
    pub fn log<S: std::fmt::Display>(&self, msg: S) {
        if self.debug {
            println!("> {}", msg);
        }
    }

    pub fn load<P: AsRef<Path>>(filename: P) -> Self {
        let mut sna = Self::default();
        let filename = filename.as_ref();

        // Read the full content of the file
        let mut file_content = {
            let mut f = File::open(filename).expect("file not found");
            let mut content = Vec::new();
            f.read_to_end(&mut content);
            content
        };
        // Copy the header
        sna.header
            .copy_from_slice(file_content.drain(0..0x100).as_slice());
        let memory_dump_size = sna.memory_size_header() as usize;
        let version = sna.version_header();

        dbg!(memory_dump_size * 1024);
        dbg!(file_content.len());

        assert!(memory_dump_size * 1024 <= file_content.len());
        sna.memory = SnapshotMemory::new(file_content.drain(0..memory_dump_size * 1024).as_slice());

        if version == 3 {
            while let Some(chunk) = Self::read_chunk(&mut file_content, &mut sna) {
                sna.chunks.push(chunk);
            }
        }

        eprintln!("{} chunks", sna.nb_chunks());

        // TODO manage chuncks
        sna
    }

    /// Count the number of kilobytes that are within chunks
    fn compute_memory_size_in_chunks(&self) -> u16 {
        let nb_pages = self.chunks.iter().filter(|c| c.is_memory_chunk()).count() as u16;
        nb_pages * 64
    }

    pub fn memory_size_header(&self) -> u16 {
        u16::from(self.header[0x6b]) + 256 * u16::from(self.header[0x6c])
    }

    fn set_memory_size_header(&mut self, size: u16) {
        self.header[0x6b] = (size % 256) as _;
        self.header[0x6c] = (size / 256) as _;
    }

    pub fn version_header(&self) -> u8 {
        self.header[0x10]
    }

    /// Create a new snapshot that contains only information understandable
    /// by the required version
    /// TODO return an error in case of failure instead of panicing
    pub fn fix_version(&self, version: SnapshotVersion) -> Self {
        // Clone the snapshot in order to patch it
        let mut cloned = self.clone();

        // Delete un-needed data
        match version {
            SnapshotVersion::V1 => {
                for idx in 0x6d..=0xff {
                    cloned.header[idx] = 0;
                }
            }
            SnapshotVersion::V2 => {
                for idx in 0x75..=0xff {
                    cloned.header[idx] = 0;
                }
            }
            SnapshotVersion::V3 => {}
        };

        // Write the version number
        cloned.header[0x10] = version as u8;

        // We have to modify the memory coding and remove the chunks
        if !cloned.chunks.is_empty() && version != SnapshotVersion::V3 {
            let memory = self.memory_dump();
            let memory_size = memory.len() / 1024;
            if memory_size > 128 {
                panic!("V1 or V2 snapshots cannot code more than 128kb of memory");
            }
            if memory_size != 0 && memory_size != 64 && memory_size != 128 {
                panic!("Memory of {}kb", memory_size);
            }

            // specify the right memory size ...
            cloned.set_memory_size_header(memory_size as _);

            // ... and replace it by the expected content
            cloned.memory = SnapshotMemory::new(&memory);

            // remove all the chunks
            cloned.chunks.clear();
            assert_eq!(cloned.chunks.len(), 0);
        }

        // TODO add a case to remove main memory in V3 snapshots in order
        // to crunch it and reduce sna size

        cloned
    }

    /// Read a chunk if available
    fn read_chunk(file_content: &mut Vec<u8>, _sna: &mut Self) -> Option<SnapshotChunk> {
        if file_content.len() < 4 {
            return None;
        }
        let code = file_content.drain(0..4).as_slice().to_vec();
        let data_length = file_content.drain(0..4).as_slice().to_vec();

        eprintln!("{:?} / {:?}", std::str::from_utf8(&code), data_length);

        let data_length = {
            let mut count = 0;
            for i in 0..4 {
                count = 256 * count + data_length[3 - i] as usize;
            }
            count
        };

        let content = file_content.drain(0..data_length).as_slice().to_vec();

        // Generate the 4 size array
        let code = {
            let mut new_code = [0; 4];
            new_code.copy_from_slice(&code);
            new_code
        };
        let chunk = match code {
            [0x4d, 0x45, 0x4d, _] => MemoryChunk::from(code, content).into(), /*
            ['B', 'R', 'K', 'S'] => BreakpointChunk::from(content),
            ['D', 'S', 'C', _] => InsertedDiscChunk::from(code, content)
            ['C', 'P', 'C', '+'] => CPCPlusChunk::from(content)
             */
            _ => UnknownChunk::from(code, content).into(),
        };

        Some(chunk)
    }

    pub fn nb_chunks(&self) -> usize {
        self.chunks.len()
    }

    /// Save the snapshot V3 on disc
    #[deprecated]
    pub fn save_sna(&self, fname: &str) -> Result<(), std::io::Error> {
        self.save(fname, SnapshotVersion::V2)
    }

    pub fn save(&self, fname: &str, version: SnapshotVersion) -> Result<(), std::io::Error> {
        let mut buffer = File::create(fname)?;
        self.write(&mut buffer, version)
    }

    pub fn write(&self, buffer: &mut File, version: SnapshotVersion) -> Result<(), std::io::Error> {
        // Convert the snapshot to ensure header is correct
        let sna = self.fix_version(version);

        // Write header
        buffer.write_all(&sna.header)?;

        // Write main memory if any
        if sna.memory_size_header() > 0 {
            assert_eq!(
                sna.memory.memory().len(),
                sna.memory_size_header() as usize * 1024
            );
            buffer.write_all(&sna.memory.memory())?;
        }

        // Write chunks if any
        for chunck in &self.chunks {
            buffer.write_all(chunck.code())?;
            buffer.write_all(&chunck.size_as_array())?;
            buffer.write_all(chunck.data())?;
        }

        Ok(())
    }

    /// Returns all the memory of the snapshot in a linear way by mixing both the hardcoded memory of the snapshot and the memory of chunks
    pub fn memory_dump(&self) -> Vec<u8> {
        // by default, the memory i already coded
        let mut memory = self.memory;

        let mut max_memory = self.memory_size_header() as usize * 1024;

        // but it can be patched by chunks
        for chunk in &self.chunks {
            if let Some(memory_chunk) = chunk.memory_chunk() {
                let memory_chunk = dbg!(memory_chunk);
                let address = memory_chunk.abstract_address();
                let content = memory_chunk.uncrunched_memory();
                max_memory = address + 64 * 1024;

                if memory.len() < max_memory {
                    memory = memory.increased_size();
                }
                memory.memory_mut()[address..max_memory].copy_from_slice(&content); // TODO treat the case where extra memory is used as `memroy` may need to be extended
            }
        }
        memory.memory()[..max_memory].to_vec()
    }

    /// Returns the memory that is hardcoded in the snapshot
    pub fn memory_block(&self) -> &SnapshotMemory {
        &self.memory
    }

    /// Add the content of a file at the required position
    pub fn add_file(&mut self, fname: &str, address: usize) -> Result<(), SnapshotError> {
        let f = File::open(fname).unwrap();
        let data: Vec<u8> = f.bytes().map(Result::unwrap).collect();
        let size = data.len();

        self.log(format!(
            "Add {} in 0x{:x} (0x{:x} bytes)",
            fname, address, size
        ));
        self.add_data(&data, address)
    }

    /// Add the memory content at the required posiiton
    ///
    /// ```
    /// use cpclib::sna::Snapshot;
    ///
    /// let mut sna = Snapshot::default();
    /// let data = vec![0,2,3,5];
    /// sna.add_data(&data, 0x4000);
    /// ```
    pub fn add_data(&mut self, data: &[u8], address: usize) -> Result<(), SnapshotError> {
        if address + data.len() > 0x10000 * 2 {
            Err(SnapshotError::NotEnougSpaceAvailable)
        } else {
            if address < 0x10000 && (address + data.len()) >= 0x10000 {
                eprintln!("[Warning] Start of file is in main memory (0x{:x}) and  end of file is in extra banks (0x{:x}).", address, (address + data.len()));
            }
            // TODO add warning when writting in other banks

            for (idx, byte) in data.iter().enumerate() {
                let current_pos = address + idx;
                if self.memory_already_written.test(current_pos) {
                    eprintln!("[WARNING] Replace memory in 0x{:x}", current_pos);
                }
                self.memory.memory_mut()[current_pos] = *byte;
                self.memory_already_written.set(current_pos);
            }

            Ok(())
        }
    }

    /// Change a memory value. Panic if memory size is not appropriate
    /// TODO should enlarge memory if needed or write un Chunks
    pub fn set_memory(&mut self, address: u32, value: u8) {
        assert!(address < 0x20000);
        let address = address as usize;

        self.memory.memory_mut()[address] = value;
    }

    /// Change the value of a flag
    pub fn set_value(&mut self, flag: SnapshotFlag, value: u16) -> Result<(), SnapshotError> {
        let offset = flag.offset();

        match flag.elem_size() {
            1 => {
                if value > 255 {
                    Err(SnapshotError::InvalidValue)
                } else {
                    self.header[offset] = value as u8;
                    Ok(())
                }
            }

            2 => {
                self.header[offset + 0] = (value % 256) as u8;
                self.header[offset + 1] = (value / 256) as u8;
                Ok(())
            }
            _ => panic!("Unable to handle size != 1 or 2"),
        }
    }

    pub fn get_value(&self, flag: &SnapshotFlag) -> FlagValue {
        if flag.indice().is_some() {
            // Here we treate the case where we read only one value
            let offset = flag.offset();
            match flag.elem_size() {
                1 => FlagValue::Byte(self.header[offset]),
                2 => FlagValue::Word(
                    u16::from(self.header[offset + 1]) * 256 + u16::from(self.header[offset]),
                ),
                _ => panic!(),
            }
        } else {
            // Here we treat the case where we read an array
            let mut vals: Vec<FlagValue> = Vec::new();
            for idx in 0..flag.nb_elems() {
                // By construction, >1
                let mut flag2 = *flag;
                flag2.set_indice(idx).unwrap();
                vals.push(self.get_value(&flag2));
            }
            FlagValue::Array(vals)
        }
    }

    pub fn print_info(&self) {
        for flag in SnapshotFlag::enumerate().iter() {
            println!("{:?} => {}", &flag, &self.get_value(flag));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SnapshotMemory;

    #[test]
    fn test_memory() {
        assert!(SnapshotMemory::default().is_empty());
        assert!(SnapshotMemory::default_64().is_64k());
        assert!(SnapshotMemory::default_128().is_128k());

        assert_eq!(SnapshotMemory::default().len(), 0);
        assert_eq!(SnapshotMemory::default_64().len(), 64 * 1024);
        assert_eq!(SnapshotMemory::default_128().len(), 128 * 1024);

        assert_eq!(SnapshotMemory::default().memory().len(), 0);
        assert_eq!(SnapshotMemory::default_64().memory().len(), 64 * 1024);
        assert_eq!(SnapshotMemory::default_128().memory().len(), 128 * 1024);
    }

    #[test]
    fn test_increase() {
        let memory = SnapshotMemory::default();
        assert!(memory.is_empty());

        let memory = memory.increased_size();
        assert!(memory.is_64k());
        assert_eq!(memory.memory().len(), 64 * 1024);

        let memory = memory.increased_size();
        assert!(memory.is_128k());
        assert_eq!(memory.memory().len(), 128 * 1024);
    }

    #[test]
    #[should_panic]
    fn test_increase2() {
        SnapshotMemory::default_128().increased_size();
    }

}
