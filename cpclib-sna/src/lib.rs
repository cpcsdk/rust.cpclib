#![feature(vec_drain_as_slice)]

use bitsets;

use std::fmt;
use std::fs::File;

use std::io::prelude::*;

use std::path::Path;
use std::str::FromStr;

use num_enum::TryFromPrimitive;
use std::ops::AddAssign;
use std::ops::DerefMut;

pub mod flags;
pub mod parse;
mod chunks;
mod error;

pub use flags::*;
pub use chunks::*;
pub use error::*;

///! Re-implementation of createsnapshot by Ramlaid/Arkos
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
    /// Header of the snaphsot
    header: [u8; HEADER_SIZE],
    /// Memory for V2 snapshot or V3 before saving
    memory: SnapshotMemory,
    memory_already_written: bitsets::DenseBitSet,
    /// list of chuncks; memory chuncks are removed once memory is written
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

        assert!(memory_dump_size * 1024 <= file_content.len());
        sna.memory = SnapshotMemory::new(file_content.drain(0..memory_dump_size * 1024).as_slice());

        if version == 3 {
            while let Some(chunk) = Self::read_chunk(
                                            &mut file_content, 
                                            &mut sna) {
                sna.chunks.push(chunk);
            }
        }

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

        // Compress memory chunks for V3
        if version == SnapshotVersion::V3 && !self.has_memory_chunk() {
            println!("Generate chunks from standard memory");
            let chunks = cloned.memory.to_chunks();
            for idx in 0..chunks.len() {
                cloned.chunks.insert(idx, chunks[idx].clone());
            }
            cloned.memory = SnapshotMemory::default();
            cloned.set_memory_size_header(0);
        }

        cloned
    }

    /// Read a chunk if available
    fn read_chunk(file_content: &mut Vec<u8>, _sna: &mut Self) -> Option<SnapshotChunk> {
        if file_content.len() < 4 {
            return None;
        }

        let code = file_content.drain(0..4).as_slice().to_vec();
        let data_length = file_content.drain(0..4).as_slice().to_vec();

       // eprintln!("{:?} / {:?}", std::str::from_utf8(&code), data_length);

       // compute the data length based on the 4 bytes that represent it
        let data_length = {
            let mut count = 0;
            for i in 0..4 {
                count = 256 * count + data_length[3 - i] as usize;
            }
            count
        };

        // read the appropriate number of bytes
        let content = file_content.drain(0..data_length).as_slice().to_vec();

        // Generate the 4 size array
        let code = {
            let mut new_code = [0; 4];
            assert_eq!(code.len(), 4);
            new_code.copy_from_slice(&code);
            new_code
        };
        let chunk = match code {
            [b'M', b'E', b'M', _] => MemoryChunk::from(code, content).into(), /*
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
    pub fn save_sna<P: AsRef<Path>>(&self, fname: P) -> Result<(), std::io::Error> {
        self.save(fname, SnapshotVersion::V2)
    }

    pub fn save<P: AsRef<Path>>(&self, fname: P, version: SnapshotVersion) -> Result<(), std::io::Error> {
        let mut buffer = File::create(fname.as_ref())?;
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
        println!("Memory header: {}", sna.memory_size_header() );

        // Write chunks if any
        for chunk in &sna.chunks {
            println!("Add chunk: {:?}", chunk.code());
            buffer.write_all(chunk.code())?;
            buffer.write_all(&chunk.size_as_array())?;
            buffer.write_all(chunk.data())?;
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
                let address = memory_chunk.abstract_address();
                let content = memory_chunk.uncrunched_memory();
                max_memory = address + 64 * 1024;

                if memory.len() < max_memory {
                    memory = memory.increased_size();
                }
                memory.memory_mut()[address..max_memory].copy_from_slice(&content);
                // TODO treat the case where extra memory is used as `memroy` may need to be extended
            }
        }
        memory.memory()[..max_memory].to_vec()
    }

    /// Check if the snapshot has some memory chunk
    pub fn has_memory_chunk(&self) -> bool {
        self.chunks.iter().any(|c|{
            c.is_memory_chunk()
        })
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
    /// use cpclib_sna::Snapshot;
    ///
    /// let mut sna = Snapshot::default();
    /// let data = vec![0,2,3,5];
    /// sna.add_data(&data, 0x4000);
    /// 
    /// ```
    /// TODO: re-implement with set_byte
    /// 
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

    #[deprecated]
    pub fn set_memory(&mut self, address: u32, value: u8) {
        self.set_byte(address, value);
    }

    /// Change a memory value. Panic if memory size is not appropriate
    /// If memory is saved insided chuncks, the chuncks are unwrapped
    pub fn set_byte(&mut self, address: u32, value: u8) {
        let address = address as usize;

        // unroll chuncks if any
        if self.memory.is_empty() {
            self.memory = SnapshotMemory::new(&self.memory_dump());
            let mut idx = 0;
            while idx < self.chunks.len() {
                if self.chunks[0].is_memory_chunk() {
                    self.chunks.remove(idx);
                }
                else {
                    idx +=1;
                }
            } 
            self.set_memory_size_header( (self.memory.len() / 1024) as u16);

        }
 
        // finally write in memory
        self.memory.memory_mut()[address] = value;
    }

    pub fn get_byte(&self, address: u32) -> u8 {
        self.memory.memory()[address as usize]
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
