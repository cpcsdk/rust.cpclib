// Implement WABP chunk (and file content) for WINAPE
// this has not been yet deeply tested. Consider everything is false

use std::ops::Deref;

use cpclib_common::riff::{RiffChunk, RiffCode, RiffLen};
use delegate::delegate;



fn push_u32(vec: &mut Vec<u8>, mut nb: u32) {
    for _ in 0..4 {
        vec.push( (nb%256) as _);
        nb >>= 8;
    }
    assert_eq!(nb, 0);
}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum BreakpointOrigin {
    User = 1,
    Assembler = 4
}


#[repr(u8)]
#[derive(Clone, Copy)]
pub enum BreakPointAccess {
    Read = 1<<6,
    Write = 1<<7,
    ReadWrite = (1<<7) + (1<<6)
}

// TODO finish to get the other ones
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum BreakpointIoType {
    User = 0,
    GateArray = 1,

}

#[derive(Clone, Copy)]
pub struct BreakpointFlag(u8);

pub struct Counts {
    max: u32,
    count: u32
}

impl Counts {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::with_capacity(4);
        push_u32(&mut vec, self.max);
        push_u32(&mut vec, self.count);

        vec
    }
}

pub struct Condition(String);

impl Condition {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::new();
        let condition = self.0.as_bytes();
        let nb_chars = condition.len() as u32;
        push_u32(&mut vec, nb_chars);
        for c in condition {
            vec.push(*c);
        }
        vec
    }
}

pub struct CountsAndCondition(Option<Counts>, Option<Condition>);


impl CountsAndCondition {
    pub fn none() -> Self {
        CountsAndCondition(None, None)
    }
    pub fn has_counts(&self) -> bool {
        self.0.is_some()
    }

    pub fn has_condition(&self) -> bool {
        self.1.is_some()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::new();

        if let Some(counts) = &self.0 {
            vec.extend_from_slice(&counts.to_bytes());
        }

        if let Some(condition) = &self.1 {
            vec.extend_from_slice(&condition.to_bytes());
        }
        vec
    }
}

pub struct WinapeAddress(u32);
impl Deref for WinapeAddress {
    type Target= u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl WinapeAddress {
    pub fn new(address: u16, has_count: bool, has_condition: bool) -> WinapeAddress {
        let mut addr = address as u32;
        if has_count {
            addr += 1<<31;
        } 
        if has_condition {
            addr += 1<<30;
        }

        Self(addr)
    }

    pub fn address(&self) -> u16 {
        (self.0 & 0xffff) as u16
    }

    pub fn has_count(&self) -> bool {
        (self.0 & 1<<31) != 0
    }

    pub fn has_condition(&self) -> bool {
        (self.0 & 1<<30) != 0
    }
}

pub struct CodeBreakpoint {
    origin: BreakpointOrigin,
    address: u16,
    counts_condition: CountsAndCondition
}

impl CodeBreakpoint {
    pub fn new(address: u16) -> Self {
        Self {
            origin: BreakpointOrigin::Assembler,
            address,
            counts_condition: CountsAndCondition::none()
        }
    }


    pub fn winape_address(&self) -> WinapeAddress {
        WinapeAddress::new(
            self.address, 
            self.counts_condition.has_counts(), 
            self.counts_condition.has_counts())
    }
}

impl CodeBreakpoint {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::new();

        vec.push(self.origin as u8);
        push_u32(&mut vec, *self.winape_address().deref());
        vec.extend_from_slice(&self.counts_condition.to_bytes());

        vec
    }
}

pub struct MemoryBreakpoint {
    access: BreakPointAccess,
    address: u16,
    counts_condition: CountsAndCondition
}


impl MemoryBreakpoint {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::new();

        vec.push(self.access as u8);
        push_u32(&mut vec, *self.winape_address().deref());
        vec.extend_from_slice(&self.counts_condition.to_bytes());
        vec.push(0);
        vec.push(0);
        vec
    }

    pub fn winape_address(&self) -> WinapeAddress {
        WinapeAddress::new(
            self.address, 
            self.counts_condition.has_counts(), 
            self.counts_condition.has_counts())
    }
}
pub struct IOBreakpoint {
    r#type: BreakpointIoType,
    flag: BreakpointFlag,
    address: u16,
    address_mask: u16,
    counts_condition: CountsAndCondition
}


impl IOBreakpoint {
    pub fn winape_address(&self) -> WinapeAddress {
        WinapeAddress::new(
            self.address, 
            self.counts_condition.has_counts(), 
            self.counts_condition.has_counts())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        unimplemented!()
    }
}

pub struct  CodeBreakpoints(Vec<CodeBreakpoint>);
pub struct MemoryBreakpoints(Vec<MemoryBreakpoint>);
pub struct IOBreakpoints(Vec<IOBreakpoint>);

pub enum WabpAnyBreakpoint {
    Code(CodeBreakpoint),
    Memory(MemoryBreakpoint),
    IO(IOBreakpoint)
}

impl WabpAnyBreakpoint {
    pub fn new(address: u16) -> Self {
        WabpAnyBreakpoint::Code(CodeBreakpoint::new(address))
    }
}


impl CodeBreakpoints {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::new();

        // Store the number of breakpoints
        let nb = self.0.len();
        push_u32(&mut vec, nb as _);

        for brk in self.0.iter() {
            vec.extend_from_slice(&brk.to_bytes());
        }

        vec
    }
}


impl MemoryBreakpoints {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::new();

        // Store the number of breakpoints
        let nb = self.0.len();
        push_u32(&mut vec, nb as _);

        for brk in self.0.iter() {
            vec.extend_from_slice(&brk.to_bytes());
        }

        vec
    }
}

impl IOBreakpoints {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::new();

        // Store the number of breakpoints
        let nb = self.0.len();
        push_u32(&mut vec, nb as _);

        for brk in self.0.iter() {
            vec.extend_from_slice(&brk.to_bytes());
        }

        vec
    }
}

pub struct Wabp {
    code: CodeBreakpoints,
    memory: MemoryBreakpoints,
    io:  IOBreakpoints
}

impl Default for Wabp {
    fn default() -> Self {
        Self::new()
    }
}

impl Wabp {
    pub fn new() -> Self {
        Wabp { 
            code: CodeBreakpoints(Default::default()), 
            memory: MemoryBreakpoints(Default::default()), 
            io: IOBreakpoints(Default::default()) 
        }
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::new();
        vec.extend_from_slice(&self.code.to_bytes());
        vec.extend_from_slice(&self.memory.to_bytes());
        vec.extend_from_slice(&self.io.to_bytes());
        vec
    }
}

impl Wabp {
    pub fn add_breakpoint(&mut self, brk: WabpAnyBreakpoint) {
        match brk {
            WabpAnyBreakpoint::Code(brk) => {
                self.code.0.push(brk);
            },
            WabpAnyBreakpoint::Memory(brk) => {
                self.memory.0.push(brk);
            },
            WabpAnyBreakpoint::IO(brk) => {
                self.io.0.push(brk)
            }
        }
    }

}

pub struct WabpChunk {
    riff: RiffChunk,
    wabp: Wabp
}

impl WabpChunk {
    const CODE: RiffCode = RiffCode::new([b'W', b'A', b'B', b'P']);


    pub fn empty() -> Self {
        Self{
            riff: RiffChunk::new(Self::CODE, Default::default()),
            wabp: Wabp::new()
        }
    }

    pub fn add_breakpoint(&mut self, brk: WabpAnyBreakpoint) {
        self.wabp.add_breakpoint(brk);
        let mut data = self.wabp.to_bytes();
        self.riff = RiffChunk::from_buffer(&mut data);
    }

    delegate! {
        to self.riff {
            pub fn code(&self) -> &RiffCode;
            pub fn len(&self) -> &RiffLen;
            pub fn data(&self) -> &[u8];
            fn add_bytes(&mut self, data: &[u8]);
        }
    }
}