use std::fmt::Display;
use std::ops::Deref;

use cpclib_common::riff::{RiffChunk, RiffCode, RiffLen};
use delegate::delegate;
use strum::{EnumString, IntoStaticStr};

#[nutype::nutype(
    validate(len_char_max = 127),
    default = "",
    derive(AsRef, Default, Debug, Clone, PartialEq)
)]
pub struct String127(String);

#[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, Ord, IntoStaticStr, EnumString)]
#[strum(serialize_all = "UPPERCASE", ascii_case_insensitive)]
pub enum RemuBreakPointType {
    #[strum(serialize = "EXEC")]
    #[strum(serialize = "EXECUTE")]
    #[strum(to_string = "EXEC")]
    Exec,
    IO,
    #[strum(serialize = "MEM")]
    #[strum(serialize = "MEMORY")]
    #[strum(to_string = "MEM")]
    Mem
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, Ord, IntoStaticStr, EnumString)]
#[strum(serialize_all = "UPPERCASE", ascii_case_insensitive)]
pub enum RemuBreakPointAccessMode {
    #[strum(serialize = "R")]
    #[strum(serialize = "READ")]
    Read,
    #[strum(serialize = "READWRITE")]
    #[strum(serialize = "RW")]
    #[strum(to_string = "RW")]
    ReadWrite,
    #[strum(serialize = "W")]
    #[strum(serialize = "WRITE")]
    Write
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, Ord, IntoStaticStr, EnumString)]
#[strum(serialize_all = "UPPERCASE", ascii_case_insensitive)]
pub enum RemuBreakPointRunMode {
    #[strum(serialize = "STOP")]
    #[strum(serialize = "STOPPER")]
    #[strum(to_string = "STOP")]
    Stop,
    #[strum(serialize = "WATCH")]
    #[strum(serialize = "WATCHER")]
    #[strum(to_string = "WATCH")]
    Watch
}

#[derive(Debug, PartialEq, Clone)]
pub struct AdvancedRemuBreakPoint {
    pub brk_type: RemuBreakPointType,
    pub access_mode: RemuBreakPointAccessMode,
    pub run_mode: RemuBreakPointRunMode,
    pub addr: u16,
    pub mask: u16,
    pub size: u16,
    pub value: u8,
    pub val_mask: u8,
    pub condition: Option<String127>,
    pub name: Option<String127>,
    pub step: Option<u16>
}

impl Default for AdvancedRemuBreakPoint {
    fn default() -> Self {
        Self {
            brk_type: RemuBreakPointType::Exec,
            access_mode: RemuBreakPointAccessMode::ReadWrite,
            run_mode: RemuBreakPointRunMode::Stop,
            addr: 0,
            mask: 0xFFFF,
            size: 1,
            value: 0,
            val_mask: 0,
            condition: None,
            name: None,
            step: None
        }
    }
}

impl Display for AdvancedRemuBreakPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let brk = self;
        write!(
            f,
            "{},{},{},addr={},mask={},size={},value={},valmask={},name={},condition={},step={}",
            <&RemuBreakPointType as Into<&'static str>>::into(&brk.brk_type),
            <&RemuBreakPointAccessMode as Into<&'static str>>::into(&brk.access_mode),
            <&RemuBreakPointRunMode as Into<&'static str>>::into(&brk.run_mode),
            brk.addr,
            brk.mask,
            brk.size,
            brk.value,
            brk.val_mask,
            brk.name
                .as_ref()
                .map(|s| format!("\"{}\"", s.as_ref()))
                .unwrap_or("imported".to_owned()),
            brk.condition
                .as_ref()
                .map(|s| format!("\"{}\"", s.as_ref()))
                .unwrap_or("".to_owned()),
            brk.step
                .as_ref()
                .map(|s| format!("{s}"))
                .unwrap_or("".to_owned()),
        )
    }
}

pub enum RemuBreakPoint {
    Memory(u16, u8),
    Rom(u16, u8),
    Advanced(AdvancedRemuBreakPoint)
}

impl From<AdvancedRemuBreakPoint> for RemuBreakPoint {
    fn from(value: AdvancedRemuBreakPoint) -> Self {
        RemuBreakPoint::Advanced(value)
    }
}

pub enum RemuEntry {
    BreakPoint(RemuBreakPoint),
    // (name, address, bank)
    Label(String, u16, u8),
    // (name, value)
    Alias(String, u16)
}

impl<T: Into<RemuBreakPoint>> From<T> for RemuEntry {
    fn from(value: T) -> Self {
        let brk = value.into();
        Self::BreakPoint(brk)
    }
}

impl Display for RemuEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemuEntry::BreakPoint(brk) => {
                match brk {
                    RemuBreakPoint::Memory(address, bank) => {
                        write!(f, "brk {address} {bank};")
                    },
                    RemuBreakPoint::Rom(address, bank) => {
                        write!(f, "rombrk {address} {bank};")
                    },
                    RemuBreakPoint::Advanced(brk) => {
                        write!(f, "acebreak {brk};")
                    }
                }
            },
            RemuEntry::Label(name, address, bank) => write!(f, "label {name} {address} {bank};"),
            RemuEntry::Alias(name, value) => write!(f, "alias {name} {value};")
        }
    }
}

impl RemuEntry {
    pub fn new_breakpoint(address: u16, bank: u8) -> Self {
        Self::BreakPoint(RemuBreakPoint::Memory(address, bank))
    }

    pub fn new_rom_breakpoint(address: u16, bank: u8) -> Self {
        Self::BreakPoint(RemuBreakPoint::Rom(address, bank))
    }

    pub fn new_remu_breakpoint(brk: AdvancedRemuBreakPoint) -> Self {
        Self::BreakPoint(RemuBreakPoint::Advanced(brk))
    }

    pub fn new_label(name: String, address: u16, bank: u8) -> Self {
        Self::Label(name, address, bank)
    }

    pub fn new_alias(name: String, value: u16) -> Self {
        Self::Alias(name, value)
    }
}

#[derive(Clone, Debug)]
pub struct RemuChunk {
    riff: RiffChunk
}

impl Deref for RemuChunk {
    type Target = RiffChunk;

    fn deref(&self) -> &Self::Target {
        &self.riff
    }
}

impl RemuChunk {
    const CODE: RiffCode = RiffCode::new([b'R', b'E', b'M', b'U']);

    delegate! {
        to self.riff {
            pub fn code(&self) -> &RiffCode;
            pub fn len(&self) -> &RiffLen;
            pub fn data(&self) -> &[u8];
            fn add_bytes(&mut self, data: &[u8]);
        }
    }

    pub fn empty() -> Self {
        Self::from(Self::CODE, Default::default())
    }

    pub fn from<C: Into<RiffCode>>(code: C, content: Vec<u8>) -> Self {
        let code = code.into();
        assert_eq!(code, Self::CODE);

        Self {
            riff: RiffChunk::new(code, content)
        }
    }

    pub fn add_entry(&mut self, entry: &RemuEntry) {
        self.add_bytes(entry.to_string().as_bytes());
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::RemuBreakPointAccessMode;

    #[test]
    fn test_string_enum() {
        let repr: &'static str = RemuBreakPointAccessMode::ReadWrite.into();
        assert_eq!("RW", repr);

        let repr: &'static str = RemuBreakPointAccessMode::Write.into();
        assert_eq!("WRITE", repr);

        assert_eq!(
            RemuBreakPointAccessMode::from_str("R").unwrap(),
            RemuBreakPointAccessMode::Read
        );

        assert_eq!(
            RemuBreakPointAccessMode::from_str("r").unwrap(),
            RemuBreakPointAccessMode::Read
        );

        assert_eq!(
            RemuBreakPointAccessMode::from_str("WRITE").unwrap(),
            RemuBreakPointAccessMode::Write
        );

        assert_eq!(
            RemuBreakPointAccessMode::from_str("WRite").unwrap(),
            RemuBreakPointAccessMode::Write
        );
    }
}
