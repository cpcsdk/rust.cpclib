extern crate cpc;
use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::str::FromStr;
use std::path::Path;
use std::io::BufReader;
use std::fmt;

extern crate clap;
use clap::{Arg, App, SubCommand};

pub mod built_info {
include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

extern crate built;
extern crate time;
extern crate semver;



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


/**
 * Convert a string to its unsigned 32 bits representation (to access to extra memory)
 */
fn string_to_nb(source: &str) -> u32 {
    let error =format!("Unable to read the value: {}", source);
    if source.starts_with("0x") {
        u32::from_str_radix(&source[2..], 16).expect(&error)
    }
    else {
        source.parse::<u32>().expect(&error)
    }
}


/// Encode a flag of the snaphot
#[derive(PartialEq, Debug, Copy, Clone)]
enum SnapshotFlag {
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

impl SnapshotFlag {


    


    pub fn enumerate() -> [SnapshotFlag;67] {
        use SnapshotFlag::*;

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
            INT_REQ]
    }

    /// Return the location in the header for the flag (and its potential index)
    pub fn offset(&self) -> usize {
        use SnapshotFlag::*;
        match self {
            &GA_PAL(ref idx) |
            &CRTC_REG(ref idx) |
            &PSG_REG(ref idx) |
            &GA_MULTIMODE(ref idx)
                => self.base() + idx.unwrap_or(0) * self.elem_size(),
            _
                => self.base()
        }
    }

    pub fn indice(&self) -> Option<usize> {
        use SnapshotFlag::*;

        match self {
            &GA_PAL(ref idx) |
            &CRTC_REG(ref idx) |
            &PSG_REG(ref idx) |
            &GA_MULTIMODE(ref idx)
            => idx.clone(),
            _ => Some(0) // For standard stuff indice is considered to be 0
        }
    }

    pub fn set_indice(&mut self, indice: usize) -> Result<(), SnapshotError>{
        use SnapshotFlag::*;
        match self {
            &mut GA_PAL(ref mut idx) |
                &mut CRTC_REG(ref mut idx) |
                &mut PSG_REG(ref mut idx) |
                &mut GA_MULTIMODE(ref mut idx)
                => {*idx = Some(indice); Ok(())},
            _ => Err(SnapshotError::InvalidIndex)
        }
    }


    /// Return the header base position that corresponds to the flag
    pub fn base(&self) -> usize {
        use SnapshotFlag::*;
        match self {
            &Z80_AF => 0x11,
            &Z80_F => 0x11,
            &Z80_A => 0x12,
            &Z80_BC => 0x13,
            &Z80_C => 0x13,
            &Z80_B => 0x14,
            &Z80_DE => 0x15,
            &Z80_E => 0x15,
            &Z80_D => 0x16,
            &Z80_HL => 0x17,
            &Z80_L => 0x17,
            &Z80_H => 0x18,
            &Z80_R => 0x19,
            &Z80_I => 0x1a,
            &Z80_IFF0 => 0x1b,
            &Z80_IFF1 => 0x1c,
            &Z80_IX => 0x1d,
            &Z80_IXL => 0x1d,
            &Z80_IXH => 0x1e,
            &Z80_IY => 0x1f,
            &Z80_IYL => 0x1f,
            &Z80_IYH => 0x20,
            &Z80_SP => 0x21,
            &Z80_PC => 0x23,
            &Z80_IM => 0x25,
            &Z80_AFX => 0x26,
            &Z80_FX => 0x26,
            &Z80_AX => 0x27,
            &Z80_BCX => 0x28,
            &Z80_CX => 0x28,
            &Z80_BX => 0x29,
            &Z80_DEX => 0x2a,
            &Z80_EX => 0x2a,
            &Z80_DX => 0x2b,
            &Z80_HLX => 0x2c,
            &Z80_LX => 0x2c,
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
        use SnapshotFlag::*;
        match self {
            &GA_PAL(_) => 17,
            &CRTC_REG(_) => 18,
            &PSG_REG(_) => 16,
            &GA_MULTIMODE(_) => 6,
            _ => 1
        }
    }

    /// Return the size of one unique element
    pub fn elem_size(&self) -> usize {
        use SnapshotFlag::*;
        match self {
            &Z80_AF|
                &Z80_BC|
                &Z80_DE|
                &Z80_HL|
                &Z80_IX|
                &Z80_IY|
                &Z80_SP|
                &Z80_PC|
                &Z80_AFX|
                &Z80_BCX|
                &Z80_DEX|
                &Z80_HLX|
                &CRTC_STATE => 2,

                &Z80_F|
                    &Z80_A|
                    &Z80_C|
                    &Z80_B|
                    &Z80_E|
                    &Z80_D|
                    &Z80_L|
                    &Z80_H|
                    &Z80_R|
                    &Z80_I|
                    &Z80_IFF0|
                    &Z80_IFF1|
                    &Z80_IXL|
                    &Z80_IXH|
                    &Z80_IYL|
                    &Z80_IYH|
                    &Z80_IM|
                    &Z80_FX|
                    &Z80_AX|
                    &Z80_CX|
                    &Z80_BX|
                    &Z80_EX|
                    &Z80_DX|
                    &Z80_LX|
                    &Z80_HX|
                    &GA_PEN|
                    &GA_ROMCFG|
                    &GA_RAMCFG|
                    &CRTC_SEL|
                    &ROM_UP|
                    &PPI_A|
                    &PPI_B|
                    &PPI_C|
                    &PPI_CTL|
                    &PSG_SEL|
                    &CPC_TYPE|
                    &GA_VSC|
                    &GA_ISC|
                    &INT_REQ |
                    &INT_NUM |
                    &FDD_MOTOR |
                    &FDD_TRACK |
                    &PRNT_DATA |
                    &CRTC_TYPE |
                    &CRTC_HCC |
                    &CRTC_CLC |
                    &CRTC_RLC |
                    &CRTC_VAC |
                    &CRTC_VSWC |
                    &CRTC_HSWC => 1,
                    

                &GA_PAL(_) => 1,
                &CRTC_REG(_) => 1,
                &PSG_REG(_) => 1,
                &GA_MULTIMODE(_) => 1,
        }
    }


    pub fn comment(&self) -> &str{
        use SnapshotFlag::*;

        match self {
            &Z80_AF => "\t\tZ80 register AF",
            &Z80_F => "\t\tZ80 register F",
            &Z80_A => "\t\tZ80 register A",
            &Z80_BC => "\t\tZ80 register BC",
            &Z80_C => "\t\tZ80 register C",
            &Z80_B => "\t\tZ80 register B",
            &Z80_DE => "\t\tZ80 register DE",
            &Z80_E => "\t\tZ80 register E",
            &Z80_D => "\t\tZ80 register D",
            &Z80_HL => "\t\tZ80 register HL",
            &Z80_L => "\t\tZ80 register L",
            &Z80_H => "\t\tZ80 register H",
            &Z80_R => "\t\tZ80 register R",
            &Z80_I => "\t\tZ80 register I",
            &Z80_IFF0 => "\tZ80 interrupt flip-flop IFF0",
            &Z80_IFF1 => "\tZ80 interrupt flip-flop IFF1",
            &Z80_IX => "\t\tZ80 register IX",
            &Z80_IXL => "\t\tZ80 register IX (low)",
            &Z80_IXH => "\t\tZ80 register IX (high)",
            &Z80_IY => "\t\tZ80 register IY",
            &Z80_IYL => "\t\tZ80 register IY (low)",
            &Z80_IYH => "\t\tZ80 register IY (high)",
            &Z80_SP => "\t\tZ80 register SP",
            &Z80_PC => "\t\tZ80 register PC",
            &Z80_IM => "\t\tZ80 interrupt mode (0,1,2)",
            &Z80_AFX => "\t\tZ80 register AF'",
            &Z80_FX => "\t\tZ80 register F'",
            &Z80_AX => "\t\tZ80 register A'",
            &Z80_BCX => "\t\tZ80 register BC'",
            &Z80_CX => "\t\tZ80 register C'",
            &Z80_BX => "\t\tZ80 register B'",
            &Z80_DEX => "\t\tZ80 register DE'",
            &Z80_EX => "\t\tZ80 register E'",
            &Z80_DX => "\t\tZ80 register D'",
            &Z80_HLX => "\t\tZ80 register HL'",
            &Z80_LX => "\t\tZ80 register L'",
            &Z80_HX => "\t\tZ80 register H'",
            &GA_PEN => "\t\tGA: index of selected pen",
            &GA_PAL(_) => "\t\tGA: current palette (0..16)",
            &GA_ROMCFG => "\tGA: multi configuration",
            &GA_RAMCFG => "\tCurrent RAM configuration",
            &CRTC_SEL => "\tCRTC: index of selected register",
            &CRTC_REG(_) => "\tCRTC: register data (0..17)",
            &ROM_UP => "\t\tCurrent ROM selection",
            &PPI_A => "\t\tPPI: port A",
            &PPI_B => "\t\tPPI: port B",
            &PPI_C => "\t\tPPI: port C",
            &PPI_CTL => "\t\tPPI: control port",
            &PSG_SEL => "\t\tPSG: index of selected register",
            &PSG_REG(_) => "\t\tPSG: register data (0..15)",
            &CPC_TYPE => "\tCPC type: \n\t\t\t0 = CPC464\n\t\t\t1 = CPC664\n\t\t\t2 = CPC6128\n\t\t\t3 = unknown\n\t\t\t4 = 6128 Plus\n\t\t\t5 = 464 Plus\n\t\t\t6 = GX4000",
            &INT_NUM => "\tinterrupt number (0..5)",
            &GA_MULTIMODE(_) => "\t6 mode bytes (one for each halt)",
            &FDD_MOTOR => "\tFDD motor drive state (0=off, 1=on)",
            &FDD_TRACK => "\tFDD current physical track",
            &PRNT_DATA => "\tPrinter Data/Strobe Register",
            &CRTC_TYPE => "\tCRTC type:\n\t\t\t0 = HD6845S/UM6845\n\t\t\t1 = UM6845R\n\t\t\t2 = MC6845\n\t\t\t3 = 6845 in CPC+ ASIC\n\t\t\t4 = 6845 in Pre-ASIC",
            &CRTC_HCC => "\tCRTC horizontal character counter register",
            &CRTC_CLC => "\tCRTC character-line counter register",
            &CRTC_RLC => "\tCRTC raster-line counter register",
            &CRTC_VAC => "\tCRTC vertical total adjust counter register",
            &CRTC_VSWC => "\tCRTC horizontal sync width counter",
            &CRTC_HSWC => "\tCRTC vertical sync width counter",
            &CRTC_STATE => "\tCRTC state flags. \n\t\t\t0 if '1'/'0' VSYNC active/inactive\n\t\t\t1 if '1'/'0' HSYNC active/inactive\n\t\t\t2-7 reserved\n\t\t\t7 if '1'/'0' Vert Total Adjust active/inactive\n\t\t\t8-15 reserved (0)",
            &GA_VSC => "\t\tGA vsync delay counter",
            &GA_ISC => "\t\tGA interrupt scanline counter",
            &INT_REQ => "\t\tInterrupt request flag\n\t\t\t0=no interrupt requested\n\t\t\t1=interrupt requested",

        }
    }
}

impl FromStr for SnapshotFlag {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {

        let s = &s.to_uppercase();

        if s.contains(":") {

            let elems = s.split(":").collect::<Vec<_>>();
            let s = elems[0];
            let idx = match elems[1].parse::<usize>() {
                Ok(idx) => idx,
                Err(_) => return Err(String::from("Unable to parse index"))
            };

            let indexed_flag = match s {
                "GA_PAL" => SnapshotFlag::GA_PAL(Some(idx)),
                "CRTC_REG" => SnapshotFlag::CRTC_REG(Some(idx)),
                "PSG_REG" => SnapshotFlag::PSG_REG(Some(idx)),
                "GA_MULTIMODE" => SnapshotFlag::GA_MULTIMODE(Some(idx)),
                _ => {return Err(String::from("Unable to convert string to a flag"));}
            };

            if indexed_flag.indice().unwrap() < indexed_flag.nb_elems() {
                return Ok(indexed_flag);
            }
            else {
                return Err(format!("Wrong index size {:?}", indexed_flag));
            }
        }
        else {
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
                _ => Err(String::from("Unable to convert string to a flag"))

            }
        }
    }
}

#[derive(Debug)]
enum FlagValue {
    Byte(u8),
    Word(u16),
    Array(Vec<FlagValue>), // Restr$icted to Byte or Word

}

impl fmt::Display for FlagValue{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &FlagValue::Byte(ref val) => {
                write!(f, "0x{:x}", val)
            },
            &FlagValue::Word(ref val) => {
                write!(f, "0x{:x}", val)
            }
            &FlagValue::Array(ref array) => {
                write!(f, "[")
                    .and_then(|_x|{
                        write!(f, "{:?}",
                        &array
                            .iter()
                            .map(|b|{format!("{}", b)})
                            .collect::<Vec<_>>())
                    })
                    .and_then(|_x|{write!(f, "]")})
            }
        }
    }
}


#[derive(Debug)]
pub enum SnapshotError {
    FileError,
    NotEnougSpaceAvailable,
    InvalidValue,
    FlagDoesNotExists,
    InvalidIndex
}

struct SnapshotChunk {
    code: [u8;4],
    data: Vec<u8>   
}

impl SnapshotChunk {
    pub fn code(&self) -> &[u8;4] {
            & (self.code)
    }

    pub fn size(&self) -> u32 {
        return 0;
    }

    pub fn size_as_array(&self) -> [u8;4] {
        let mut size = self.size();
        let mut array = [0, 0, 0, 0];

        for i in 0..4 {
            array[i] = (size % 256) as u8;
            size = size / 256;
        }

        array
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

const PAGE_SIZE:usize = 0x4000;
const HEADER_SIZE:usize = 256;

struct Snapshot {
    header: [u8; HEADER_SIZE],
    memory: [u8; PAGE_SIZE*8],
    chuncks: Vec<SnapshotChunk>,

    // nothing to do with the snapshot. Should be moved elsewhere
    pub debug: bool
}


impl Default for Snapshot {
    fn default() -> Snapshot {
        Snapshot{
            header: [
                0x4D, 0x56, 0x20, 0x2D, 0x20, 0x53, 0x4E, 0x41, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0xc0, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14,
                0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x0C,
                0x8D, 0xC0, 0x00, 0x3F, 0x28, 0x2E, 0x8E, 0x26, 0x00, 0x19, 0x1E, 0x00, 0x07, 0x00, 0x00, 0x30,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x1E, 0x00, 0x82, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x3F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x02, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x7F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x32, 0x00, 0x08, 0x02, 0x00, 0x04, 0x00,
                0x01, 0x00, 0x02, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                memory: [0; PAGE_SIZE*8],
                chuncks: Vec::new(),
                debug: false
        }
    }
}


impl Snapshot {

    pub fn log<S: std::fmt::Display>(&self, msg: S) {
        if self.debug {
            println!("> {}", msg);
        }
    }

    pub fn load(filename: &Path) -> Snapshot {
        let mut sna = Snapshot::default();

        let mut f = File::open(filename).expect("file not found");

        f.read_exact(&mut sna.header);
        f.read_exact(&mut sna.memory);

        // TODO manage chuncks
        sna
    }


    /// Save the snapshot V3 on disc
    pub fn save_sna(&self, fname:&str) -> Result<(), std::io::Error>{
        let mut buffer = File::create(fname)?;

        // Write header and main memory
        buffer.write(&self.header)?;
        buffer.write(&self.memory)?;

        // TODO add extra memory ?

        // Save the chucks
        for chunck in self.chuncks.iter() {
           buffer.write(chunck.code());
           buffer.write(&chunck.size_as_array());
           buffer.write(chunck.data());
        }

        //TODO add the chuncks
        Ok(())
    }

    /// Add the content of a file at the required position
    pub fn add_file(&mut self, fname:&str, address: usize) -> Result<(), SnapshotError>{
        let f = File::open(fname).unwrap();
        let data:Vec<u8> = f.bytes().map(|byte| byte.unwrap()).collect();
        let size = data.len();

        self.log(format!("Add {} in 0x{:x} (0x{:x} bytes)", fname, address, size));
        self.add_data(data, address)
    }

    /// Add the memory content at the required posiiton
    ///
    /// ```
    /// let mut sna = Snapshot::default();
    /// let data = vec![0,2,3,5];
    /// sna.add_data(data, 0x4000);
    pub fn add_data(&mut self, data:Vec<u8>, address: usize) -> Result<(), SnapshotError>{

        if address + data.len() > 0x10000*2 {
            Err(SnapshotError::NotEnougSpaceAvailable)
        }
        else {
            if address < 0x10000 && (address + data.len()) >= 0x10000 {
                eprintln!("[Warning] Start of file is in main memory (0x{:x}) and  end of file is in extra banks (0x{:x}).", address, (address + data.len()));
            }
            // TODO add warning when writting in other banks

            for (idx, byte) in data.iter().enumerate() {
                self.memory[address + idx] = byte.clone();
            }

            Ok(())
        }
    }

    /// Change a memory value
    pub fn set_memory(&mut self, address: u32, value: u8) {
        assert!(address < 0x20000);
        let address = address as usize;

        self.memory[address] = value;
    }

    /// Change the value of a flag
    pub fn set_value(&mut self, flag: SnapshotFlag, value: u16) -> Result<(), SnapshotError>{
        let offset = flag.offset();

        match flag.elem_size() {
            1 => {
                if value > 255 {
                    Err(SnapshotError::InvalidValue)
                }
                else {
                    self.header[offset] = value as u8;
                    Ok(())
                }
            },

            2 => {
                self.header[offset + 0] = (value%256) as u8 ;
                self.header[offset + 1] = (value/256) as u8 ;
                Ok(())
            }
            _ => panic!("Unable to handle size != 1 or 2")
        }
    }

    pub fn get_value(&self, flag: &SnapshotFlag) -> FlagValue {

        if flag.indice().is_some() {
            // Here we treate the case where we read only one value
            let offset = flag.offset();
            match flag.elem_size() {
                1 => {
                    return FlagValue::Byte(self.header[offset]);
                },
                2 => {
                    return FlagValue::Word(self.header[offset+1] as u16 *256 + self.header[offset] as u16)
                },
                _ => panic!()
            };
        }

        else {
            // Here we treat the case where we read an array
            let mut vals:Vec<FlagValue> = Vec::new();
            for idx in 0..flag.nb_elems() { // By construction, >1
                let mut flag2 = flag.clone();
                flag2.set_indice(idx);
                vals.push(self.get_value(&flag2));
            }
            return FlagValue::Array(vals);
        }
    }

    pub fn print_info(&self) {
        for flag in SnapshotFlag::enumerate().into_iter() {
            println!("{:?} => {}", &flag, &self.get_value(flag));
        }
    }


}

fn main() {
    eprintln!("[WARNING] This is still a draft version that implement still few functionnalities");

    let desc_before = format!("Profile {} compiled: {}", built_info::PROFILE, built_info::BUILT_TIME_UTC);
    let matches = App::new("createSnapshot")
                          .version(built_info::PKG_VERSION)
                          .author("Krusty/Benediction")
                          .about("Amstrad CPC snapshot manipulation")
                          .before_help(&desc_before[..])
                          .after_help("This tool tries to be similar than Ramlaid's one")
                          .arg(Arg::with_name("info")
                               .help("Display informations on the loaded snapshot")
                               .multiple(false)
                               .long("info")
                               .requires("inSnapshot")
                           )
                          .arg(Arg::with_name("debug")
                            .help("Display debugging information while manipulating the snapshot")
                            .long("debug")
                          )
                          .arg(Arg::with_name("OUTPUT")
                               .help("Sets the output file to generate")
                               .conflicts_with("flags")
                               .conflicts_with("info")
                               .conflicts_with("getToken")
                               .last(true)
                               .required(true))
                          .arg(Arg::with_name("inSnapshot")
                               .takes_value(true)
                               .short("i")
                               .long("inSnapshot")
                               .value_name("INFILE")
                               .multiple(false)
                               .number_of_values(1)
                               .help("Load <INFILE> snapshot file")
                               )
                          .arg(Arg::with_name("load")
                               .takes_value(true)
                               .short("l")
                               .long("load")
                               .multiple(true)
                               .number_of_values(2)
                               .help("Specify a file to include. -l fname address"))
                          .arg(Arg::with_name("getToken")
                               .takes_value(true)
                               .short("g")
                               .long("getToken")
                               .multiple(true)
                               .number_of_values(1)
                               .help("Display the value of a snapshot token")
                               .requires("inSnapshot")
                           )
                          .arg(Arg::with_name("setToken")
                               .takes_value(true)
                               .short("s")
                               .long("setToken")
                               .multiple(true)
                               .number_of_values(2)
                               .help("Set snapshot token <$1> to value <$2>\nUse <$1>:<val> to set array value\n\t\tex '-s CRTC_REG:6 20' : Set CRTC register 6 to 20"))
                          .arg(Arg::with_name("putData")
                               .takes_value(true)
                               .short("p")
                               .long("putData")
                               .multiple(true)
                               .number_of_values(2)
                               .help("Put <$2> byte at <$1> address in snapshot memory")

                            )
                          .arg(Arg::with_name("flags")
                                .help("List the flags and exit")
                               .long("flags"))
                          .get_matches();




    // Display all tokens
    if matches.is_present("flags") {
        for flag in SnapshotFlag::enumerate().into_iter() {
            println!("{:?} / {:?} bytes.{}", flag, flag.elem_size(), flag.comment());
        }
        return;
    }


    let mut sna = if matches.is_present("inSnapshot"){
        let fname = matches.value_of("inSnapshot").unwrap();
        let path = Path::new(&fname);
        Snapshot::load(path)
    }
    else {
        Snapshot::default()
    };

    // Activate the debug mode
    sna.debug = matches.is_present("debug");



    if matches.is_present("info") {
        sna.print_info();
        return;
    }


    // Manage the files insertion
    if matches.is_present("load") {
        let loads = matches.values_of("load").unwrap().collect::<Vec<_>>();
        for i in 0..(loads.len()/2) {
            let fname = loads[i*2+0];
            let place = loads[i*2+1];

            let address = {
                if place.starts_with("0x") {
                    u32::from_str_radix(&place[2..], 16).unwrap()
                }
                else if place.starts_with("0") {
                    u32::from_str_radix(&place[1..],8).unwrap()
                }
                else {
                    place.parse::<u32>().unwrap()
                }
            };
            sna.add_file(fname, address as usize).expect("Unable to add file");
        }
    }



    // Patch memory
    if matches.is_present("putData") {
        let data = matches.values_of("putData").unwrap().collect::<Vec<_>>();

        for i in 0..(data.len()/2) {
            let address = string_to_nb(data[i*2+0]);
            let value = string_to_nb(data[i*2+1]);
            assert!(value < 0x100);

            sna.set_memory(address, value as u8);
        }
    }

    // Read the tokens
    if matches.is_present("getToken") {
        for token in matches.values_of("getToken").unwrap() {
            let mut token = SnapshotFlag::from_str(token).unwrap();
            println!("{:?} => {}", &token, sna.get_value(&token));
        }
        return;
    }

    // Set the tokens
    if matches.is_present("setToken") {
        let loads = matches.values_of("setToken").unwrap().collect::<Vec<_>>();
        for i in 0..(loads.len()/2) {

            // Read the parameters from the command line
            let token = loads[i*2+0];
            let (token, _index) = if token.contains(":") {
                let elems = token.split(":").collect::<Vec<_>>();
                (elems[0], Some(elems[1].parse::<usize>().expect("Unable to read indice")))
            }
            else {
                (token, None)
            };
            let value = {
                let source =loads[i*2+1];
                string_to_nb(source)
            };

            // Get the token
            let token = SnapshotFlag::from_str(token).unwrap();
            sna.set_value(token, value as u16);


            sna.log(format!("Token {:?} set at value {} (0x{:x})", token, value, value));
        }
    }


    let fname = matches.value_of("OUTPUT").unwrap();
    sna.save_sna(&fname);

}
