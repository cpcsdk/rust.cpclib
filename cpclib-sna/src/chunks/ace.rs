use std::borrow::Cow;
use std::ops::Deref;

use cpclib_common::itertools::Itertools;
use cpclib_common::riff::{RiffChunk, RiffCode, RiffLen};
use delegate::delegate;

#[derive(Clone)]
pub enum AceMemMapType {
    Undefined,
    MainRam(u8),          // XXX only 0 or 1 ?
    ExtensionRam(u8, u8), // XXX bank (0-3) page (0-31)
    ExtensionRom(u8),     // XXX rom slot (0-255)
    FirmwareRom,
    CartridgeRom(u8),
    AsicIOPage
}

impl From<&AceMemMapType> for u8 {
    fn from(val: &AceMemMapType) -> Self {
        match val {
            AceMemMapType::Undefined => 0,
            AceMemMapType::MainRam(_) => 1,
            AceMemMapType::ExtensionRam(..) => 2,
            AceMemMapType::ExtensionRom(_) => 3,
            AceMemMapType::FirmwareRom => 4,
            AceMemMapType::CartridgeRom(_) => 5,
            AceMemMapType::AsicIOPage => 6
        }
    }
}

impl From<(u8, u8, u8)> for AceMemMapType {
    fn from(value: (u8, u8, u8)) -> Self {
        match value.0 {
            0 => AceMemMapType::Undefined,
            1 => AceMemMapType::MainRam(value.1),
            2 => AceMemMapType::ExtensionRam(value.1, value.2),
            3 => AceMemMapType::ExtensionRom(value.1),
            4 => AceMemMapType::FirmwareRom,
            5 => AceMemMapType::CartridgeRom(value.1),
            6 => AceMemMapType::AsicIOPage,
            _ => unreachable!()
        }
    }
}

impl AceMemMapType {
    #[inline]
    pub fn code(&self) -> u8 {
        self.into()
    }

    #[inline]
    pub fn bank(&self) -> u8 {
        match self {
            Self::MainRam(bank)
            | Self::ExtensionRam(bank, _)
            | Self::ExtensionRom(bank)
            | Self::CartridgeRom(bank) => *bank,
            _ => 0
        }
    }

    #[inline]
    pub fn page(&self) -> u8 {
        match self {
            Self::ExtensionRam(_, page) => *page,
            _ => 0
        }
    }
}

#[repr(u8)]
pub enum AceSymbolType {
    Absolute,
    Relative
}

#[derive(Clone, Debug)]
pub struct AceSymbol<'a> {
    buffer: Cow<'a, [u8]>
}

impl From<Vec<u8>> for AceSymbol<'_> {
    fn from(value: Vec<u8>) -> Self {
        let name_len = value[0];
        let buffer_len = Self::buffer_len(name_len as _);
        assert_eq!(buffer_len, value.len());

        Self {
            buffer: Cow::Owned(value)
        }
    }
}

impl<'a> From<&'a [u8]> for AceSymbol<'a> {
    fn from(value: &'a [u8]) -> Self {
        let name_len = value[0];
        let buffer_len = Self::buffer_len(name_len as _);
        assert_eq!(buffer_len, value.len());

        Self {
            buffer: Cow::Borrowed(value)
        }
    }
}

impl AceSymbol<'_> {
    pub fn buffer_len(name_len: u8) -> usize {
        1 + name_len as usize + 1 + 1 + 1 + 2 + 1 + 2
    }

    pub fn new(name: &str, address: u16, map_type: AceMemMapType, sym_type: AceSymbolType) -> Self {
        let name_len = name.len().min(255);
        assert!(name_len != 0);

        let buffer_len = Self::buffer_len(name_len as _);
        let mut buffer = vec![0u8; buffer_len];

        buffer[0] = name_len as _;
        for (idx, b) in name.as_bytes().iter().enumerate() {
            buffer[1 + idx] = *b;
        }

        buffer[name_len + 1] = map_type.code();
        buffer[name_len + 2] = map_type.bank();
        buffer[name_len + 3] = map_type.page();

        buffer[name_len + 6] = sym_type as u8;

        buffer[name_len + 7] = (address / 256) as u8;
        buffer[name_len + 8] = (address % 256) as u8;

        buffer.into()
    }

    pub fn name_len(&self) -> u8 {
        self.buffer[0]
    }

    pub fn name(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.buffer[1..(self.name_len() as usize + 1)]) }
    }

    pub fn mem_type(&self) -> AceMemMapType {
        let name_len = self.name_len() as usize;
        (
            self.buffer[name_len + 1],
            self.buffer[name_len + 2],
            self.buffer[name_len + 3]
        )
            .into()
    }

    pub fn address(&self) -> u16 {
        let name_len = self.name_len() as usize;
        (self.buffer[name_len + 7] as u16) * 256 + (self.buffer[name_len + 8] as u16)
    }
}

#[derive(Clone, Debug)]
pub struct AceSymbolChunk {
    riff: RiffChunk
}

impl From<RiffChunk> for AceSymbolChunk {
    fn from(value: RiffChunk) -> Self {
        Self { riff: value }
    }
}

impl Deref for AceSymbolChunk {
    type Target = RiffChunk;

    fn deref(&self) -> &Self::Target {
        &self.riff
    }
}

impl AceSymbolChunk {
    const CODE: RiffCode = RiffCode::new([b'S', b'Y', b'M', b'B']);

    delegate! {
        to self.riff {
            pub fn code(&self) -> &RiffCode;
            pub fn len(&self) -> &RiffLen;
            pub fn data(&self) -> &[u8];
            fn add_bytes(&mut self, data: &[u8]);
        }
    }

    pub fn empty() -> Self {
        Self::new(Self::CODE, Vec::new())
    }

    pub fn new<C: Into<RiffCode>>(code: C, content: Vec<u8>) -> Self {
        let code = code.into();
        assert_eq!(code, Self::CODE);

        Self {
            riff: RiffChunk::new(code, content)
        }
    }

    /// Add a symbol in the chunk. Warning it is cropped to a length of 255
    pub fn add_symbol(&mut self, sym: AceSymbol) {
        self.add_bytes(&sym.buffer);
    }

    pub fn add_symbols<'a, S: Iterator<Item = AceSymbol<'a>>>(&mut self, symbs: S) {
        symbs.for_each(|s| self.add_symbol(s));
    }

    pub fn get_symbols(&self) -> Vec<AceSymbol> {
        let mut res = Vec::new();

        let mut idx = 0;
        while idx < self.len().value() as usize {
            let name_len = self.data()[idx];
            let buffer_len = AceSymbol::buffer_len(name_len);
            let symb = AceSymbol::from(self.data()[idx..(idx + buffer_len)].to_vec());
            res.push(symb);

            idx += buffer_len;
        }

        res
    }

    pub fn print_info(&self) {
        let s = self
            .get_symbols()
            .into_iter()
            .map(|s| format!("{} = 0x{:.4x}", s.name(), s.address()))
            .join("\n");
        println!("{s}")
    }
}

// https://www.cpcwiki.eu/index.php/Snapshot#BRKC

pub struct AceBreakPoint<'a> {
    buffer: Cow<'a, [u8; 216]>
}

#[repr(u8)]
pub enum AceBrkRuntimeMode {
    Break = 0,
    Watch = 1
}

impl From<[u8; 216]> for AceBreakPoint<'_> {
    fn from(value: [u8; 216]) -> Self {
        Self {
            buffer: Cow::Owned(value)
        }
    }
}

impl<'a> From<&'a [u8; 216]> for AceBreakPoint<'a> {
    fn from(value: &'a [u8; 216]) -> Self {
        Self {
            buffer: Cow::Borrowed(value)
        }
    }
}

impl<'a> From<AceBreakPoint<'a>> for [u8; 216] {
    fn from(val: AceBreakPoint<'a>) -> Self {
        val.buffer.into_owned()
    }
}

impl AceBreakPoint<'_> {
    const BRK_TYPE_EXEC: u8 = 0;
    const BRK_TYPE_MEM: u8 = 1;
    const BRK_TYPE_PORT: u8 = 2;

    pub fn new_execution(
        address: u16,
        runtime_mode: AceBrkRuntimeMode,
        map_type: AceMemMapType
    ) -> Self {
        let mut buffer = [0; 216];
        buffer[0x00] = Self::BRK_TYPE_EXEC;
        buffer[0x02] = runtime_mode as u8;
        buffer[0x03] = map_type.code();
        buffer[0xD0] = map_type.bank();
        buffer[0xD1] = map_type.page();
        buffer[0x04] = (address / 256) as u8; // I have followed instructions on https://www.cpcwiki.eu/index.php/Snapshot#BRKC but rasm seems to do the opposite
        buffer[0x05] = (address % 256) as u8;
        buffer.into()
    }

    pub fn with_condition(self, condition: &str) -> Self {
        assert!(condition.len() < 127);
        let mut buffer = self.buffer.into_owned();
        for (idx, b) in condition.as_bytes().iter().enumerate() {
            buffer[0x10 + idx] = *b;
        }
        buffer[0x10 + condition.len()] = 0;
        buffer.into()
    }

    pub fn with_user_name(self, user_name: &str) -> Self {
        assert!(user_name.len() < 63);
        let mut buffer = self.buffer.into_owned();

        for (idx, b) in user_name.as_bytes().iter().enumerate() {
            buffer[0x90 + idx] = *b;
        }
        buffer[0x90 + user_name.len()] = 0;
        buffer.into()
    }
}

#[derive(Clone, Debug)]
pub struct AceBreakPointChunk {
    riff: RiffChunk
}

impl From<RiffChunk> for AceBreakPointChunk {
    fn from(value: RiffChunk) -> Self {
        Self { riff: value }
    }
}

impl Deref for AceBreakPointChunk {
    type Target = RiffChunk;

    fn deref(&self) -> &Self::Target {
        &self.riff
    }
}

impl AceBreakPointChunk {
    const CODE: RiffCode = RiffCode::new([b'B', b'R', b'K', b'C']);

    delegate! {
        to self.riff {
            pub fn code(&self) -> &RiffCode;
            pub fn len(&self) -> &RiffLen;
            pub fn data(&self) -> &[u8];
            fn add_bytes(&mut self, data: &[u8]);

        }
    }

    pub fn empty() -> Self {
        Self::new(Self::CODE, Vec::new())
    }

    pub fn new<C: Into<RiffCode>>(code: C, content: Vec<u8>) -> Self {
        let code = code.into();
        assert_eq!(code, Self::CODE);

        Self {
            riff: RiffChunk::new(code, content)
        }
    }

    pub fn add_breakpoint_raw(&mut self, raw: &[u8; 216]) {
        assert!(raw.len() == 216);
        self.add_bytes(raw);
    }

    pub fn add_breakpoint(&mut self, brk: AceBreakPoint) {
        self.add_breakpoint_raw(&brk.buffer)
    }

    pub fn nb_breakpoints(&self) -> usize {
        self.len().value() as usize / 216
    }
}
