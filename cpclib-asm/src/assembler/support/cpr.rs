use std::ops::Deref;

use crate::page_info::PageInformation;
use cpclib_common::bitvec::vec::BitVec;
use cpclib_cpr::{CartridgeBank, Cpr};

#[derive(Clone)]
pub struct CprAssembler {
	cpr: Cpr,
	pub(crate) pages_info: Vec<PageInformation>,
	written_bytes: Vec<BitVec>,
	selected_index: Option<usize>,
	base_address: Vec<u16>,

}


impl Default for CprAssembler {
	fn default() -> Self {

		let mut cpr = Self {
			cpr: Default::default(),
			pages_info: Default::default(),
			written_bytes: Default::default(),
			selected_index: Default::default(),
			base_address: Default::default()
		};
		cpr.add_bank(0);
		cpr
	}
}


impl Deref for CprAssembler {
	type Target = Cpr;

	fn deref(&self) -> &Self::Target {
		&self.cpr
	}
}


impl CprAssembler {

	pub(crate) fn set_base_address(&mut self, address: u16) {
		self.selected_index.as_ref()
			.map(|&idx| self.base_address[idx as usize] = address);
	}


	pub(crate) fn base_address(&self) -> Option<u16> {
		self.selected_index.as_ref()
			.map(|&idx| self.base_address[idx as usize])
	}


	pub fn selected_written_bytes_mut(&mut self) -> Option<&mut BitVec> {
		self.selected_index.as_ref()
			.map(|&idx| &mut self.written_bytes[idx])
	}

	pub fn selected_written_bytes(& self) -> Option<& BitVec> {
		self.selected_index.as_ref()
			.map(|&idx| & self.written_bytes[idx])
	}


	pub fn selected_active_page_info_mut(&mut self) -> Option<&mut PageInformation> {
		self.selected_index.as_ref()
			.map(|&idx| &mut self.pages_info[idx])		
	}

	pub fn selected_active_page_info(& self) -> Option<& PageInformation> {
		self.selected_index.as_ref()
			.map(|&idx| &self.pages_info[idx])		
	}

	pub fn selected_bank_mut(&mut self) -> Option<&mut CartridgeBank> {
		self.selected_index.as_ref()
			.map(|&idx| self.cpr.bank_mut(idx).unwrap())		
	}

	pub fn selected_bank(&self) -> Option<& CartridgeBank> {
		self.selected_index.as_ref()
			.map(|&idx| self.cpr.bank(idx).unwrap())		
	}

	pub fn select(&mut self, bank_number: u8) {
		if let Some(idx) = self.code_to_index(bank_number) {
			self.selected_index = Some(idx);
		} else {
			self.add_bank(bank_number);
			self.selected_index = Some(self.pages_info.len() - 1);
		}
	}

	fn add_bank(&mut self, bank_number: u8) {
		// TODO check if the bank is already preset and crash otherwhise
		self.cpr.add_bank(CartridgeBank::new(bank_number));
		self.pages_info.push(PageInformation::default());
		self.written_bytes.push(BitVec::repeat(false, 0x4000));
		self.base_address.push(0);
		self.selected_index = Some(self.pages_info.len() -1);
	}

	fn number_to_code(bank_number: u8) -> String {
		CartridgeBank::code_for(bank_number)
	}

	// assume the code is valid
	// TODO reduce String building if it is too slow
	fn code_to_index(&self, bank_number: u8) -> Option<usize> {
		let code = Self::number_to_code(bank_number);
		self.cpr.banks().iter().map(|b| b.code())
			.position(|c| c.to_string() == code)
	}

	pub fn get_byte(&self, address: u16) -> Option<u8> {
		self.selected_bank()
			.map(|b| b.data()[address as usize])
	}

	pub fn set_byte(&mut self, address: u16, byte: u8) {
		if self.selected_index.is_some() {
			let bank = self.selected_bank_mut().unwrap();
			bank.set_byte(address, byte);
		} else {
			todo!()
		}
	}
}