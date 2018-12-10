use crate::disc::edsk::ExtendedDsk;


use arrayref;
use bitfield::Bit;
use slice_of_array::prelude::*;

use std::iter::Iterator;

pub struct AmsdosManager {
	disc: ExtendedDsk,
	side: u8
}


#[derive(Debug)]
pub struct AmsdosFileName {
	name: String,
	extension: String
}


pub enum AmsdosFileType {
	Basic = 0,
	Protected = 1,
	Binary = 2
}

impl AmsdosFileName {
	pub fn from_entry_format(buffer: &[u8;11]) -> AmsdosFileName {
		let name = &buffer[..8];
		// Remove bit 7 of each char
		let extension = buffer[8..].iter().map(|&c|{c&0b01111111}).collect::<Vec<_>>();

		AmsdosFileName {
			name: String::from_utf8_lossy(name).trim().to_owned(),
			extension: String::from_utf8_lossy(&extension).trim().to_owned()
		}
	}
}

// http://www.cpc-power.com/cpcarchives/index.php?page=articles&num=92
#[derive(Debug)]
pub struct AmsdosEntry {
	user : u8,
	file_name : AmsdosFileName,
	read_only: bool,
	system: bool,
	nb_entries: u8,
	nb_blocs: u8,
	blocs: [u8;16]
}

impl AmsdosEntry {
	pub fn from_buffer(buffer: &[u8;32]) -> AmsdosEntry  {
		AmsdosEntry {
			user: buffer[0],
			file_name: AmsdosFileName::from_entry_format(
				array_ref!(buffer, 1+1,1+8+3-1)
			),
			read_only: buffer[1+8+0].bit(7),
			system: buffer[1+8+1].bit(7),
			nb_entries: buffer[12],
			nb_blocs: buffer[15],
			blocs: buffer[16..].to_array()

		}
	}

	pub fn is_erased(&self) -> bool {
		self.user == 0xe5
	}

	pub fn is_read_only(&self) -> bool {
		self.read_only
	}

	pub fn is_system(&self) -> bool {
		self.system
	}

	pub fn format(&self) -> String {
		format!(
			"{}:{}.{}",
			self.user,
			self.file_name.name,
			self.file_name.extension
		)
	} 
}


pub struct AmsdosEntries {
	entries: Vec<AmsdosEntry>	
}


impl AmsdosEntries {

	/// Returns all the entries of the catalog
	pub fn all_entries(&self) -> impl Iterator<Item = &AmsdosEntry> {
		self.entries.iter()
	}

	/// Returns an iterator on the visible entries : they are not erased and are not system entries
	pub fn visible_entries(&self) -> impl Iterator<Item = &AmsdosEntry> {
		self.entries.iter()
			.filter(|&entry|{
				!entry.is_erased() && !entry.is_system()
			})
	}
}

impl AmsdosManager {
	pub fn new(disc: ExtendedDsk, side: u8) -> AmsdosManager {
		AmsdosManager {
			disc,
			side
		}
	}


	/// Return the entries of the Amsdos catalog
	pub fn catalog(&self) -> AmsdosEntries {
		let mut entries = Vec::new();
		let bytes = self.disc.sectors_bytes(0, 0xc1, 4, self.side).unwrap();
		
		for idx in 0..64/*(bytes.len() / 32)*/ {
			let entry_buffer=&bytes[(idx*32)..(idx+1)*32];
			let entry = AmsdosEntry::from_buffer(
				array_ref!(entry_buffer, 0, 32)
			);
			entries.push(entry);
		}

		AmsdosEntries {
			entries
		}
	}

	pub fn print_catalog(&self) {
		let entries = self.catalog();
		for entry in entries.visible_entries() {
			if !entry.is_erased() && !entry.is_system() {
				println!("{}", entry.format());
			}
		}
	}
}