#![feature(slice_take)]

use std::fs::File;
use std::io::prelude::*;
use std::ops::Deref;
use std::path::Path;

use cpclib_common::bitsets;
use cpclib_common::bitvec::vec::BitVec;
use cpclib_common::riff::{RiffChunk, RiffCode};
use num_enum::TryFromPrimitive;

mod chunks;
mod error;
pub mod flags;
mod memory;
pub mod parse;

#[cfg(feature = "snapshot")]
pub mod cli;

pub use chunks::*;
pub use error::*;
pub use flags::*;
pub use memory::*;

/// ! Re-implementation of createsnapshot by Ramlaid/Arkos
/// ! in rust by Krusty/Benediction

/// Original options
///
/// {'i',"inSnapshot",0,1,1,"Load <$1> snapshot file"},
/// {'l',"loadFileData",0,0,2,"Load <$1> file data at <$2> address in snapshot memory (or use AMSDOS header load address if <$2> is negative)"},
/// {'p',"putData",0,0,2,"Put <$2> byte at <$1> address in snapshot memory"},
/// {'s',"setToken",0,0,2,"Set snapshot token <$1> to value <$2>\n\t\t"
/// "Use <$1>:<val> to set array value\n\t\t"
/// "ex '-s CRTC_REG:6 20' : Set CRTC register 6 to 20"},
/// {'x',"clearMemory",0,1,0,"Clear snapshot memory"},
/// {'r',"romDeconnect",0,1,0,"Disconnect lower and upper rom"},
/// {'e',"enableInterrupt",0,1,0,"Enable interrupt"},
/// {'d',"disableInterrupt",0,1,0,"Disable interrupt"},
/// {'c',"configFile",0,1,1,"Load a config file with createSnapshot option"},
/// {'t',"tokenList",0,1,0,"Display setable snapshot token ID"},
/// {'o',"output",0,0,3,"Output <$3> bytes of data from address <$2> to file <$1>"},
/// {'f',"fillData",0,0,3,"Fill snapshot from <$1> over <$2> bytes, with <$3> datas"},
/// {'g',"fillText",0,0,3,"Fill snapshot from <$1> over <$2> bytes, with <$3> text"},
/// {'j',"loadIniFile",0,1,1,"Load <$1> init file"},
/// {'k',"saveIniFile",0,1,1,"Save <$1> init file"},

pub const HEADER_SIZE: usize = 256;

#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub enum SnapshotVersion {
    /// Version 1 of Snapshsots
    V1 = 1,
    /// Version 2 of Snapshsots
    V2,
    /// Version 3 of Snapshsots (use of chunks)
    V3
}

impl SnapshotVersion {
    /// Check if snapshot ius V3 version
    pub fn is_v3(self) -> bool {
        if let SnapshotVersion::V3 = self {
            true
        }
        else {
            false
        }
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
    memory_already_written: BitVec,
    /// list of chuncks; memory chuncks are removed once memory is written
    chunks: Vec<SnapshotChunk>,

    // nothing to do with the snapshot. Should be moved elsewhere
    pub debug: bool
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
        let mut sna = Self {
            header: [
                0x4D,
                0x56,
                0x20,
                0x2D,
                0x20,
                0x53,
                0x4E,
                0x41, // MV - SNA
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x03, // version
                0x00, // F
                0x00, // A
                0x00, // C
                0x00, // B
                0x00, // E
                0x00, // D
                0x00, // L
                0x00, // H
                0x00, // R
                0x00, // I
                0x00, // IFF0
                0x00, // IFF1
                0x00, // IXL
                0x00, // IXH
                0x00, // IYL
                0x00, // IYH
                0x00, // SP L
                0xC0, // SP h
                0x00, // PCL
                0x00, // PCH
                0x01, // interput mode
                0x00, // F'
                0x00, // A'
                0x00, // C'
                0x00, // B'
                0x00, // E'
                0x00, // D'
                0x00, // L'
                0x00, // H'
                0x00, // selected pen
                0x04,
                0x0A,
                0x15,
                0x1C,
                0x18,
                0x1D,
                0x0C,
                0x05,
                0x0D,
                0x16,
                0x06,
                0x17,
                0x1E,
                0x00,
                0x1F,
                0x0E,
                0x04,              // palette
                0x8D,              // gate array multi conf
                0xC0 & 0b00111111, // ram cfg
                0x00,              // crtc reg selected
                0x3F,
                0x28,
                0x2E,
                0x8E,
                0x26,
                0x00,
                0x19,
                0x1E,
                0x00,
                0x07,
                0x00,
                0x00,
                0x30,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00, // crtc values
                0x00, // rom selection
                0x00, // 0xFF
                // PPI A
                0x00, // 0x1E
                // PPI B
                0x00, // PPI C
                0x82,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x3F,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x40, // 0x80 <= nb of kilobytes
                0x00,
                0x02,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x7F, // printer strobe
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00, // 0x32
                // CRTC horizontal character counter register
                0x00,
                0x00, // 0x08
                // CRTC character-line counter register
                0x00, // 0x02
                // CRTC raster-line counter register
                0x00, // CRTC vertical total adjust counter register
                0x00, // 0x04
                //  	CRTC horizontal sync width counter
                0x00,
                0x00, // 0x01
                0x00,
                0x02,
                0x00, // 0x20
                // ga interrupt scanline
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00
            ],
            memory: SnapshotMemory::default_64(),
            chunks: Vec::new(),
            memory_already_written: BitVec::repeat(false, BANK_SIZE*4*1), // 64kbits
            debug: false
        };

        let end_string = b"BND FRAMEWORK RULEZ!";
        assert_eq!(end_string.len(), 20);
        let start = sna.header.len() - 20;
        sna.header[start..].copy_from_slice(end_string);

        assert_eq!(
            sna.memory.memory().len(),
            sna.memory_size_header() as usize * 1024
        );
        sna
    }
}

impl Snapshot {
    pub fn new_6128() -> Result<Self, String> {
        let content = include_bytes!("cpc6128.sna").to_vec();
        Self::from_buffer(content)
    }

    pub fn new_6128_v2() -> Result<Self, String> {
        let content = include_bytes!("cpc6128_v2.sna").to_vec();
        Self::from_buffer(content)
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

    pub fn load<P: AsRef<Path>>(filename: P) -> Result<Self, String> {
        let filename = filename.as_ref();

        // Read the full content of the file
        let mut file_content = {
            let mut f = File::open(filename).map_err(|e| e.to_string())?;
            let mut content = Vec::new();
            f.read_to_end(&mut content);
            content
        };

        Self::from_buffer(file_content)
    }

    pub fn from_buffer(mut file_content: Vec<u8>) -> Result<Self, String> {
        let mut sna = Self::default();

        if file_content.len() < 0x100 {
            return Err("SNA file is invalid".to_owned());
        }

        // Copy the header
        sna.header
            .copy_from_slice(file_content.drain(0..0x100).as_slice());
        let memory_dump_size = sna.memory_size_header() as usize;
        let version = sna.version_header();

        assert!(memory_dump_size * 1024 <= file_content.len());
        sna.memory = SnapshotMemory::new(file_content.drain(0..memory_dump_size * 1024).as_slice());

        if version == 3 {
            while let Some(chunk) = Self::read_chunk(&mut file_content, &mut sna) {
                sna.chunks.push(chunk);
            }
        }

        Ok(sna)
    }

    /// Count the number of kilobytes that are within chunks
    fn compute_memory_size_in_chunks(&self) -> u16 {
        let nb_pages = self.chunks.iter().filter(|c| c.is_memory_chunk()).count() as u16;
        nb_pages * 64
    }

    pub fn memory_size_header(&self) -> u16 {
        u16::from(self.header[0x6B]) + 256 * u16::from(self.header[0x6C])
    }

    fn set_memory_size_header(&mut self, size: u16) {
        self.header[0x6B] = (size % 256) as _;
        self.header[0x6C] = (size / 256) as _;
    }

    pub fn version_header(&self) -> u8 {
        self.header[0x10]
    }

    pub fn version(&self) -> SnapshotVersion {
        self.version_header().try_into().unwrap()
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
                for idx in 0x6D..=0xFF {
                    cloned.header[idx] = 0;
                }
            },
            SnapshotVersion::V2 => {
                // for idx in 0x75..=0xFF {
                //     cloned.header[idx] = 0;
                // }
                // unused but not set to 0
            },
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
            // println!("Generate chunks from standard memory");
            let chunks = cloned.memory.to_chunks();
            for idx in 0..chunks.len() {
                cloned.chunks.insert(idx, chunks[idx].clone());
            }
            cloned.memory = SnapshotMemory::default();
            cloned.set_memory_size_header(0);
        }

        cloned
    }

    /// Save the snapshot V2 on disc
    #[deprecated]
    pub fn save_sna<P: AsRef<Path>>(&self, fname: P) -> Result<(), std::io::Error> {
        self.save(fname, SnapshotVersion::V2)
    }

    pub fn save<P: AsRef<Path>>(
        &self,
        fname: P,
        version: SnapshotVersion
    ) -> Result<(), std::io::Error> {
        let mut buffer = File::create(fname.as_ref())?;
        self.write_all(&mut buffer, version)
    }

    pub fn write_all<B: Write>(
        &self,
        buffer: &mut B,
        version: SnapshotVersion
    ) -> Result<(), std::io::Error> {
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
            buffer.write_all(sna.memory.memory())?;
        }
        // println!("Memory header: {}", sna.memory_size_header());

        // Write chunks if any
        for chunk in &sna.chunks {
            chunk.riff().write_all(buffer)?;
        }

        Ok(())
    }

    /// Change the value of a flag
    pub fn set_value(&mut self, flag: SnapshotFlag, value: u16) -> Result<(), SnapshotError> {
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
                self.header[offset] = (value % 256) as u8;
                self.header[offset + 1] = (value / 256) as u8;
                Ok(())
            },
            _ => panic!("Unable to handle size != 1 or 2")
        }
    }

    pub fn get_value(&self, flag: &SnapshotFlag) -> FlagValue {
        if flag.indice().is_some() {
            // Here we treate the case where we read only one value
            let offset = flag.offset();
            match flag.elem_size() {
                1 => FlagValue::Byte(self.header[offset]),
                2 => {
                    FlagValue::Word(
                        u16::from(self.header[offset + 1]) * 256 + u16::from(self.header[offset])
                    )
                },
                _ => panic!()
            }
        }
        else {
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
}

/// Memory relaterd code
impl Snapshot {
    #[deprecated]
    pub fn set_memory(&mut self, address: u32, value: u8) {
        self.set_byte(address, value);
    }

    #[inline(always)]
    pub fn nb_pages(&self) -> usize {
        self.memory.nb_pages()
    }

    /// Ensure the sna has the appropriate number of pages
    pub fn resize(&mut self, nb_pages: usize) {
        self.unwrap_memory_chunks();

        while self.nb_pages() < nb_pages {
            self.memory = self.memory.increased_size();
        }
        while self.nb_pages() > nb_pages {
            self.memory = self.memory.decreased_size();
        }

        self.set_memory_size_header(64* self.nb_pages() as u16);
        self.memory_already_written.resize_with(self.nb_pages()*0x1_0000, |_| false)
    }

    /// To play easier with memory, remove all the memory chunks and use a linearized memory version
    /// Memory array MUST be empty before calling this method
    pub fn unwrap_memory_chunks(&mut self) {
        if self.memory.is_empty() {
            // uncrunch the memory blocks
            self.memory = SnapshotMemory::new(&self.memory_dump());

            // remove the memory chunks
            let mut idx = 0;
            while idx < self.chunks.len() {
                if self.chunks[idx].is_memory_chunk() {
                    self.chunks.remove(idx);
                    // no need to increment idx as the next chunk is at the same position
                }
                else {
                    idx += 1;
                }
            }

            // update the memory size
            self.set_memory_size_header((self.memory.len() / 1024) as u16);
        }
    }

    /// Change a memory value. Panic if memory size is not appropriate
    /// If memory is saved insided chuncks, the chuncks are unwrapped
    pub fn set_byte(&mut self, address: u32, value: u8) {
        self.unwrap_memory_chunks();
        let address = address as usize;

        // resize if needed
        while self.memory.len() - 1 < address {
            self.memory = self.memory.increased_size();
        }

        // finally write in memory
        self.memory.memory_mut()[address] = value;
    }

    /// Can only work on snapshot where memory is not stored in chunks.
    /// TODO modify this behavior
    pub fn get_byte(&self, address: u32) -> u8 {
        self.memory.memory()[address as usize]
    }

    /// Returns all the memory of the snapshot in a linear way by mixing both the hardcoded memory of the snapshot and the memory of chunks
    pub fn memory_dump(&self) -> Vec<u8> {
        // by default, the memory i already coded
        let mut memory = self.memory.clone();

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

        while memory.len() < max_memory {
            memory = memory.increased_size();
        }

        memory.memory()[..max_memory].to_vec()
    }

    /// Check if the snapshot has some memory chunk
    pub fn has_memory_chunk(&self) -> bool {
        self.chunks.iter().any(|c| c.is_memory_chunk())
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
    /// ```
    /// TODO: re-implement with set_byte
    pub fn add_data(&mut self, data: &[u8], address: usize) -> Result<(), SnapshotError> {

        let last_used_address = address + data.len() - 1;
        if last_used_address >= 0x10000 * 2 {
            Err(SnapshotError::NotEnougSpaceAvailable)
        }
        else {
            if address < 0x10000 && last_used_address >= 0x10000 {
                eprintln!("[Warning] Start of file is in main memory (0x{:x}) and  end of file is in extra banks (0x{:x}).", address, (address + data.len()));
            }
            // TODO add warning when writting in other banks

            for (idx, byte) in data.iter().enumerate() {
                let current_pos = address + idx;
                if *self.memory_already_written.get(current_pos).unwrap() {
                    eprintln!("[WARNING] Replace memory in 0x{:x}", current_pos);
                }
                self.memory.memory_mut()[current_pos] = *byte;
                self.memory_already_written.set(current_pos, true);
            }

            Ok(())
        }
    }
}

/// Chunks related code
impl Snapshot {
    /// Read a chunk if available
    fn read_chunk(file_content: &mut Vec<u8>, _sna: &mut Self) -> Option<SnapshotChunk> {
        if file_content.len() < 4 {
            return None;
        }

        let chunk = RiffChunk::from_buffer(file_content);

        let chunk: SnapshotChunk = match chunk.code().deref() {
            [b'M', b'E', b'M', _] => MemoryChunk::from(chunk).into(), //
            [b'B', b'R', b'K', b'S'] => WinapeBreakPointChunk::from(chunk).into(),
            [b'B', b'R', b'K', b'C'] => AceBreakPointChunk::from(chunk).into(),
            [b'S', b'Y', b'M', b'B'] => AceSymbolChunk::from(chunk).into(),
            // ['D', 'S', 'C', _] => InsertedDiscChunk::from(code, content)
            // ['C', 'P', 'C', '+'] => CPCPlusChunk::from(content)
            _ => UnknownChunk::from(chunk).into()
        };

        Some(chunk)
    }

    pub fn nb_chunks(&self) -> usize {
        self.chunks.len()
    }

    pub fn chunks(&self) -> &[SnapshotChunk] {
        &self.chunks
    }

    pub fn add_chunk<C: Into<SnapshotChunk>>(&mut self, c: C) {
        self.chunks.push(c.into());
    }

    pub fn get_chunk<C: Into<RiffCode>>(&self, code: C) -> Option<&SnapshotChunk> {
        let code = code.into();
        self.chunks().iter().find(|chunk| chunk.code() == &code)
    }
}

#[cfg(test)]
mod tests {
    use similar_asserts::assert_eq;

    use crate::{Snapshot, BANK_SIZE};

    use super::SnapshotMemory;

    #[test]
    fn test_resize() {
        let mut sna = Snapshot::default();
        assert_eq!(sna.nb_pages(), 1);
        assert_eq!(sna.memory_dump().len(), BANK_SIZE*4);

        sna.resize(2);
        assert_eq!(sna.nb_pages(), 2);
        assert_eq!(sna.memory_dump().len(), BANK_SIZE*4*2);
    }

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
        SnapshotMemory::default_576().increased_size();
    }
}
