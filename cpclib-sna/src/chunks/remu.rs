use std::fmt::Display;


use delegate::delegate;

use crate::{Code, SnapshotChunkData};

pub enum RemuEntry {
    // (address, bank)
    BreakPoint(u16, u8),
    // (address, bank)
    RomBreakPoint(u16, u8),
    // (name, address, bank)
    Label(String, u16, u8),
    // (name, value)
    Alias(String, u16)
}

impl Display for RemuEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemuEntry::BreakPoint(address, bank) => write!(f, "brk {address} {bank};"),
            RemuEntry::RomBreakPoint(address, bank) => write!(f, "rombrk {address} {bank};"),
            RemuEntry::Label(name, address, bank) => write!(f, "label {name} {address} {bank};"),
            RemuEntry::Alias(name, value) => write!(f, "alias {name} {value};")
        }
    }
}

impl RemuEntry {
    pub fn new_breakpoint(address: u16, bank: u8) -> Self {
        Self::BreakPoint(address, bank)
    }

    pub fn new_rom_breakpoint(address: u16, bank: u8) -> Self {
        Self::RomBreakPoint(address, bank)
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
    data: SnapshotChunkData
}

impl RemuChunk {
    const CODE: Code = Code([b'R', b'E', b'M', b'U']);

    delegate! {
        to self.data {
            pub fn code(&self) -> &Code;
            pub fn size(&self) -> usize;
            pub fn size_as_array(&self) -> [u8; 4];
            pub fn data(&self) -> &[u8];
            fn add_bytes(&mut self, data: &[u8]);
        }
    }

    pub fn empty() -> Self {
        Self::from(Self::CODE, Default::default())
    }

    pub fn from<C: Into<Code>>(code: C, content: Vec<u8>) -> Self {
        let code = code.into();
        assert_eq!(code, Self::CODE);

        let c = Self {
            data: SnapshotChunkData {
                code,
                data: content
            }
        };

        c
    }

    pub fn add_entry(&mut self, entry: &RemuEntry) {
        self.add_bytes(entry.to_string().as_bytes());
    }
}
