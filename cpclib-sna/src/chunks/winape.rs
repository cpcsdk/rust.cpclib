use std::ops::Deref;

use cpclib_common::riff::{RiffBlock, RiffCode, RiffLen};
use delegate::delegate;


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
    riff: RiffBlock
}


impl Deref for WinapeBreakPointChunk {
    type Target= RiffBlock;

    fn deref(&self) -> &Self::Target {
        &self.riff
    }
}

impl WinapeBreakPointChunk {
    const CODE: RiffCode = RiffCode::new([b'B', b'R', b'K', b'S']);

    delegate! {
        to self.riff {
            pub fn code(&self) -> &RiffCode;
            pub fn len(&self) -> &RiffLen;
            pub fn data(&self) -> &[u8];
            fn add_bytes(&mut self, data: &[u8]);

        }
    }

    pub fn empty() -> Self {
        Self::from(Self::CODE, Vec::new())
    }

    pub fn from<C: Into<RiffCode>>(code: C, content: Vec<u8>) -> Self {
        let code = code.into();
        assert_eq!(code, Self::CODE);

        Self {
            riff: RiffBlock::new(
                code,
                content
            )
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
        (self.len().value() / 5)  as usize 
    }
}
