use std::ops::{Deref, DerefMut};

use crate::{MemoryChunk, SnapshotChunk};

pub const BANK_SIZE: usize = 0x4000;

/// 3 different states are possible. No memory, 64kb or 128kb
#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum SnapshotMemory {
    /// No memory is stored within the snapshot
    Empty(Box<[u8; 0]>),
    /// A 64kb page is stored within the snapshot
    SixtyFourKb(Box<[u8; BANK_SIZE * 4]>),
    /// A 128kb page is stored within the snapshot
    OneHundredTwentyHeightKb(Box<[u8; BANK_SIZE * 8]>),
    // 192
    OneHundredNinetyTwoKb(Box<[u8; BANK_SIZE * 12]>),
    // 256
    TwoHundredFiftySixKb(Box<[u8; BANK_SIZE * 16]>),
    //  320
    ThreeHundredTwentyKb(Box<[u8; BANK_SIZE * 20]>),
    // 384
    ThreeHundredHeightyFourKb(Box<[u8; BANK_SIZE * 24]>),
    // 448
    FourHundredFortyHeightKb(Box<[u8; BANK_SIZE * 28]>),
    //  512
    FiveHundredTwelveKb(Box<[u8; BANK_SIZE * 32]>),
    //  576
    FiveHundredSeventySixKb(Box<[u8; BANK_SIZE * 36]>)
}

impl Default for SnapshotMemory {
    fn default() -> Self {
        Self::Empty(Default::default())
    }
}

impl std::fmt::Debug for SnapshotMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = match self {
            SnapshotMemory::Empty(_) => "Empty",
            SnapshotMemory::SixtyFourKb(_) => "64kb",
            SnapshotMemory::OneHundredTwentyHeightKb(_) => "128kb",
            SnapshotMemory::OneHundredNinetyTwoKb(_) => "192kb",
            SnapshotMemory::TwoHundredFiftySixKb(_) => "256kb",
            SnapshotMemory::ThreeHundredTwentyKb(_) => "320kb",
            SnapshotMemory::ThreeHundredHeightyFourKb(_) => "384kb",
            SnapshotMemory::FourHundredFortyHeightKb(_) => "448kb",
            SnapshotMemory::FiveHundredTwelveKb(_) => "512kb",
            SnapshotMemory::FiveHundredSeventySixKb(_) => "576kb"
        };
        write!(f, "SnapshotMemory ({code})")
    }
}

#[allow(missing_docs)]
impl SnapshotMemory {
    pub fn nb_pages(&self) -> usize {
        match self {
            SnapshotMemory::Empty(_) => 0,
            SnapshotMemory::SixtyFourKb(_) => 1,
            SnapshotMemory::OneHundredTwentyHeightKb(_) => 2,
            SnapshotMemory::OneHundredNinetyTwoKb(_) => 3,
            SnapshotMemory::TwoHundredFiftySixKb(_) => 4,
            SnapshotMemory::ThreeHundredTwentyKb(_) => 5,
            SnapshotMemory::ThreeHundredHeightyFourKb(_) => 6,
            SnapshotMemory::FourHundredFortyHeightKb(_) => 7,
            SnapshotMemory::FiveHundredTwelveKb(_) => 8,
            SnapshotMemory::FiveHundredSeventySixKb(_) => 9
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Empty(_) => true,
            _ => false
        }
    }

    pub fn is_64k(&self) -> bool {
        match self {
            Self::SixtyFourKb(_) => true,
            _ => false
        }
    }

    pub fn is_128k(&self) -> bool {
        match self {
            Self::OneHundredTwentyHeightKb(_) => true,
            _ => false
        }
    }

    /// Read only access to the memory
    pub fn memory(&self) -> &[u8] {
        match self {
            SnapshotMemory::Empty(mem) => mem.deref(),
            SnapshotMemory::SixtyFourKb(mem) => mem.deref(),
            SnapshotMemory::OneHundredTwentyHeightKb(mem) => mem.deref(),
            SnapshotMemory::OneHundredNinetyTwoKb(mem) => mem.deref(),
            SnapshotMemory::TwoHundredFiftySixKb(mem) => mem.deref(),
            SnapshotMemory::ThreeHundredTwentyKb(mem) => mem.deref(),
            SnapshotMemory::ThreeHundredHeightyFourKb(mem) => mem.deref(),
            SnapshotMemory::FourHundredFortyHeightKb(mem) => mem.deref(),
            SnapshotMemory::FiveHundredTwelveKb(mem) => mem.deref(),
            SnapshotMemory::FiveHundredSeventySixKb(mem) => mem.deref()
        }
    }

    /// Reda write access to the memory
    pub fn memory_mut(&mut self) -> &mut [u8] {
        match self {
            SnapshotMemory::Empty(mem) => mem.deref_mut(),
            SnapshotMemory::SixtyFourKb(mem) => mem.deref_mut(),
            SnapshotMemory::OneHundredTwentyHeightKb(mem) => mem.deref_mut(),
            SnapshotMemory::OneHundredNinetyTwoKb(mem) => mem.deref_mut(),
            SnapshotMemory::TwoHundredFiftySixKb(mem) => mem.deref_mut(),
            SnapshotMemory::ThreeHundredTwentyKb(mem) => mem.deref_mut(),
            SnapshotMemory::ThreeHundredHeightyFourKb(mem) => mem.deref_mut(),
            SnapshotMemory::FourHundredFortyHeightKb(mem) => mem.deref_mut(),
            SnapshotMemory::FiveHundredTwelveKb(mem) => mem.deref_mut(),
            SnapshotMemory::FiveHundredSeventySixKb(mem) => mem.deref_mut()
        }
    }

    pub fn len(&self) -> usize {
        self.memory().len()
    }

    pub fn decreased_size(&self) -> Self {
        todo!()
    }

    /// Produce a novel memory that is bigger
    pub fn increased_size(&self) -> Self {
        match self {
            SnapshotMemory::Empty(_mem) => Self::default_64(),
            SnapshotMemory::SixtyFourKb(mem) => {
                let mut new = Self::default_128();
                new.memory_mut()[0..self.len()].copy_from_slice(mem.deref());
                new
            },
            SnapshotMemory::OneHundredTwentyHeightKb(mem) => {
                let mut new = Self::default_192();
                new.memory_mut()[0..self.len()].copy_from_slice(mem.deref());
                new
            },
            SnapshotMemory::OneHundredNinetyTwoKb(mem) => {
                let mut new = Self::default_256();
                new.memory_mut()[0..self.len()].copy_from_slice(mem.deref());
                new
            },
            SnapshotMemory::TwoHundredFiftySixKb(mem) => {
                let mut new = Self::default_320();
                new.memory_mut()[0..self.len()].copy_from_slice(mem.deref());
                new
            },
            SnapshotMemory::ThreeHundredTwentyKb(mem) => {
                let mut new = Self::default_384();
                new.memory_mut()[0..self.len()].copy_from_slice(mem.deref());
                new
            },
            SnapshotMemory::ThreeHundredHeightyFourKb(mem) => {
                let mut new = Self::default_448();
                new.memory_mut()[0..self.len()].copy_from_slice(mem.deref());
                new
            },
            SnapshotMemory::FourHundredFortyHeightKb(mem) => {
                let mut new = Self::default_512();
                new.memory_mut()[0..self.len()].copy_from_slice(mem.deref());
                new
            },
            SnapshotMemory::FiveHundredTwelveKb(mem) => {
                let mut new = Self::default_576();
                new.memory_mut()[0..self.len()].copy_from_slice(mem.deref());
                new
            },
            SnapshotMemory::FiveHundredSeventySixKb(_) => unreachable!()
        }
    }

    pub fn new(source: &[u8]) -> Self {
        match source.len() {
            0 => Self::default(),
            0x10000 => Self::new_64(source),
            0x20000 => Self::new_128(source),
            _ => unreachable!()
        }
    }

    /// Build the chunk representation for the memory sapcz
    pub fn to_chunks(&self) -> Vec<SnapshotChunk> {
        let memory = self.memory();
        let mut chunks = Vec::new();
        let mut current_idx = [b'M', b'E', b'M', b'0'];
        let mut cursor = 0;

        while cursor < memory.len() {
            let next_cursor = cursor + 64 * 1024;
            let current_memory = &memory[cursor..next_cursor.min(memory.len())];

            let current_chunk = MemoryChunk::build(current_idx, current_memory, true);
            chunks.push(SnapshotChunk::Memory(current_chunk));
            cursor = next_cursor;
            current_idx[3] += 1;
        }

        chunks
    }

    pub fn new_64(source: &[u8]) -> Self {
        assert_eq!(source.len(), 64 * 1024);
        let mut mem = Self::default_64();
        mem.memory_mut().copy_from_slice(source);
        mem
    }

    pub fn new_128(source: &[u8]) -> Self {
        assert_eq!(source.len(), 128 * 1024);
        let mut mem = Self::default_128();
        mem.memory_mut().copy_from_slice(source);
        mem
    }

    pub fn default_64() -> Self {
        Self::SixtyFourKb(Box::new([0; BANK_SIZE * 4]))
    }

    pub fn default_128() -> Self {
        Self::OneHundredTwentyHeightKb(Box::new([0; BANK_SIZE * 8]))
    }

    pub fn default_192() -> Self {
        Self::OneHundredNinetyTwoKb(Box::new([0; BANK_SIZE * 12]))
    }

    pub fn default_256() -> Self {
        Self::TwoHundredFiftySixKb(Box::new([0; BANK_SIZE * 16]))
    }

    pub fn default_320() -> Self {
        Self::ThreeHundredTwentyKb(Box::new([0; BANK_SIZE * 20]))
    }

    pub fn default_384() -> Self {
        Self::ThreeHundredHeightyFourKb(Box::new([0; BANK_SIZE * 24]))
    }

    pub fn default_448() -> Self {
        Self::FourHundredFortyHeightKb(Box::new([0; BANK_SIZE * 28]))
    }

    pub fn default_512() -> Self {
        Self::FiveHundredTwelveKb(Box::new([0; BANK_SIZE * 32]))
    }

    pub fn default_576() -> Self {
        Self::FiveHundredSeventySixKb(Box::new([0; BANK_SIZE * 36]))
    }
}
