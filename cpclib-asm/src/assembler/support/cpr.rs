use cpclib_common::bitvec::vec::BitVec;
use cpclib_common::riff::{RiffChunk, RiffCode};
use cpclib_cpr::{CartridgeBank, Cpr};

use super::banks::DecoratedPages;
use crate::AssemblerError;
use crate::page_info::PageInformation;
use crate::support::banks::Bank;

/// A CPR contains several BANKS
/// However we store PAGES to allow the user to use any ORG value.
/// At the end only 16K will be selected
#[derive(Clone, Debug)]
pub struct CprAssembler {
    pages: DecoratedPages,
    codes: Vec<(u8, String)>
}

impl Default for CprAssembler {
    fn default() -> Self {
        Self {
            pages: DecoratedPages::default(),
            codes: Vec::with_capacity(32)
        }
    }
}

impl TryInto<Cpr> for &CprAssembler {
    type Error = AssemblerError;

    fn try_into(self) -> Result<Cpr, AssemblerError> {
        let mut chunks = Vec::with_capacity(self.codes.len());

        for (code, page) in self.codes.iter().zip(self.pages.pages.iter()) {
            let bank: Bank = page.try_into().map_err(|e: AssemblerError| {
                AssemblerError::AssemblingError {
                    msg: format!("Error when building CPR bloc {}. {}", code.1, e)
                }
            })?;
            let riff_code = RiffCode::from(code.1.as_str());
            let riff = RiffChunk::new(riff_code, bank.into());
            let chunk: CartridgeBank = riff.try_into().unwrap();
            chunks.push(chunk);
        }

        chunks.sort_by_key(|bloc| bloc.code().clone());

        Ok(chunks.into())
    }
}

impl CprAssembler {
    pub fn build_cpr(&self) -> Result<Cpr, AssemblerError> {
        self.try_into()
    }

    pub fn selected_written_bytes(&self) -> Option<&BitVec> {
        self.pages.selected_written_bytes()
    }

    pub fn selected_active_page_info_mut(&mut self) -> Option<&mut PageInformation> {
        self.pages.selected_active_page_info_mut()
    }

    pub fn selected_active_page_info(&self) -> Option<&PageInformation> {
        self.pages.selected_active_page_info()
    }

    pub fn page_infos(&self) -> impl Iterator<Item = &PageInformation> {
        self.pages.page_infos()
    }

    // pub fn selected_bank_mut(&mut self) -> Option<&mut CartridgeBank> {
    // self.selected_index.as_ref()
    // .map(|&idx| self.cpr.bank_mut(idx).unwrap())
    // }
    //
    // pub fn selected_bank(&self) -> Option<& CartridgeBank> {
    // self.selected_index.as_ref()
    // .map(|&idx| self.cpr.bank(idx).unwrap())
    // }

    pub fn select(&mut self, bank_number: u8) {
        if let Some(idx) = self.code_to_index(bank_number) {
            self.pages.selected_index = Some(idx);
        }
        else {
            self.add_bank(bank_number);
            self.pages.selected_index = Some(self.pages.pages.len() - 1);
        }
    }

    fn add_bank(&mut self, bank_number: u8) {
        let code = Self::number_to_code(bank_number);

        assert!(self.code_to_index(bank_number).is_none()); // TODO raise an error
        self.pages.add_new_and_select();
        self.codes.push((bank_number, code.into()));
    }

    fn number_to_code(bank_number: u8) -> &'static str {
        CartridgeBank::code_for(bank_number)
    }

    // assume the code is valid
    // TODO reduce String building if it is too slow
    fn code_to_index(&self, bank_number: u8) -> Option<usize> {
        let code = Self::number_to_code(bank_number);
        self.codes.iter().position(|c| c.1 == code)
    }

    pub fn selected_bloc(&self) -> Option<u8> {
        self.pages.selected_index().map(|idx| self.codes[idx].0)
    }

    pub fn get_byte(&self, address: u16) -> Option<u8> {
        self.pages.get_byte(address)
    }

    /// Write the byte in the page and save this information in written bytes
    pub fn set_byte(&mut self, address: u16, byte: u8) -> Result<(), AssemblerError> {
        // update the page limit to unsure that 16kb is used at max

        if let Some(first) = self.pages.selected_active_page_info().unwrap().startadr {
            let max = (first as u32 + 0x4000).min(0xFFFF) as u16;
            if max > self.pages.selected_active_page_info().unwrap().output_limit {
                dbg!(max, self.pages.selected_active_page_info());
                todo!()
            }
            else {
                self.pages
                    .selected_active_page_info_mut()
                    .unwrap()
                    .set_limit(max)?;
            }
        }

        self.pages.set_byte(address, byte);
        Ok(())
    }

    pub fn reset_written_bytes(&mut self) {
        self.pages.reset_written_bytes();
    }
}

impl CprAssembler {
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }
}
