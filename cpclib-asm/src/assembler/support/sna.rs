use std::ops::{Deref, DerefMut};

use cpclib_common::bitvec::vec::BitVec;
use cpclib_sna::Snapshot;

use crate::page_info::PageInformation;

#[derive(Clone)]
pub(crate) struct SnaAssembler {
    pub(crate) sna: Snapshot,
    pub pages_info: Vec<PageInformation>,
    pub written_bytes: BitVec
}

impl Default for SnaAssembler {
    fn default() -> Self {
        let mut sna = Snapshot::default(); // Snapshot::new_6128().unwrap();
        sna.unwrap_memory_chunks();

        let nb_pages = (sna.memory_size_header() / 64) as usize;

        let pages_info = vec![Default::default(); nb_pages];
        let written_bytes = BitVec::repeat(false, 0x4000 * 4 * nb_pages);

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
    pub fn resize(&mut self, nb_pages: usize) {
        self.pages_info.resize(nb_pages, Default::default());
        self.written_bytes.resize(nb_pages * 0x1_0000, false);
        self.sna.resize(nb_pages);

        debug_assert_eq!(nb_pages, self.pages_info.len());
        debug_assert_eq!(self.sna.nb_pages(), self.pages_info.len());
    }

    #[inline]
    pub fn page_info_mut(&mut self, page: u8) -> &mut PageInformation {
        &mut self.pages_info[page as usize]
    }

    #[inline]
    pub fn page_info(& self, page: u8) -> &PageInformation {
        & self.pages_info[page as usize]
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
