use crate::disc::edsk::ExtendedDsk;


use arrayref;
use bitfield::Bit;
use slice_of_array::prelude::*;

use std::iter::Iterator;



/// The AmsdosFileName structure is used to encode several informations
/// - the user
/// - the filename (up to 8 chars)
/// - the extension (up to 3 chars)
pub struct AmsdosFileName {
	user: u8,
	name: String,
	extension: String
}

impl std::fmt::Debug for AmsdosFileName {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}:{}.{}", self.user(), self.name, self.extension)
	}
}

impl AmsdosFileName {
	pub fn filename_header_format(&self) -> [u8;8] {
		let mut content = [' ' as u8;8];
		for (i, c) in self.name.as_bytes().iter().enumerate() {
			content[i] = *c;
		}
		content
	}

	pub fn extension_header_format(&self) -> [u8;3] {
		let mut content = [' ' as u8;3];
		for (i, c) in self.extension.as_bytes().iter().enumerate() {
			content[i] = *c;
		}
		content
	}

	pub fn user(&self) -> u8 {
		self.user
	}

	pub fn new(user: u8, filename: &str, extension: &str) -> Result<AmsdosFileName, String> {

		let filename = filename.trim();
		let extension = extension.trim();

		// TODO check the user validity
		if filename.len() > 8 {
			return Err(format!("{} should use at most 8 chars", filename));
		}

		if extension.len() > 3 {
			return Err(format!("{} should use at most 3 chars", extension));
		}

		if filename.to_ascii_uppercase() != filename.to_ascii_uppercase() {
			return Err(format!("{} contains non ascii characters", filename));
		}

		if extension.to_ascii_uppercase() != extension.to_ascii_uppercase() {
			return Err(format!("{} contains non ascii characters", extension));
		}

		//TODO see if upercase is needed
		Ok(AmsdosFileName{
			user,
			name: filename.to_owned()/*.to_uppercase()*/,
			extension: extension.to_owned()/*.to_uppercase()*/,
		})

	}
}

pub enum AmsdosFileType {
	Basic = 0,
	Protected = 1,
	Binary = 2
}

impl From<u8> for AmsdosFileType {
	fn from(val: u8) -> AmsdosFileType {
		match val {
			0 => AmsdosFileType::Basic,
			1 => AmsdosFileType::Protected,
			2 => AmsdosFileType::Binary,
			_ => unreachable!()
		}
	}
}

impl std::fmt::Debug for AmsdosFileType{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let repr = match self {
			AmsdosFileType::Basic => "Basic",
			AmsdosFileType::Protected => "Protected",
			AmsdosFileType::Binary => "Binary"
		};
		write!(f, "{}", repr)
	}
}

impl AmsdosFileName {
	pub fn from_entry_format(buffer: &[u8;12]) -> AmsdosFileName {
		let user = buffer[0];
		let name = &buffer[1..9];
		// Remove bit 7 of each char
		let extension = buffer[9..].iter().map(|&c|{c&0b01111111}).collect::<Vec<_>>();

		AmsdosFileName {
			user,
			name: String::from_utf8_lossy(name).trim().to_owned(),
			extension: String::from_utf8_lossy(&extension).trim().to_owned()
		}
	}
}

// http://www.cpc-power.com/cpcarchives/index.php?page=articles&num=92
#[derive(Debug)]
pub struct AmsdosEntry {
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
			file_name: AmsdosFileName::from_entry_format(
				array_ref!(buffer, 1,1+8+3)
			),
			read_only: buffer[1+8+0].bit(7),
			system: buffer[1+8+1].bit(7),
			nb_entries: buffer[12],
			nb_blocs: buffer[15],
			blocs: buffer[16..].to_array()
		}
	}

	pub fn is_erased(&self) -> bool {
		self.file_name.user() == 0xe5
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
			self.file_name.user,
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

	/// Returns an iterator on the entries not erased
	pub fn without_erased_entries(&self) -> impl Iterator<Item = &AmsdosEntry> {
		self.entries.iter()
			.filter(|&entry|{
				!entry.is_erased()
			})
	}

}

const DIRECTORY_SIZE:usize = 64;
const DATA_SECTORS:[u8;9] = [0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9];
const DATA_NB_RECORDS_PER_TRACK:u8 = 36;
const DATA_BLOCK_SHIFT:u8 = 3;
const DATA_BLOCK_MASK:u8 = 7;
const DATA_EXTENT_MASK:u8 = 0;
const DATA_NB_BLOCKS:u8 = 180;
const DATA_TWO_DIRECTORY_BLOCKS: u8 = 0x00c0;
const DATA_SIZE_OF_CHECKSUM_VECTOR:u8 = 16;
const DATA_RESERVED_TRACK: u8 = 0;
const DATA_FIRST_SECTOR_NUMBER: u8 = DATA_SECTORS[0];
const DATA_SECTORS_PER_TRACK: u8 = 9;
const DATA_GAP_LENGTH_READ_WRITE : u8 = 42;
const DATA_GAP_LENGTH_FORMAT : u8 = 82;
const DATA_FILLER_BYTE: u8 = 0xe9;
const DATA_LOG2_SECTOR_SIZE_MINUS_SEVEN: u8 = 2;
const DATA_RECORDS_PER_TRACK:u8 = 4;


/// http://cpctech.cpc-live.com/docs/manual/s968se09.pdf
/// Current implementatin only focus on DATA format
/// 
pub struct AmsdosManager {
	disc: ExtendedDsk,
	side: u8
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
		let bytes = self.disc.sectors_bytes(
			0, 
			DATA_FIRST_SECTOR_NUMBER, 
			4, 
			self.side).unwrap();
		
		for idx in 0..DIRECTORY_SIZE/*(bytes.len() / 32)*/ {
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

    /// Print the catalog on screen
	pub fn print_catalog(&self) {
		let entries = self.catalog();
		for entry in entries.visible_entries() {
			if !entry.is_erased() && !entry.is_system() {
				println!("{}", entry.format());
			}
		}
	}

	/// Returns the list of used blocs by file (except erased files)
	pub fn used_blocs(&self) -> std::collections::HashSet<u8> {
		self.catalog()
			.without_erased_entries()
			.flat_map(|e|{
				&e.blocs[..(e.nb_blocs as usize)]
			})
			.map(|&b|{
				b
			})
			.collect()
	}

	/// Add a file to the disc
	pub fn add_content(&mut self, data: &[u8]) {

	}

	pub fn compute_basic_header(filename: &AmsdosFileName, data: &[u8]) -> AmsdosHeader {
		AmsdosHeader::build_header(
			filename, 
			AmsdosFileType::Basic,
			0x0000, 
			0x0000, 
			data)
	}

	pub fn compute_binary_header(filename: &AmsdosFileName, loading_address: u16, execution_address: u16, data: &[u8]) -> AmsdosHeader {
		AmsdosHeader::build_header(
			filename, 
			AmsdosFileType::Binary,
			loading_address, 
			execution_address, 
			data)
	}
}





/// http://www.cpcwiki.eu/index.php/AMSDOS_Header
pub struct AmsdosHeader {
	content: [u8; 128]
}

impl std::fmt::Debug for AmsdosHeader {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		writeln!(f, "File: {:?}", self.amsdos_filename())?;
		writeln!(f, "Type: {:?}", self.file_type())?;
		writeln!(f, "Size 0x{:x}", self.file_length())?;
		writeln!(f, "Loading address 0x{:x}", self.loading_address())?;
		writeln!(f, "Execution address 0x{:x}", self.execution_address())?;
		writeln!(f, "Checksum 0x{:x}", self.checksum())?;
		Ok(())
	}
}

impl AmsdosHeader {
	/// XXX currently untested
	pub fn build_header(
		filename: &AmsdosFileName,
		file_type: AmsdosFileType,
		loading_address: u16,
		execution_address: u16,
		data: &[u8]) -> AmsdosHeader {

		let mut content = AmsdosHeader {
			content: [0 as u8; 128]
		};

		content.set_amsdos_filename(filename);
		content.set_file_type(file_type);
		content.set_loading_address(loading_address);
		content.set_file_length(data.len() as u16);
		content.set_execution_address(execution_address);
		content.update_checksum();

		content
	}

	pub fn set_user(&mut self, user: u8) -> &mut Self {
		self.content[0] = user;
		self
	}

	pub fn user(&self) -> u8 {
		self.content[0]
	}

	pub fn set_amsdos_filename(&mut self, filename: &AmsdosFileName) -> &mut Self {
		self.set_user(filename.user());
		self.set_filename(filename.filename_header_format());
		self.set_extension(filename.extension_header_format());
		self
	}

	pub fn amsdos_filename(&self) -> AmsdosFileName {
		AmsdosFileName::new(
			self.user(),
			&self.filename(),
			&self.extension()
		).unwrap()
	}


	pub fn set_filename(&mut self, filename: [u8; 8])  -> &mut Self{
		for (idx, val) in filename.iter().enumerate() {
			self.content[1+idx]	= *val;
		}
		self
	}

	pub fn filename(&self) -> String {
		self.content[1..9].iter()
			.filter(|&&c|{ c != ' ' as u8})
			.map(|&c|{ c as char})
			.collect::<String>()
	}

	pub fn set_extension(&mut self, extension: [u8; 3]) -> &mut Self{
		for (idx, val) in extension.iter().enumerate() {
			self.content[9+idx]	= *val;
		}
		self
	}

	pub fn extension(&self) -> String {
		self.content[9..(9+3)].iter()
			.filter(|&&c|{ c != ' ' as u8})
			.map(|&c|{ c as char})
			.collect::<String>()
	}

	pub fn set_file_type(&mut self, file_type: AmsdosFileType) -> &mut Self{
		self.content[18] = file_type as u8;
		self
	}

	pub fn file_type(&self) -> AmsdosFileType {
		self.content[18].into()
	}

	pub fn set_loading_address(&mut self, address: u16) -> &mut Self {
		self.set_16bits_value(address, 21)
	}

	pub fn loading_address(&self) -> u16 {
		self.get_16bits_value(21)
	}

	pub fn set_file_length(&mut self, length: u16) -> &mut Self {
		self.set_16bits_value(length, 24);
		self.content[64] = self.content[24];
		self.content[65] = self.content[25];
		self.content[66] = 0;
		self
	}

	pub fn file_length(&self) -> u16 {
		self.get_16bits_value(24)
	}

	pub fn file_length2(&self) -> u16 {
		self.get_16bits_value(64)
	}


	pub fn set_execution_address(&mut self, execution_address: u16) -> &mut Self {
		self.set_16bits_value(execution_address, 26)
	}

	pub fn execution_address(&self) -> u16 {
		self.get_16bits_value(26)
	}

	fn set_16bits_value(&mut self, value: u16, at: usize) -> &mut Self {
		self.content[at+0] = (value% 256) as u8;
		self.content[at+1] = (value/ 256) as u8;
		self
	}

	fn get_16bits_value(&self,  at: usize) -> u16 {
		let low = self.content[at+0] as u16;
		let high = self.content[at+1] as u16;

		256*high + low
	}

	pub fn checksum(&self) -> u16 {
		self.get_16bits_value(67)
	}

	pub fn update_checksum(&mut self) -> &mut Self {
		let checksum = self.compute_checksum();
		self.set_16bits_value(checksum, 67);
		self
	}

	pub fn compute_checksum(&self) -> u16 {
		self.content[0..=66].iter().map(|&b|{b as u16}).sum::<u16>()
	}

	pub fn as_bytes(&self) -> &[u8; 128] {
		&self.content
	}
}




/// Encode an amsdos file. 
/// Warning content may be larger than the real size of the file. It is up to the user to remove the extra_space
pub struct AmsdosFile {
	header: AmsdosHeader,
	content: Vec<u8>
}


impl AmsdosFile {

	pub fn binary_file_from_buffer(
		filename: &AmsdosFileName, 
		loading_address: u16, 
		execution_address: u16, 
		data: &[u8]) -> AmsdosFile {

		let header = AmsdosHeader::build_header(
			filename, 
			AmsdosFileType::Binary,
			loading_address, 
			execution_address, 
			data);
		let content = data.to_vec();

		AmsdosFile {
			header,
			content
		}
	}

	/// Return an iterator on the full content of the file: header + content + extra  bytes
	pub fn full_content(&self) -> impl Iterator<Item=&u8> {
		self.header.as_bytes().iter()
			.chain(self.content.iter())
	}
}