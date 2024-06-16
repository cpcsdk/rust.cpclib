use std::ops::{Deref, DerefMut};

use cpclib_common::bitvec::vec::BitVec;

use crate::page_info::PageInformation;
use crate::{AssemblerError, MyDefault};

pub(crate) type Bank = [u8; 0x4000];

type Page = [u8; 0x1_0000];
impl MyDefault for Page {
    fn default() -> Page {
        [0; 0x1_0000]
    }
}

#[derive(Clone, Debug)]
pub struct DecoratedPage((Page, PageInformation, BitVec));

impl Deref for DecoratedPage {
    type Target = (Page, PageInformation, BitVec);

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DecoratedPage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for DecoratedPage {
    fn default() -> Self {
        Self((
            Page::default(),
            PageInformation::default(),
            BitVec::repeat(false, 0x4000 * 4)
        ))
    }
}

impl DecoratedPage {
    #[inline(always)]
    fn page(&self) -> &Page {
        &self.0 .0
    }

    #[inline(always)]
    fn page_information(&self) -> &PageInformation {
        &self.0 .1
    }

    #[inline(always)]
    fn written_bytes(&self) -> &BitVec {
        &self.0 .2
    }

    /// Set the byte and store the written address
    pub fn set_byte(&mut self, address: u16, byte: u8) {
        self.0 .0[address as usize] = byte;
        self.0 .2.set(address as _, true);
    }

    pub fn get_byte(&self, address: u16) -> u8 {
        self.0 .0[address as usize]
    }
}

#[derive(Clone, Debug)]
pub struct DecoratedPages {
    pub(crate) pages: Vec<DecoratedPage>,
    pub(crate) selected_index: Option<usize>
}

impl Default for DecoratedPages {
    fn default() -> Self {
        DecoratedPages {
            pages: Vec::with_capacity(0),
            selected_index: None
        }
    }
}

impl TryInto<Bank> for &DecoratedPage {
    type Error = AssemblerError;

    /// Copy the Page to the beginning of the bank, unless it is too huge
    fn try_into(self) -> Result<Bank, Self::Error> {
        let binary_bloc = self.binary_bloc();

        if binary_bloc.len() > 0x4000 {
            return Err(AssemblerError::AssemblingError {
                msg: format!("0x{:X} > 0x4000", binary_bloc.len())
            });
        }

        // get the appropriate bytes and copy them to the beginning
        let mut bank: Bank = [0; 0x4000];
        bank[..binary_bloc.len()].copy_from_slice(binary_bloc);
        Ok(bank)
    }
}

impl DecoratedPage {
    /// REturns the memory bloc of written byte
    pub fn binary_bloc(&self) -> &[u8] {
        if let Some(start) = &self.page_information().startadr {
            let stop = self.page_information().maxadr as usize;
            &self.page()[(*start as usize)..=stop]
        }
        else {
            &self.page()[..0]
        }
    }
}

impl DecoratedPages {
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index.clone()
    }

    pub fn selected_written_bytes(&self) -> Option<&BitVec> {
        self.selected_index.as_ref().map(|&idx| &self.pages[idx].2)
    }

    pub fn selected_active_page_info_mut(&mut self) -> Option<&mut PageInformation> {
        self.selected_index
            .as_ref()
            .map(|&idx| &mut self.pages[idx].1)
    }

    pub fn selected_active_page_info(&self) -> Option<&PageInformation> {
        self.selected_index.as_ref().map(|&idx| &self.pages[idx].1)
    }

    pub fn page_infos(&self) -> impl Iterator<Item = &PageInformation> {
        self.pages.iter().map(|d| &d.1)
    }
}

impl DecoratedPages {
    pub fn add_new_and_select(&mut self) {
        self.selected_index = Some(self.pages.len());
        self.pages.push(DecoratedPage::default());
    }

    pub fn select_next(&mut self) -> Result<(), AssemblerError> {
        self.selected_index = self.selected_index.map(|v| v + 1).or(Some(0));

        if *self.selected_index.as_ref().unwrap() >= self.pages.len() {
            Err(AssemblerError::AssemblingError {
                msg: "There were less banks in previous pass".to_owned()
            })
        }
        else {
            Ok(())
        }
    }

    /// Write the byte in the page and save this information in written bytes
    pub fn set_byte(&mut self, address: u16, byte: u8) {
        if let Some(idx) = &self.selected_index {
            self.pages[*idx].set_byte(address, byte);
        }
        else {
            todo!()
        }
    }

    pub fn get_byte(&self, address: u16) -> Option<u8> {
        self.selected_index
            .as_ref()
            .map(|&idx| self.pages[idx].get_byte(address))
    }

    pub fn reset_written_bytes(&mut self) {
        self.pages.iter_mut().for_each(|p| p.2.fill(false));
    }
}

impl DecoratedPages {
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }
}
