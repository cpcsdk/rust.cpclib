use std::ops::{Deref, DerefMut};

use cpclib_common::bitvec::vec::BitVec;
use cpclib_sna::Snapshot;

use crate::page_info::PageInformation;

#[derive(Clone)]
pub(crate) struct SnaAssembler {
    pub(crate) sna: Snapshot,
    pub(crate) pages_info: Vec<PageInformation>,
    pub written_bytes: BitVec
}

impl Default for SnaAssembler {
    fn default() -> Self {
        let mut sna = Snapshot::default(); // Snapshot::new_6128().unwrap();
        sna.unwrap_memory_chunks();

        let pages_info = vec![Default::default(); 2];
        let written_bytes = BitVec::repeat(false, 0x4000 * 2 * 4);

        SnaAssembler {
            sna,
            pages_info,
            written_bytes
        }
    }
}

impl Deref for SnaAssembler {
    type Target = Snapshot;

    fn deref(&self) -> &Self::Target {
        &self.sna
    }
}

impl DerefMut for SnaAssembler {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sna
    }
}

impl SnaAssembler {
    pub fn resize(&mut self, expected_nb: usize) {
        self.pages_info.resize(expected_nb, Default::default());
        self.written_bytes.resize(expected_nb * 0x1_0000, false);
    }
}

impl SnaAssembler {
    /// Write the byte in the snapshot  and save this information in written bytes
    pub fn set_byte(&mut self, address: u32, byte: u8) {
        self.deref_mut().set_byte(address, byte);
        self.written_bytes.set(address as _, true);
    }

    pub fn reset_written_bytes(&mut self) {
        self.written_bytes.fill(false);
    }
}
