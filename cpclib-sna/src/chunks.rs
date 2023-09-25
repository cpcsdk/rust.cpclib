use std::ops::Deref;

use delegate::delegate;

#[derive(Clone, Debug, PartialEq)]
pub struct Code([u8; 4]);

impl Deref for Code {
    type Target = [u8; 4];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<[u8; 4]> for Code {
    fn from(value: [u8; 4]) -> Self {
        Code(value)
    }
}

impl From<&str> for Code {
    fn from(value: &str) -> Self {
        let code = value.as_bytes().take(..4).unwrap();
        Code([code[0], code[1], code[2], code[3]])
    }
}

#[derive(Clone, Debug)]
/// Raw chunk data.
pub struct SnapshotChunkData {
    /// Identifier of the chunk
    code: Code,
    /// Content of the chunk
    data: Vec<u8>
}

#[allow(missing_docs)]
impl SnapshotChunkData {
    pub fn code(&self) -> &Code {
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

    pub fn add_bytes(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
    }
}

#[derive(Clone, Debug)]
/// Memory chunk that superseeds the snapshot memory if any.
pub struct MemoryChunk {
    /// Raw content of the memory chunk (i.e. compressed version)
    pub(crate) data: SnapshotChunkData
}

#[allow(missing_docs)]
impl MemoryChunk {
    delegate! {
        to self.data {
        pub fn code(&self) -> &Code;
            pub fn size(&self) -> usize;
            pub fn size_as_array(&self) -> [u8; 4];
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
    pub fn from<C: Into<Code>>(code: C, data: Vec<u8>) -> Self {
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
            data: SnapshotChunkData { code, data }
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
            Self::from(code, res)
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
                    }
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
                return Self::from(code, data.to_vec());
            }

            let chunk = Self::from(code, res.clone());

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
            return self.data.data.clone();
        }

        let mut content = Vec::new();

        let mut idx = 0;
        let data = &self.data.data;
        let mut read_byte = move || {
            if idx == self.data.data.len() {
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
        self.data.data.len() != 64 * 1024
    }
}

#[derive(Clone, Debug)]
pub struct AceSymbolChunk {
    data: SnapshotChunkData
}

impl AceSymbolChunk {
    delegate! {
        to self.data {
            pub fn code(&self) -> &Code;
            pub fn size(&self) -> usize;
            pub fn size_as_array(&self) -> [u8; 4];
            pub fn data(&self) -> &[u8];
            fn add_bytes(&mut self, data: &[u8]);
        }
    }

    pub fn new() -> Self {
        Self {
            data: SnapshotChunkData {
                code: "SYMB".into(),
                data: vec![0u8; 4usize]
            }
        }
    }

    pub fn from<C: Into<Code>>(code: C, content: Vec<u8>) -> Self {
        let code = code.into();
        assert_eq!(code[0], b'S');
        assert_eq!(code[1], b'Y');
        assert_eq!(code[2], b'M');
        assert_eq!(code[3], b'B');

        Self {
            data: SnapshotChunkData {
                code,
                data: content
            }
        }
    }

    /// Add a symbol in the chunk. Warning it is cropped to a length of 255
    pub fn add_symbol(&mut self, name: &str, address: u16) {
        // Build the payload for the current symbol
        let len = name.len().min(255);
        let mut bytes: Vec<u8> = vec![0; 1 + len + 6 + 2];

        bytes[0] = len as u8;
        for (idx, b) in name[..len].as_bytes().into_iter().enumerate() {
            bytes[idx + 1] = *b;
        }
        let high = ((address & 0xFF00) >> 8) as u8;
        let low = (address & 0x00FF) as u8;

        bytes[name.len() + 1 + 6 + 0] = high;
        bytes[name.len() + 1 + 6 + 1] = low;

        // add the payload
        self.add_bytes(&bytes);

        // update chunk size
        let payload_size = self.size() - 4;
        self.data.data[0] = (payload_size & 0xFF) as u8;
        self.data.data[1] = ((payload_size >> 8) & 0xFF) as u8;
        self.data.data[2] = ((payload_size >> 16) & 0xFF) as u8;
        self.data.data[3] = ((payload_size >> 24) & 0xFF) as u8;
    }

    pub fn get_symbols(&self) -> Vec<(&str, u16)> {
        let mut res = Vec::new();

        let mut idx = 4;
        while idx < self.size() {
            let count = self.data()[idx] as usize;
            idx += 1;
            let name = &self.data()[idx..(idx + count)];
            idx += count;
            let name = std::str::from_utf8(name).unwrap();
            idx += 6;
            let low = self.data()[idx] as u16;
            idx += 1;
            let high = self.data()[idx] as u16;
            idx += 1;
            let address = low + 256 * high;

            res.push((name, address));
        }

        res
    }
}

#[derive(Clone, Debug)]
pub struct WinapeBreakPointChunk {
    data: SnapshotChunkData
}

impl WinapeBreakPointChunk {
    delegate! {
        to self.data {
            pub fn code(&self) -> &Code;
            pub fn size(&self) -> usize;
            pub fn size_as_array(&self) -> [u8; 4];
            pub fn data(&self) -> &[u8];
            fn add_bytes(&mut self, data: &[u8]);

        }
    }

    pub fn from<C: Into<Code>>(code: C, content: Vec<u8>) -> Self {
        let code = code.into();
        assert_eq!(code[0], b'B');
        assert_eq!(code[1], b'R');
        assert_eq!(code[2], b'K');
        assert_eq!(code[3], b'S');

        Self {
            data: SnapshotChunkData {
                code,
                data: content
            }
        }
    }

    pub fn add_breakpoint_raw(&mut self, raw: &[u8]) {
        assert!(raw.len() == 5);
        self.add_bytes(raw);
    }

    pub fn nb_breakpoints(&self) -> usize {
        self.size() / 5
    }
}

#[derive(Clone, Debug)]
/// Unknwon kind of chunk
pub struct UnknownChunk {
    /// Raw data of the chunk
    data: SnapshotChunkData
}

impl UnknownChunk {
    delegate! {
        to self.data {
        pub fn code(&self) -> &Code;
            pub fn size(&self) -> usize;
            pub fn size_as_array(&self) -> [u8; 4];
            pub fn data(&self) -> &[u8];
        }
    }

    /// Generate the chunk from raw data
    pub fn from<C: Into<Code>>(code: C, data: Vec<u8>) -> Self {
        let code = code.into();
        Self {
            data: SnapshotChunkData { code, data }
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
    AceSymbol(AceSymbolChunk),
    /// The chunk is a memory chunk
    Memory(MemoryChunk),
    /// The chunk is a breakpoint chunk for winape emulator
    WinapeBreakPoint(WinapeBreakPointChunk),
    /// The type of the chunk is unknown
    Unknown(UnknownChunk)
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

        if self.is_memory_chunk() {
            self.memory_chunk().unwrap().print_info();
        }
    }

    pub fn is_memory_chunk(&self) -> bool {
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

    /// Provides the code of the chunk
    pub fn code(&self) -> &Code {
        match self {
            SnapshotChunk::AceSymbol(chunck) => chunck.code(),
            SnapshotChunk::Memory(chunk) => chunk.code(),
            SnapshotChunk::Unknown(chunk) => chunk.code(),
            SnapshotChunk::WinapeBreakPoint(chunk) => chunk.code()
        }
    }

    pub fn size(&self) -> usize {
        match self {
            SnapshotChunk::AceSymbol(chunk) => chunk.size(),
            SnapshotChunk::Memory(chunk) => chunk.size(),
            SnapshotChunk::WinapeBreakPoint(chunk) => chunk.size(),
            SnapshotChunk::Unknown(chunk) => chunk.size()
        }
    }

    pub fn size_as_array(&self) -> [u8; 4] {
        match self {
            SnapshotChunk::AceSymbol(chunk) => chunk.size_as_array(),
            SnapshotChunk::Memory(chunk) => chunk.size_as_array(),
            SnapshotChunk::WinapeBreakPoint(ref chunk) => chunk.size_as_array(),
            SnapshotChunk::Unknown(chunk) => chunk.size_as_array()
        }
    }

    pub fn data(&self) -> &[u8] {
        match self {
            SnapshotChunk::AceSymbol(chunk) => chunk.data(),
            SnapshotChunk::Memory(chunk) => chunk.data(),
            SnapshotChunk::WinapeBreakPoint(chunk) => chunk.data(),
            SnapshotChunk::Unknown(chunk) => chunk.data()
        }
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

impl From<UnknownChunk> for SnapshotChunk {
    fn from(chunk: UnknownChunk) -> Self {
        SnapshotChunk::Unknown(chunk)
    }
}
