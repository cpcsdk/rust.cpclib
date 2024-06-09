use cpclib_common::bitvec::vec::BitVec;

use crate::{page_info::PageInformation, AssemblerError, MyDefault};


/// ! Lots of things will probably be inspired from RASM
type Page = [u8; 0x1_0000];
impl MyDefault for Page {
    fn default() -> Page {
        [0; 0x1_0000]
    }
}

#[derive(Clone, Debug)]
pub struct Pages{
	pub(crate) pages: Vec<(Page, PageInformation, BitVec)>,
	pub(crate) selected_index: Option<usize>
}

impl Default for Pages {
	fn default() -> Self {
		Pages {
			pages: Vec::with_capacity(0),
			selected_index: None
		}
	}	
}

impl Pages {
	pub fn selected_written_bytes_mut(&mut self) -> Option<&mut BitVec> {
		self.selected_index.as_ref()
			.map(|&idx| &mut self.pages[idx].2)
	}

	pub fn selected_written_bytes(& self) -> Option<& BitVec> {
		self.selected_index.as_ref()
			.map(|&idx| & self.pages[idx].2)
	}


	pub fn selected_active_page_info_mut(&mut self) -> Option<&mut PageInformation> {
		self.selected_index.as_ref()
			.map(|&idx| &mut self.pages[idx].1)		
	}

	pub fn selected_active_page_info(& self) -> Option<& PageInformation> {
		self.selected_index.as_ref()
			.map(|&idx| & self.pages[idx].1)
	}
}


impl Pages {
	pub fn add_new_and_select(&mut self) {
		self.selected_index = Some(self.pages.len());
		self.pages.push((
			Page::default(),
			PageInformation::default(),
			BitVec::repeat(false, 0x4000 * 4)
		));
	}

	pub fn select_next(&mut self) -> Result<(), AssemblerError> {
		self.selected_index = self.selected_index.map(|v| v + 1).or(Some(0));

		if *self.selected_index.as_ref().unwrap() >= self.pages.len() {
			Err(AssemblerError::AssemblingError {
				msg: "There were less banks in previous pass".to_owned()
			})
		} else {
			Ok(())
		}
	}

	pub fn set_byte(&mut self, address: u16, byte: u8) {
		if let Some(idx) = &self.selected_index {
			self.pages[*idx].0[address as usize] = byte;
		} else {
			todo!()
		}
	}

	pub fn get_byte(& self, address: u16) -> Option<u8> {
		self.selected_index.as_ref()
			.map(|&idx|
				self.pages[idx].0[address as usize]
			)
	}
}