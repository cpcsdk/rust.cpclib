use delegate::delegate;

use crate::{Code, SnapshotChunkData};

pub struct WinapeBreakPoint {
    buffer: [u8; 5]
}

impl WinapeBreakPoint {
    pub fn new(address: u16, page: u8) -> Self {
        let mut buffer = [0; 5];
        buffer[0] = (address & 0xFF) as u8;
        buffer[1] = (address >> 8) as u8;
        buffer[2] = page;
        Self { buffer }
    }
}

#[derive(Clone, Debug)]
pub struct WinapeBreakPointChunk {
    data: SnapshotChunkData
}

impl WinapeBreakPointChunk {
    const CODE: Code = Code([b'B', b'R', b'K', b'S']);

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
        Self::from(Self::CODE, Vec::new())
    }

    pub fn from<C: Into<Code>>(code: C, content: Vec<u8>) -> Self {
        let code = code.into();
        assert_eq!(code, Self::CODE);

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

    pub fn add_breakpoint(&mut self, brk: WinapeBreakPoint) {
        self.add_breakpoint_raw(&brk.buffer)
    }

    pub fn nb_breakpoints(&self) -> usize {
        self.size() / 5
    }
}
