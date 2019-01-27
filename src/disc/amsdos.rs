use crate::disc::edsk::ExtendedDsk;


use arrayref;
use bitfield::Bit;
use slice_of_array::prelude::*;

use std::iter::Iterator;
use crate::disc::edsk::Side;

#[derive(Debug)]
pub enum AmsdosError {
	NoEntriesAvailable,
	NoBlocAvailable,
	FileLargerThan64Kb
}


/// The AmsdosFileName structure is used to encode several informations
/// - the user
/// - the filename (up to 8 chars)
/// - the extension (up to 3 chars)
/// It does not contain property information
#[derive(Clone)]
pub struct AmsdosFileName {
	user: u8,
	name: [u8;8],
	extension: [u8;3]
}

impl std::fmt::Debug for AmsdosFileName {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}:{}.{}", self.user(), self.name(), self.extension())
	}
}

impl PartialEq for AmsdosFileName{
	fn eq(&self, other: &AmsdosFileName) -> bool {
		self.user == other.user &&
		self.name().to_uppercase() == other.name().to_uppercase() &&
		self.extension().to_uppercase() == other.extension().to_uppercase()
    }
}


impl AmsdosFileName {
	pub fn filename_header_format(&self) -> &[u8;8] {
		/*
		let mut content = [' ' as u8;8];
		println!("*{:?}*", self.name);
		for (i, c) in self.name.as_bytes().iter().enumerate() {
			content[i] = *c;
		}
		content
		*/
		&self.name
	}

	pub fn extension_header_format(&self) -> &[u8;3] {
		/*
		let mut content = [' ' as u8;3];
		for (i, c) in self.extension.as_bytes().iter().enumerate() {
			content[i] = *c;
		}
		content
		*/
		&self.extension
	}

	pub fn from_slice(slice: &[u8]) -> AmsdosFileName {
		Self::from_entry_format(array_ref!(slice, 0, 12))
	}

	/// Create an amsdos filename from a catalog entry buffer
	pub fn from_entry_format(buffer: &[u8;12]) -> AmsdosFileName {
		let user: u8 = buffer[0];
		let name: [u8;8] = array_ref!(buffer, 1, 8).clone();
		// Remove bit 7 of each char
		let mut extension: [u8;3] = array_ref!(buffer, 9, 3).clone();
		extension.iter_mut().for_each(|b|{
			*b = *b & 0b01111111
		});

		let fname = AmsdosFileName {
			user,
			name,
			extension
		};

		fname
	}

	/// Build a filename compatible with the catalog entry format
	pub fn to_entry_format(&self, system: bool, read_only: bool) -> [u8; 12] {
		let mut buffer = [0; 12];
	
		buffer[0] = self.user;
		buffer[1..9].copy_from_slice(self.filename_header_format().as_ref());
		buffer[9..].copy_from_slice(self.extension_header_format().as_ref());

		if system {
			buffer[9] += 0b10000000;
		}
		if read_only {
			buffer[10] += 0b10000000;
		}

		buffer
	}



	pub fn user(&self) -> u8 {
		self.user
	}

	pub fn name(&self) -> String {
		String::from_utf8_lossy(&self.name).into_owned().trim().to_owned()
	}

	pub fn extension(&self) -> String {
		String::from_utf8_lossy(&self.extension).into_owned().trim().to_owned()
	}

	pub fn filename(&self) -> String {
		format!("{}.{}", self.name(), self.extension())
	}

	pub fn filename_with_user(&self) -> String {
		format!("{}:{}.{}", self.user(), self.name(), self.extension())
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

		let name = {
			let mut encoded_filename = [' ' as u8; 8];
			for (idx,c) in filename.as_bytes().iter().enumerate() {
				encoded_filename[idx] = *c
			}
			encoded_filename
		};

		let extension = {
			let mut encoded_extension = [' ' as u8; 3];
			for (idx,c) in extension.as_bytes().iter().enumerate() {
				encoded_extension[idx] = *c
			}
			encoded_extension
		};

		//TODO see if upercase is needed
		Ok(AmsdosFileName{
			user,
			name,
			extension
		})

	}
}

// TODO use tryfrom asap
impl From<&str> for AmsdosFileName {

	/// Make a filename conversion by considering the following format is used: user:name.extension
	fn from(content: &str) -> AmsdosFileName {
		let (user, rest) = match content.find(':') {
			None => (0, content),
			Some(1) => (
				u8::from_str_radix(&content[..1], 10).unwrap(),
				&content[2..]
			),
			_ => unreachable!()
		};

		let (filename, extension ) = match rest.find('.') {
			None => (rest, ""),
			Some(idx) => (&rest[..idx], &rest[(idx+1)..])
		};

		AmsdosFileName::new(user, filename, extension).unwrap()
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
/// Encode the index of a bloc
#[derive(Debug, Copy, Clone, Ord, Eq)]
pub enum BlocIdx {
	Empty,
	Index(std::num::NonZeroU8)
}

impl Default for BlocIdx {
	fn default()-> BlocIdx {
		BlocIdx::Empty
	}
}

impl From<u8> for BlocIdx {
	fn from(val: u8) -> BlocIdx {
		match val {
			0 => BlocIdx::Empty,
			val => BlocIdx::Index(
				unsafe{std::num::NonZeroU8::new_unchecked(val)}
			)
		}
	}
}

impl Into<u8> for &BlocIdx {
	fn into(self) -> u8 {
		match self {
			BlocIdx::Empty => 0,
			BlocIdx::Index(ref val) => val.get()
		}
	}
}

impl PartialOrd for BlocIdx {
    fn partial_cmp(&self, other: &BlocIdx) -> Option<std::cmp::Ordering> {
		let a: u8 = self.into();
		let b: u8 = other.into();
        a.partial_cmp(&b)
    }
}

impl PartialEq for BlocIdx {
	fn eq(&self, other: &BlocIdx) -> bool {
		let a: u8 = self.into();
		let b: u8 = other.into();
		a == b
    }
}

impl BlocIdx {
	pub fn is_valid(&self) -> bool {
		match self {
			BlocIdx::Empty => false,
			BlocIdx::Index(_) => true
		}
	}

	pub fn value(&self) -> u8 {
		 self.into()
	}

	/// only valid for a valid block
	pub fn track(&self) -> u8 {
		(( (self.value() as u16) << 1) / 9) as u8
	}

	/// only valid for a valid block
	pub fn sector(&self) -> u8 {
		(( (self.value() as u16) << 1) % 9) as u8
	}
}



// http://www.cpc-power.com/cpcarchives/index.php?page=articles&num=92
#[derive(Debug, Clone)]
pub struct AmsdosEntry {
	/// Location of the entry in the catalog
	idx: u8,
	/// Name of the file
	file_name : AmsdosFileName,
	read_only: bool,
	system: bool,
	num_page: u8,
	nb_pages: u8,
	blocs: [BlocIdx;16]
}

impl std::fmt::Display for AmsdosEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let fname = self.amsdos_filename().filename_with_user();
		let size = self.used_space();

		write!(f, "{} {}K", fname, size)

    }
}

impl AmsdosEntry {

	/// Provide the size, in Kb, eaten by the file on disc
	pub fn used_space(&self) -> usize {
		(self.nb_pages as usize * DATA_SECTOR_SIZE  as usize * 2) / 1024
	}

	/// Check if the given filename corresponds to the entry
	pub fn belongs_to(&self, filename: &AmsdosFileName) -> bool {
		&self.file_name == filename
	}

	pub fn amsdos_filename(&self) -> &AmsdosFileName {
		&self.file_name
	}

	pub fn from_slice(idx: u8, slice: &[u8]) -> AmsdosEntry {
		Self::from_buffer(idx, array_ref!(slice, 0, 32))
	}

	/// Create the entry from its 32 bytes
	pub fn from_buffer(idx: u8, buffer: &[u8;32]) -> AmsdosEntry  {
		AmsdosEntry {
			idx,
			file_name: AmsdosFileName::from_entry_format(
				array_ref!(buffer, 0, 12)
			),
			read_only: buffer[1+8+0].bit(7),
			system: buffer[1+8+1].bit(7),
			num_page: buffer[12],
			nb_pages: buffer[15],
			blocs: {
				let blocs = buffer[16..].iter()
					.map(|&b|{BlocIdx::from(b)})
					.collect::<Vec<BlocIdx>>();
				let mut array_blocs = [BlocIdx::default(); 16];
				for i in 0..16 {
					array_blocs[i] = blocs[i];
				}
				array_blocs
			}
		}
	}

	/// Returns the list of used blocs by tis entry
	pub fn used_blocs(&self) -> &[BlocIdx] {
		&self.blocs[..(self.nb_pages as usize)]
	}

	pub fn as_bytes(&self) -> [u8; 32] {
		let mut bytes = [0; 32];
		bytes[0..12].copy_from_slice(
			self.file_name.to_entry_format(
							self.system, 
							self.read_only).as_ref()
		);
		bytes[12] = self.num_page;
		bytes[15] = self.nb_pages;
		bytes[16..].copy_from_slice(
			&self.blocs.iter()
				.map(|b|->u8{b.into()})
				.collect::<Vec<u8>>()
		);

		bytes
	}

	pub fn len() -> usize {
		32
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
			self.file_name.name(),
			self.file_name.extension()
		)
	} 

	
}

/// Encode the catalog of an existing disc
pub struct AmsdosEntries {
	entries: Vec<AmsdosEntry>	
}

impl std::fmt::Debug for AmsdosEntries {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		for entry in self.used_entries() {
			write!(f, "{:?}", entry)?;
		}
		Ok(())
	}
}

impl AmsdosEntries {
	/// Generate a binary version that can be used to export the catalog
	pub fn as_bytes(&self) -> [u8; 64*32] {
		let mut content = [0; 64*32];
		for i in 0..64 {
			content[i*32 .. (i+1)*32].copy_from_slice(&self.entries[i].as_bytes());
		}
		content
	}

	/// Manually create the catalog from a byte slice. Usefull to manipulate catarts
	pub fn from_slice(slice: &[u8]) -> AmsdosEntries {
		let mut entries = Vec::new();
		for i in 0..64 {
			entries.push(
				AmsdosEntry::from_slice(i as _, &slice[i*32..(i+1)*32])
			)
		}
		AmsdosEntries {
			entries
		}
	}

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

	pub fn used_entries(&self) -> impl Iterator<Item = &AmsdosEntry> {
		self.entries.iter()
			.filter(|&entry|{
				entry.nb_pages > 0
			})
	}

	/// Returns entries erased 
	pub fn free_entries(&self) -> impl Iterator<Item = &AmsdosEntry> {
		self.entries.iter()
			.filter(|&entry|{
				entry.is_erased() || entry.nb_pages == 0
			})
	}

	/// Return all the entries that correspond to a given file
	pub fn for_file(&self, filename: &AmsdosFileName)-> impl Iterator<Item = & AmsdosEntry> 
	{
		let filename: AmsdosFileName = filename.clone();
		self.entries.iter()
			.filter(move |&entry|{
				entry.nb_pages > 0 &&
				entry.belongs_to(&filename)
			})


	}

	/// Returns one available entry or None if there is no entry
	pub fn one_empty_entry(&self) -> Option<&AmsdosEntry> {
		let res = self.free_entries().take(1).collect::<Vec<_>>();
		if res.len() > 0 {
			Some(res[0])
		}
		else {
			None
		}
	}

	/// Returns the blocs that are not referenced in the catalog.
	/// Bloc used in erased files are returned (so they may be broken)
	pub fn available_blocs(&self) -> Vec<BlocIdx> {
		// first 2 blocs are not available if we trust idsk. So  Ido the same
		let mut set = (2..=255).map(|b|{BlocIdx::from(b)})
						.collect::<std::collections::BTreeSet<BlocIdx>>();
		let used = self.used_entries()
			.flat_map(|e|{e.blocs.iter()})
			.map(|&b|{b})
			.collect::<std::collections::BTreeSet<BlocIdx>>();
		set.difference(&used)
			.map(|&b|{b})
			.collect::<Vec<BlocIdx>>()
	}

	/// Returns one available bloc or None if there is no entry
	pub fn one_empty_bloc(&self) -> Option<BlocIdx> {
		let res = self.available_blocs();
		if res.len() > 0 {
			Some(res[0])
		}
		else {
			None
		}
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
const DATA_SECTOR_SIZE: usize = 512;



/// Minimal information needed to access to twe two sectors of a given bloc
struct BlocAccessInformation{
	track1: u8,
	sector1_id: u8,
	track2: u8,
	sector2_id:u8
}



/// http://cpctech.cpc-live.com/docs/manual/s968se09.pdf
/// Current implementatin only focus on DATA format
/// 
pub struct AmsdosManager {
	disc: ExtendedDsk,
	side: crate::disc::edsk::Side
}

impl AmsdosManager {
	pub fn disc(&self) -> &ExtendedDsk {
		&self.disc
	}

	pub fn new_from_disc<S: Into<Side>>(disc: ExtendedDsk, side: S) -> AmsdosManager {
		AmsdosManager {
			disc,
			side: side.into()
		}
	}

	/// Return the entries of the Amsdos catalog
	/// Panic if dsk is not compatible
	pub fn catalog(&self) -> AmsdosEntries {
		let mut entries = Vec::new();
		let bytes = self.disc.sectors_bytes(
			self.side,
			0, 
			DATA_FIRST_SECTOR_NUMBER, 
			4).unwrap();
		
		for idx in 0..DIRECTORY_SIZE/*(bytes.len() / 32)*/ {
			let entry_buffer=&bytes[(idx*32)..(idx+1)*32];
			let entry = AmsdosEntry::from_buffer(
				idx as u8,
				array_ref!(entry_buffer, 0, 32)
			);
			entries.push(entry);
		}

		AmsdosEntries {
			entries
		}
	}

	/// Rewrite the whole catalog
	pub fn set_catalog(&mut self, entries: &AmsdosEntries) {
		let entries = entries.as_bytes();
		println!("{}", entries.len() as f32/32.0);

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

	/// Return the file if it exists
	pub fn get_file<F: Into<AmsdosFileName>>(&self, filename: F) -> Option<AmsdosFile> {
		// Collect the entries for the given file
		let entries = {
			let filename = filename.into();
			let entries = self.catalog().for_file(&filename)
							.map(|e|{e.clone()})
							.collect::<Vec<_>>();
			if entries.len() == 0 {
				return None
			}
			entries
		};

		// Retreive the binary data
		let content = entries.iter().flat_map(|entry|{
									self.read_entry(entry)
							})
							.collect::<Vec<u8>>();
		let mut file = AmsdosFile::from_buffer(&content);
		file.shrink_content_to_fit_header_size();

		Some(file)
	}

	/// Add the given amsdos file to the disc
	/// Code is greatly inspired by idsk with no special verifications.
	/// In case of error, the disk is in a broken state => part of the file may be stored...
	pub fn add_file(
		&mut self, 
		file: &AmsdosFile, 
		is_system: bool, 
		is_read_only: bool) -> Result<(), AmsdosError> {

		let content: Vec<u8> = file.full_content()
								.map(|&b|{b})
								.collect::<Vec<u8>>();

		let mut file_pos = 0;
		let file_size = content.len();
		let mut nb_pages = 0;
		println!("File size {} bytes", file_size);
		while file_pos < file_size {
			println!("File pos {}", file_pos);
			let entry_idx = match self.catalog().one_empty_entry() {
				Some(entry) => entry.idx,
				None => return Err(AmsdosError::NoEntriesAvailable)
			};

			println!("Select entry {}", entry_idx);

			let entry_num_page = nb_pages;
			nb_pages += 1;

			let page_size = {
				let mut size = (file_size - file_pos + 127) >> 7; 
				if size > 128 {
					size = 128;
				}
				size
			};
			let entry_nb_pages = page_size;

			// Get the blocs idx AND store the associated Kb on disc
			let nb_blocs = (entry_nb_pages + 7) >> 3;
			let blocs = {
				let mut blocs = [BlocIdx::default(); 16];
				for b in 0..nb_blocs {
					let bloc_idx = match self.catalog().one_empty_bloc() {
						Some(bloc_idx) => bloc_idx,
						None => return Err(AmsdosError::NoBlocAvailable)
					};
					blocs[b] = bloc_idx;
					println!("Select bloc{:?}", bloc_idx);
					self.update_bloc(
						bloc_idx, 
						&Self::padding(
							&content[file_pos..(file_pos+2*DATA_SECTOR_SIZE).min(file_size)],
							2*DATA_SECTOR_SIZE)
					);
					file_pos += 2*DATA_SECTOR_SIZE;
				}
				blocs
			};

			// Update the entry on disc
			let new_entry = AmsdosEntry {
				idx: entry_idx,
				file_name: file.amsdos_filename(),
				read_only: is_read_only,
				system: is_system,
				num_page: entry_num_page,
				nb_pages: entry_nb_pages as u8,
				blocs
			};
			self.update_entry(new_entry)
		}
		Ok(())
	}


	/// Returns a Vec<u8> of the right size by padding 0
	pub fn padding(data: &[u8], size: usize) -> Vec<u8> {
		if data.len() == size {
			data.to_vec()
		}
		else if data.len() > size {
			unreachable!()
		}
		else {
			let missing = size - data.len();
			let mut data = data.to_vec();
			data.resize(size, 0);
			data
		}
	}

	/// Write the entry information on disc AFTER the sectors has been set up.
	/// Panic if dsk is invalid
	/// Still stolen to iDSK
	pub fn update_entry(&mut self, entry: AmsdosEntry) {
		// compute the track/sector
		let min_sect = self.disc.min_sector(self.side);
		let sector_id = (entry.idx >> 4) + min_sect;
		let track = if min_sect == 0x41 {
			2
		} else if min_sect == 1 {
			1
		}
		else {
			0
		}; // XXX why ?
	
		let mut sector = self.disc.sector_mut(
			self.side,
			track, 
			sector_id).unwrap();
		let idx_in_sector:usize = ((entry.idx & 15)  << 5) as usize;
		let mut bytes = &mut sector.values_mut()[idx_in_sector..(idx_in_sector+AmsdosEntry::len())];
		bytes.copy_from_slice(entry.as_bytes().as_ref());
	}



	/// Returns the appropriate information to access the bloc
	/// Blindly stolen to iDSK
	fn bloc_access_information(&self, bloc_idx: BlocIdx) -> BlocAccessInformation {
		// Compute the information to access the first sector
		let sector_pos = bloc_idx.sector();
		let min_sector = self.disc.get_track_information(self.side, 0)
								.unwrap()
								.min_sector();
		let track = {
			let mut track = bloc_idx.track();
			if min_sector == 0x41 {
				track += 2;
			}
			else if min_sector == 0x01 {
				track += 1;
			}
			track
		};

		if track > self.disc.nb_tracks_per_side() - 1 {
			unimplemented!("Need to format track");
		}

		let track1 = track;
		let sector1_id = sector_pos + min_sector;

		// Compute the information to access the second sector
		// TODO set this knowledge in edsk
		let (sector2_id, track2) = {
			if sector_pos > 8 {
				(0 + min_sector, track+1)
			}
			else {
				(sector_pos+1+min_sector, track)
			}
		};

		BlocAccessInformation {
			track1,
			sector1_id,
			track2,
			sector2_id
		}
	}

	/// Write bloc content on disc. One bloc use 2 sectors
	/// Implementation is stolen to iDSK
	pub fn update_bloc(&mut self, bloc_idx: BlocIdx, content: &[u8]) {
		// More tests are needed to check if it can work without that
		assert_eq!(
			content.len(),
			DATA_SECTOR_SIZE*2
		);

		let access_info = self.bloc_access_information(bloc_idx);

		// Copy in first sector
		let mut sector = self.disc.sector_mut(
			self.side,
		 	access_info.track1,
		 	access_info.sector1_id).unwrap();
		sector.set_values(&content[0..DATA_SECTOR_SIZE]).unwrap();

		// Copy in second sector
		let mut sector = self.disc.sector_mut(
			self.side,
		 	access_info.track2,
		 	access_info.sector2_id).unwrap();
		sector.set_values(&content[DATA_SECTOR_SIZE..2*DATA_SECTOR_SIZE]).unwrap();
	}

	/// Read the content of the given bloc
	pub fn read_bloc(&self, bloc_idx: BlocIdx) -> Vec<u8> {
		let access_info = self.bloc_access_information(bloc_idx);

		let sector1_data = self.disc.sector(
								self.side, 
								access_info.track1, 
								access_info.sector1_id).unwrap()
								.values();

		let sector2_data = self.disc.sector(
								self.side, 
								access_info.track2, 
								access_info.sector2_id).unwrap()
								.values();


		let mut content = sector1_data.to_vec();
		content.extend_from_slice(sector2_data);

		assert_eq!(
			content.len(),
			DATA_SECTOR_SIZE*2
		);

		content
	}


	/// Read the content of the given entry
	pub fn read_entry(&self, entry: &AmsdosEntry) -> Vec<u8> {
		entry.used_blocs().iter()
				.flat_map(|bloc_idx| {
					self.read_bloc(*bloc_idx)
				})
				.collect::<Vec<u8>>()
	}
}





/// http://www.cpcwiki.eu/index.php/AMSDOS_Header
#[derive(Clone)]
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

impl PartialEq for AmsdosHeader {
	fn eq(&self, other: &AmsdosHeader) -> bool {
		self.content.as_ref() == other.content.as_ref()
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

	pub fn from_buffer(buffer: &[u8]) -> AmsdosHeader {
		AmsdosHeader {
			content: *array_ref!(buffer, 0, 128)
		}
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


	pub fn set_filename(&mut self, filename: &[u8; 8])  -> &mut Self{
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

	pub fn set_extension(&mut self, extension: &[u8; 3]) -> &mut Self{
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

	/// Checks if the stored checksum correspond to the expected checksum
	pub fn is_checksum_valid(&self) -> bool {
		self.checksum() == self.compute_checksum()
	}

	pub fn as_bytes(&self) -> &[u8; 128] {
		&self.content
	}
}




/// Encode an amsdos file.
#[derive(Clone, PartialEq, Debug)]
pub struct AmsdosFile {
	header: AmsdosHeader,
	content: Vec<u8>
}

impl AmsdosFile {

	/// Create a binary file and add build the header accordingly to the arguments
	pub fn binary_file_from_buffer(
		filename: &AmsdosFileName, 
		loading_address: u16, 
		execution_address: u16, 
		data: &[u8]) -> Result<AmsdosFile, AmsdosError> {

		if data.len() > 0x10000 {
			return Err(AmsdosError::FileLargerThan64Kb);
		}

		let header = AmsdosHeader::build_header(
			filename, 
			AmsdosFileType::Binary,
			loading_address, 
			execution_address, 
			data);
		let content = data.to_vec();

		Ok(AmsdosFile {
			header,
			content
		})
	}

	/// Create a file form its header and content
	pub fn from_buffer(data: &[u8]) -> AmsdosFile {
		let (header_bytes, content_bytes) = data.split_at(128);
		AmsdosFile {
			header: AmsdosHeader::from_buffer(header_bytes),
			content: content_bytes.to_vec()
		}
	}

	pub fn amsdos_filename(&self) -> AmsdosFileName {
		self.header.amsdos_filename()
	}

	/// Return an iterator on the full content of the file: header + content 
	pub fn full_content(&self) -> impl Iterator<Item=&u8> {
		self.header.as_bytes().iter()
			.chain(self.content.iter())
	}

	pub fn header(&self) -> &AmsdosHeader {
		&self.header
	}

	pub fn content(&self) -> &[u8] {
		self.content.as_ref()
	}

	/// Files are read from disc by chunks of the size of 2 sectors. 
	/// This method removes the extra unecessary bytes.
	pub fn shrink_content_to_fit_header_size(&mut self) {
		let size = self.header.file_length() as usize;
		assert!(size <= self.content.len());
		self.content.resize(size, 0);
	}
}