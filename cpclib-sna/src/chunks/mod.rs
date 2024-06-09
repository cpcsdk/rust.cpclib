mod ace;
mod remu;
mod winape;

use std::ops::Deref;

pub use ace::*;
use cpclib_common::riff::{RiffChunk, RiffCode, RiffLen};
use delegate::delegate;
pub use remu::*;
pub use winape::*;



#[derive(Clone, Debug)]
/// Memory chunk that superseeds the snapshot memory if any.
pub struct MemoryChunk {
    /// Raw content of the memory chunk (i.e. compressed version)
    riff: RiffChunk
}


impl From<RiffChunk> for MemoryChunk {
    fn from(value: RiffChunk) -> Self {
        Self{riff: value}
    }
}

impl Deref for MemoryChunk {
    type Target = RiffChunk;

    fn deref(&self) -> &Self::Target {
        &self.riff
    }
}

#[allow(missing_docs)]
impl MemoryChunk {
    delegate! {
        to self.riff {
        pub fn code(&self) -> &RiffCode;
            pub fn len(&self) -> &RiffLen;
            pub fn data(&self) -> &[u8];
        }
    }

    pub fn print_info(&self) {
        println!(
            "\t* Address: 0x{:X}\n\t* Size: 0x{:X}",
            self.abstract_address(),
            self.uncrunched_memory().len()
        );
    }

    /// Create a memory chunk.
    /// `code` identify with memory block is concerned
    /// `data` contains the crunched version of the code
    pub fn new<C: Into<RiffCode>>(code: C, data: Vec<u8>) -> Self {
        let code = code.into();
        assert!(code[0] == b'M');
        assert!(code[1] == b'E');
        assert!(code[2] == b'M');
        assert!(
            code[3] == b'0'
                || code[3] == b'1'
                || code[3] == b'2'
                || code[3] == b'3'
                || code[3] == b'4'
                || code[3] == b'5'
                || code[3] == b'6'
                || code[3] == b'7'
                || code[3] == b'8'
        );
        Self {
            riff: RiffChunk::new(code, data)
        }
    }

    /// Build the memory chunk from the memory content. Chunk can be built in a compressed or uncompressed version
    pub fn build(code: [u8; 4], data: &[u8], compressed: bool) -> Self {
        assert_eq!(data.len(), 64 * 1024);
        let mut res = Vec::new();

        if !compressed {
            assert_eq!(data.len(), 64 * 1024);
            res.extend(data);
            assert_eq!(res.len(), data.len());
            res.resize(0x100000, 0);
            Self::new(code, res)
        }
        else {
            let mut previous = None;
            let mut count = 0;

            let mut rle = |previous_value, count| {
                if count == 1 {
                    if previous_value == 0xE5 {
                        res.push(0xE5);
                        res.push(0x00);
                    }
                    else {
                        res.push(previous_value);
                    }
                }
                else if count == 2 && previous_value != 0xE5 {
                    res.push(previous_value);
                    res.push(previous_value);
                }
                else {
                    res.push(0xE5);
                    res.push(count);
                    res.push(previous_value);
                }
            };

            for current in data.iter() {
                let current = *current;
                match previous {
                    None => {
                        previous.replace(current);
                        count = 1;
                    },
                    Some(previous_value) => {
                        // we stop when 255 are read or when current differs
                        if previous_value != current || count == 255 {
                            rle(previous_value, count);

                            previous.replace(current);
                            count = 1;
                        }
                        else {
                            count += 1;
                        }
                    }
                } // end match
            } // end for

            if count > 0 {
                rle(previous.unwrap(), count);
            }

            // We may be unable to crunch the memory
            if res.len() >= 65536 {
                return Self::new(code, data.to_vec());
            }

            let chunk = Self::new(code, res.clone());

            // #[cfg(debug_assertions)]
            {
                let produced = chunk.uncrunched_memory();
                assert_eq!(&data, &produced);
            }

            chunk
        }
    }

    /// Uncrunch the 64kbio of RLE crunched data if crunched. Otherwise, return the whole memory
    pub fn uncrunched_memory(&self) -> Vec<u8> {
        if !self.is_crunched() {
            return self.riff.data().to_vec();
        }

        let mut content = Vec::new();

        let mut idx = 0;
        let data = self.riff.data();
        let mut read_byte = move || {
            if idx == self.riff.data().len() {
                None
            }
            else {
                let byte = data[idx];
                idx += 1;
                Some(byte)
            }
        };
        while let Some(byte) = read_byte() {
            match byte {
                0xE5 => {
                    let amount = read_byte().unwrap();
                    if amount == 0 {
                        content.push(0xE5)
                    }
                    else {
                        let val = read_byte().unwrap();
                        content.reserve(content.len() + amount as usize);
                        for _idx in 0..amount {
                            content.push(val);
                        }
                    }
                },
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
        let nb = (self.riff.code()[3] - b'0') as usize;
        nb * 0x10000
    }

    /// A uncrunched memory taaks 64*1024 bytes
    pub fn is_crunched(&self) -> bool {
        self.riff.data().len() != 64 * 1024
    }
}

#[derive(Clone, Debug)]
/// Unknwon kind of chunk
pub struct UnknownChunk {
    /// Raw data of the chunk
    riff: RiffChunk
}

impl From<RiffChunk> for UnknownChunk {
    fn from(value: RiffChunk) -> Self {
        Self{riff: value}
    }
}

impl Deref for UnknownChunk {
    type Target=RiffChunk;

    fn deref(&self) -> &Self::Target {
        &self.riff
    }
}

impl UnknownChunk {
    delegate! {
        to self.riff {
        pub fn code(&self) -> &RiffCode;
            pub fn len(&self) -> &RiffLen;
            pub fn data(&self) -> &[u8];
        }
    }

    /// Generate the chunk from raw data
    pub fn new<C: Into<RiffCode>>(code: C, data: Vec<u8>) -> Self {
        let code = code.into();
        Self {
            riff: RiffChunk::new(code, data)
        }
    }
}

// pub struct InsertedDiscChunk {
// pub fn from(code: [u8;4], content: Vec<u8>) -> Self {
// unimplemented!()
// }
// }
//
// pub struct CPCPlusChunk {
// pub fn from(code: [u8;4], content: Vec<u8>) -> Self {
// unimplemented!()
// }
// }

#[derive(Clone, Debug)]
/// Represents any kind of chunks in order to manipulate them easily based on their semantic
pub enum SnapshotChunk {
    AceBreakPoint(AceBreakPointChunk),

    AceSymbol(AceSymbolChunk),
    /// The chunk is a memory chunk
    Memory(MemoryChunk),
    Remu(RemuChunk),
    /// The type of the chunk is unknown
    Unknown(UnknownChunk),
    /// The chunk is a breakpoint chunk for winape emulator
    WinapeBreakPoint(WinapeBreakPointChunk)
}

#[allow(missing_docs)]
impl SnapshotChunk {
    pub fn print_info(&self) {
        println!(
            "- Chunk: {}{}{}{}",
            self.code()[0] as char,
            self.code()[1] as char,
            self.code()[2] as char,
            self.code()[3] as char,
        );

        if let Some(chunk) = self.memory_chunk() {
            chunk.print_info();
        }
        else if let Some(chunk) = self.ace_symbol_chunk() {
            chunk.print_info();
        }
    }

    pub fn is_memory_chunk(&self) -> bool {
        self.memory_chunk().is_some()
    }

    pub fn is_ace_symbol_chunk(&self) -> bool {
        self.memory_chunk().is_some()
    }

    pub fn memory_chunk(&self) -> Option<&MemoryChunk> {
        match self {
            SnapshotChunk::Memory(ref mem) => Some(mem),
            _ => None
        }
    }

    pub fn ace_symbol_chunk(&self) -> Option<&AceSymbolChunk> {
        match self {
            SnapshotChunk::AceSymbol(ref sym) => Some(sym),
            _ => None
        }
    }


    pub fn riff(&self) -> &RiffChunk {
        match self {
            SnapshotChunk::AceBreakPoint(a) => a.deref(),
            SnapshotChunk::AceSymbol(a) => a.deref(),
            SnapshotChunk::Memory(m) => m.deref(),
            SnapshotChunk::Remu(r) => r.deref(),
            SnapshotChunk::Unknown(u) => u.deref(),
            SnapshotChunk::WinapeBreakPoint(w) => w.deref(),
        }
    }

    /// Provides the code of the chunk
    pub fn code(&self) -> &RiffCode {
        self.riff().code()
    }

    pub fn len(&self) -> &RiffLen {
        self.riff().len()
    }

    pub fn data(&self) -> &[u8] {
        self.riff().data()
    }
}

impl From<AceSymbolChunk> for SnapshotChunk {
    fn from(chunk: AceSymbolChunk) -> Self {
        SnapshotChunk::AceSymbol(chunk)
    }
}

impl From<MemoryChunk> for SnapshotChunk {
    fn from(chunk: MemoryChunk) -> Self {
        SnapshotChunk::Memory(chunk)
    }
}

impl From<WinapeBreakPointChunk> for SnapshotChunk {
    fn from(chunk: WinapeBreakPointChunk) -> Self {
        SnapshotChunk::WinapeBreakPoint(chunk)
    }
}

impl From<AceBreakPointChunk> for SnapshotChunk {
    fn from(chunk: AceBreakPointChunk) -> Self {
        SnapshotChunk::AceBreakPoint(chunk)
    }
}

impl From<RemuChunk> for SnapshotChunk {
    fn from(chunk: RemuChunk) -> Self {
        SnapshotChunk::Remu(chunk)
    }
}

impl From<UnknownChunk> for SnapshotChunk {
    fn from(chunk: UnknownChunk) -> Self {
        SnapshotChunk::Unknown(chunk)
    }
}
