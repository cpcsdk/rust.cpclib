use std::ops::AddAssign;
use std::ops::DerefMut;

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
    pub(crate) data: SnapshotChunkData,
}

#[allow(missing_docs)]
impl MemoryChunk {
    /// Create a memory chunk.
    /// `code` identify with memory block is concerned
    /// `data` contains the crunched version of the code
    pub fn from(code: [u8; 4], data: Vec<u8>) -> Self {
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
        dbg!(code);
        Self {
            data: SnapshotChunkData { code, data },
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
        } else {
            let mut previous = None;
            let mut count = 0;

            for current in data.iter() {
                match previous {
                    None => {
                        previous = Some(*current);
                        count = 1;
                    }
                    Some(previous_value) => {
                        if *current == 0xe5 || previous_value != *current || count == 255 {
                            if previous.is_some() {
                                // previous value has been repeated several times
                                if count > 1 {
                                    res.push(0xe5);
                                    res.push(count);
                                }
                                res.push(previous_value); // store the value to be replaced
                            }

                            if *current == 0xe5 {
                                previous = None;
                                count = 0;
                                res.push(0xe5);
                                res.push(0);
                            } else {
                                previous = Some(*current);
                                count = 1;
                            }
                        } else {
                            assert_eq!(previous_value, *current);
                            count += 1;
                            previous = Some(*current);
                        }
                    }
                } // end match
            } // end for

            if previous.is_some() {
                // previous value has been repeated several times
                if count > 1 {
                    res.push(0xe5);
                    res.push(count);
                }
                res.push(previous.unwrap()); // store the value to be replaced
            }
        }

        Self::from(code, res)
    }

    /// Uncrunch the 64kbio of RLE crunched data if crunched. Otherwise, return the whole memory
    pub fn uncrunched_memory(&self) -> Vec<u8> {
        if self.is_crunched() {
            return self.data.data.clone();
        }

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
                    if amount == 0 {
                        content.push(0xe5)
                    } else {
                        let val = read_byte();
                        for _idx in 0..amount {
                            // TODO use resize
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

    /// A uncrunched memory taaks 64*1024 bytes
    pub fn is_crunched(&self) -> bool {
        self.data.data.len() == 64 * 1024
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