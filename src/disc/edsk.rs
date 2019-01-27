// http://www.cpcwiki.eu/index.php/Format:DSK_disk_image_file_format


use std::path::Path;
use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::string::ToString;
use itertools::zip;

use delegate::delegate;

pub fn convert_real_sector_size_to_fdc_sector_size(mut size: u16) -> u8 {
		let mut n=0;
		while size > 0x80 {
			size = size >> 1;
			n += 1
		}

		n as _
}

pub fn convert_fdc_sector_size_to_real_sector_size(size: u8) -> u16 {
	0x80 << size
}


#[derive(Debug, PartialEq, Copy, Clone, Ord, PartialOrd, Eq)]
pub enum Side {
	SideA,
	SideB,
	Unspecified
}


impl From<u8> for Side {

	fn from(val: u8) -> Side {
		match val {
			0 => Side::SideA,
			1 => Side::SideB,
			_ => Side::Unspecified
		}
	} 
}

impl Into<u8> for Side {
	fn into(self) -> u8 {
		match self {
			Side::SideA => 0,
			Side::SideB => 1,
			Side::Unspecified => 0
		}
	}
}


impl Into<u8> for &Side {
	fn into(self) -> u8 {
		match *self {
			Side::SideA => 0,
			Side::SideB => 1,
			Side::Unspecified => 0
		}
	}
}


#[derive(Debug, Default, PartialEq, Clone)]
pub struct DiscInformation {
	pub(crate) creator_name: String, 
	pub(crate) number_of_tracks: u8, 
	pub(crate) number_of_sides: u8, 
	pub(crate) track_size_table: Vec<u8> // XXX for standard DSK only one value is provided It should be duplicated there
}


impl DiscInformation {

	pub fn creator_name(&self) -> &str {
		&self.creator_name
	}

	pub fn number_of_tracks(&self) -> u8 {
		self.number_of_tracks
	}

	pub fn number_of_sides(&self) -> u8 {
		self.number_of_sides
	}

	pub fn tracks_size_table(&self) -> &[u8] {
		&self.track_size_table
	}

	fn creator_name_as_bytes(&self) -> [u8;14] {
		let mut data = [0 as u8; 14];
		for (idx,byte) in self.creator_name.as_bytes()[0..14].iter().enumerate() {
			data[idx] = *byte;
		}
		data
	}

	// TODO manage the case of standard dsk
	pub fn from_buffer(buffer: &[u8]) -> DiscInformation {
		assert_eq!(buffer.len(), 256);
		assert_eq!(
			String::from_utf8_lossy(&buffer[..34]).to_ascii_uppercase(),
			 "EXTENDED CPC DSK File\r\nDisk-Info\r\n".to_ascii_uppercase()
		);

		let creator_name = String::from_utf8_lossy(&buffer[0x22..=0x2f]);
		let number_of_tracks = buffer[0x30];
		let number_of_sides = buffer[0x31];
		let track_size_table = &buffer[0x34..(0x34+number_of_tracks*number_of_sides)as usize];

		assert!( number_of_sides == 1 || number_of_sides == 2);

		DiscInformation {
			creator_name: creator_name.to_string(),
			number_of_tracks,
			number_of_sides,
			track_size_table:  track_size_table.to_vec()
		}
	}


	fn to_buffer(&self, buffer: &mut Vec<u8>) {
		buffer.extend_from_slice(
			"EXTENDED CPC DSK File\r\nDisk-Info\r\n".as_bytes());
		assert_eq!(buffer.len(), 34);
		
		buffer.extend_from_slice(
			&self.creator_name_as_bytes());
		assert_eq!(buffer.len(), 34+14);

		buffer.push(self.number_of_tracks);
		buffer.push(self.number_of_sides);
		assert_eq!(buffer.len(), 34+14+1+1);

		// XXX missing size of a track 
		buffer.push(0);
		buffer.push(0);
		assert_eq!(buffer.len(), 34+14+1+1+2);

		buffer.extend_from_slice(&self.track_size_table);
		assert_eq!(buffer.len(), 34+14+1+1+2 + self.track_size_table.len());

		// ensure we use 256 bytes
		buffer.resize(256, 0);
	}

	pub fn is_double_sided(&self) -> bool {
		self.number_of_sides == 2
	}

	pub fn is_single_sided(&self) -> bool{
		! self.is_double_sided()
	}

	/// Returns the length of the track including the track information block
	pub fn track_length(&self, track: u8, side: u8) -> u16{
		assert!(side < self.number_of_sides);

		let track = track as usize;
		let side = side as usize;
		let idx = if self.is_single_sided() {
			track
		}
		else {
			track*2 + side
		};

		self.track_length_at_idx(idx)
	}


	pub fn is_formatted(&self, track: u8, side: u8) -> bool{
		self.track_length(track, side) > 0
	}

	pub fn track_length_at_idx(&self, idx: usize) -> u16 {
		256 * u16::from(self.track_size_table[idx])
	}

	pub fn total_tracks_lengths(&self) -> usize {
		(0..self.number_of_distinct_tracks())
			.map(|idx: usize|{
				self.track_length_at_idx(idx) as usize
			})
			.sum::<usize>() 
	}


	pub fn number_of_distinct_tracks(&self) -> usize {
		self.track_size_table.len()
	}

}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct TrackInformation {
	pub(crate) track_number: u8,
	pub(crate) side_number: u8,
	pub(crate) sector_size: u8, // XXX check if really needed to be stored
	pub(crate) number_of_sectors: u8,
	pub(crate) gap3_length: u8,
	pub(crate) filler_byte: u8,
	pub(crate) data_rate: DataRate,
	pub(crate) recording_mode: RecordingMode,
	pub(crate) sector_information_list: SectorInformationList,
	/// The size taken by the track + header in the dsk. This is a duplicated information obtained in the DiscInformation bloc
	pub(crate) track_size: u16
}

impl TrackInformation {

	pub fn sector_size(&self) -> u8 {
		self.sector_size
	}

	pub fn sector_size_human_readable(&self) -> u16 {
		convert_fdc_sector_size_to_real_sector_size(self.sector_size)
	}



	pub fn number_of_sectors(&self) -> u8 {
		self.number_of_sectors
	}

	pub fn gap3_length(&self) -> u8 {
		self.gap3_length
	}

	pub fn filler_byte(&self) -> u8 {
		self.filler_byte
	}

	/// Fail if the track has no sector
	pub fn min_sector(&self) -> u8 {
		self.sector_information_list.sectors()
			.iter().map(|s|{s.sector_information_bloc.sector_id})
			.min()
			.unwrap()
	}

	/// Compute the sum of data contained by all the sectors.
	/// Only serves for debug purposes
	pub fn data_sum(&self) -> usize {
		self.sector_information_list.sectors.iter()
			.map(|s|{
				s.data_sum()
			})
			.sum::<usize>()
	}

	pub fn corresponds_to(&self, track: u8, side: u8) -> bool {
		self.track_number == track && self.side_number == side
	}

	pub fn from_buffer(buffer: &[u8]) -> TrackInformation {
		if 	String::from_utf8_lossy(&buffer[..0xc]).to_ascii_uppercase() != "Track-info\r\n".to_ascii_uppercase() {
			panic!("Track buffer does not seem coherent\n{:?}", buffer);
		}
		
		let track_size = buffer.len() as u16;
		let track_number = buffer[0x10];
		let side_number = buffer[0x11];
		let sector_size =  buffer[0x14];
		let number_of_sectors = buffer[0x15];
		let gap3_length = buffer[0x16];
		let filler_byte = buffer[0x17];
		let data_rate: DataRate = buffer[0x12].into();
		let recording_mode = buffer[0x13].into();


		println!("Track {} side {} sector_size {} nb_sectors {} gap length {:x}, filler_byte {:x}", track_number, side_number, sector_size, number_of_sectors, gap3_length, filler_byte);
		let sector_information_list = SectorInformationList::from_buffer(&buffer[0x18..], number_of_sectors);

		let track_info = TrackInformation {
			track_number,
			side_number,
			sector_size,
			number_of_sectors,
			gap3_length,
			filler_byte,
			data_rate,
			recording_mode,
			sector_information_list,
			track_size
		};

		assert!(track_info.track_size != 0);
		track_info

	}


	pub fn to_buffer(&self, buffer: &mut Vec<u8>) {
		let start_size = buffer.len();

		// low byte MUST be null
		assert_eq!(
			start_size % 256,
			0
		);

		buffer.extend_from_slice(&"Track-Info\r\n".as_bytes()[..12]);
		assert_eq!(buffer.len()-start_size, 12);

		buffer.push(0); 
		buffer.push(0); 
		buffer.push(0); 
		buffer.push(0); 

		buffer.push(self.track_number);
		buffer.push(self.side_number);

		buffer.push(self.data_rate.clone().into());
		buffer.push(self.recording_mode.clone().into());

		buffer.push(self.sector_size);
		buffer.push(self.number_of_sectors);
		buffer.push(self.gap3_length);
		buffer.push(self.filler_byte);

		assert_eq!(buffer.len()-start_size, 0x18);
		// Inject sectors information list
		self.sector_information_list.sectors
			.iter()
			.for_each(|s|{
				s.sector_information_bloc.to_buffer(buffer);
			});

		let added_bytes = buffer.len() - start_size;
		let missing_bytes = 256 - added_bytes ;
		buffer.resize(buffer.len() + missing_bytes, 0); 

		// Inject sectors information data
		self.sector_information_list.sectors
			.iter()
			.for_each(|s|{
				buffer.extend_from_slice(&s.values);
			});

		// Ensure the size is correct
		let added_bytes = (buffer.len() - start_size) as u16;
		assert!(added_bytes <=  self.track_size);
		let missing_bytes = self.track_size - added_bytes;
		if missing_bytes != 0 {
			buffer.resize(buffer.len() + missing_bytes as usize, 0);
		}

	}

	pub fn total_size(&self) -> usize {
		self.sector_information_list.sectors.iter()
			.map(|info|{
				info.sector_information_bloc.data_length as usize
			})
			.sum()
	}

	delegate! {
		target self.sector_information_list {
			pub fn sector(&self, sector_id: u8) -> Option<&Sector>;
			pub fn sector_mut(&mut self, sector_id: u8) -> Option<&mut Sector>;
		}
	}
}


#[derive(Debug, Clone, PartialEq)]
pub enum DataRate {
	Unknown = 0,
	SingleOrDoubleDensity = 1,
	HighDensity = 2,
	ExtendedDensity = 3
}

impl Default for DataRate {
	fn default() -> Self {
		DataRate::Unknown
	}
}

impl From<u8> for DataRate {
	fn from(b: u8) -> DataRate {
		match b {
			0 => DataRate::Unknown,
			1 => DataRate::SingleOrDoubleDensity,
			2 => DataRate::HighDensity,
			3 => DataRate::ExtendedDensity,
			_ => unreachable!()
		}
	}
}


impl Into<u8> for DataRate {
	fn into(self) -> u8 {
		match self {
			DataRate::Unknown => 0,
			DataRate::SingleOrDoubleDensity => 1,
			DataRate::HighDensity => 2,
			DataRate::ExtendedDensity => 3,
			_ => unreachable!()
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecordingMode {
	Unknown = 0,
	FM = 1,
	MFM = 2
}


impl Default for RecordingMode{
	fn default() -> Self {
		RecordingMode::Unknown
	}
}



impl From<u8> for RecordingMode {
	fn from(b: u8) -> RecordingMode {
		match b {
			0 => RecordingMode::Unknown,
			1 => RecordingMode::FM,
			2 => RecordingMode::MFM,
			_ => unreachable!()
		}
	}
}


impl Into<u8> for RecordingMode {
	fn into(self) -> u8 {
		match self {
			RecordingMode::Unknown => 0,
			RecordingMode::FM => 1,
			RecordingMode::MFM => 2,
			_ => unreachable!()
		}
	}
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct SectorInformation {
	/// track (equivalent to C parameter in NEC765 commands)
	pub(crate) track: u8,
	/// side (equivalent to H parameter in NEC765 commands)
	pub(crate) side: u8,
	/// sector ID (equivalent to R parameter in NEC765 commands)
	pub(crate) sector_id: u8,
	/// sector size (equivalent to N parameter in NEC765 commands)
	pub(crate) sector_size: u8,
	/// FDC status register 1 (equivalent to NEC765 ST1 status register)
	pub(crate) fdc_status_register_1: u8,
	/// FDC status register 2 (equivalent to NEC765 ST2 status register)
	pub(crate) fdc_status_register_2: u8,
	/// actual data length in bytes
	pub(crate) data_length: u16, // in bytes, little endian notation
}


impl SectorInformation {
	pub fn from_buffer(buffer : &[u8]) -> SectorInformation {
		let info = SectorInformation {
			track: buffer[0x00],
			side: buffer[0x01],
			sector_id: buffer[0x02],
			sector_size: buffer[0x03],
			fdc_status_register_1: buffer[0x04],
			fdc_status_register_2: buffer[0x05],
			data_length: u16::from(buffer[0x06]) + (u16::from(buffer[0x07]) *  256)
		};
		info
	}

	pub fn to_buffer(&self, buffer: &mut Vec<u8>) {
		buffer.push(self.track);
		buffer.push(self.side);
		buffer.push(self.sector_id);
		buffer.push(self.sector_size);
		buffer.push(self.fdc_status_register_1);
		buffer.push(self.fdc_status_register_2);
		buffer.push( (self.data_length%256) as u8);
		buffer.push( (self.data_length/256) as u8);
	}


}


#[derive(Debug, Default, PartialEq, Clone)] 
pub struct SectorInformationList {
	//sectors: Vec<Sector>
	pub(crate) sectors: Vec<Sector>,
}

impl SectorInformationList {
	pub fn sectors(&self) -> &[Sector] {
		&self.sectors
	}

	/// Return the number of sectors
	pub fn len(&self) -> usize {
		self.sectors.len()
	}

	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Add a sector
	pub fn add_sector(&mut self, sector: Sector) {
		self.sectors.push(sector);
	}

	pub fn from_buffer(buffer: &[u8], number_of_sectors: u8) -> SectorInformationList {

		let mut list_info = Vec::new();
		let mut list_data = Vec::new();
		let mut consummed_bytes = 0;
		

		// Get the information
		for _sector_number in 0..number_of_sectors {
			let current_buffer = &buffer[consummed_bytes..];
			let sector = SectorInformation::from_buffer(current_buffer);
			consummed_bytes += 8;
			list_info.push(sector);
		}

		// Get the data
		consummed_bytes = 256 - 0x18; // Skip the unused bytes
		for sector in list_info.iter() {
			let current_sector_size =sector.data_length as usize;
			let current_buffer = &buffer[consummed_bytes .. consummed_bytes + current_sector_size];
			list_data.push(current_buffer.to_vec());
			consummed_bytes += current_sector_size;
		}


		// merge them
		let info_drain = list_info.drain(..);
		let data_drain = list_data.drain(..);
		let sectors = zip(info_drain, data_drain)
		.map(|(info, data)|{
			Sector {
				sector_information_bloc: info,
				values: data
			}
		}).collect::<Vec<Sector>>();

		SectorInformationList {
			sectors
		}

	}



	pub fn sector(&self, sector_id: u8) -> Option<&Sector> {
		self.sectors.iter()
			.find(|sector|{
				sector.sector_information_bloc.sector_id == sector_id
			})
	}

	/// Returns the sector that correspond to the requested id
	pub fn sector_mut(&mut self, sector_id: u8) -> Option<&mut Sector> {
		self.sectors.iter_mut()
			.find(|sector|{
				sector.sector_information_bloc.sector_id == sector_id
			})
	}



/// Fill the information list with sectors corresponding to the provided arguments
	pub fn fill_with(
		&mut self, 
		ids: &[u8], 
		heads: &[u8], 
		track_number: u8, 
		sector_size: u8,
		filler_byte: u8) {
		assert_eq!(ids.len(), heads.len());
		assert_eq!(self.len(), 0);

		for idx in 0..ids.len() {
			let mut sector= Sector::default();



			sector.sector_information_bloc.track = track_number;
			sector.sector_information_bloc.sector_size = sector_size;
			sector.sector_information_bloc.sector_id = ids[idx];
			sector.sector_information_bloc.side = heads[idx];

			let data_size =  convert_fdc_sector_size_to_real_sector_size(sector.sector_information_bloc.sector_size as _) as usize;
			sector.values.resize(data_size, filler_byte);
			sector.sector_information_bloc.data_length = sector.values.len() as u16;

			self.add_sector(sector);
		}

	}
}

bitflags! {
    struct FdcStatusRegister1: u8 {
        const EndOfCylinder = 1<<7;
        const DataError = 1<<5;
        const NoData = 1<<2;
        const MissingAddressMark = 1<<0;
    }
}

bitflags! {
    struct FdcStatusRegister2: u8 {
        const ControlMark = 1<<5;
        const DataErrorInDataField = 1<<5;
        const MissingAddressMarkInDataField = 1<<0;
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Sector {
	pub(crate) sector_information_bloc: SectorInformation,
	pub(crate) values: Vec<u8>
}


impl Sector  {
	/// Number of bytes stored in the sector
	pub fn len(&self) -> u16 {
		self.values.len() as u16
	}

	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	pub fn data_sum(&self) -> usize {
		self.values.iter()
			.map(|&v|{v as usize})
			.sum::<usize>()
	}

	pub fn values(&self) -> &[u8] {
		&self.values
	}

	pub fn values_mut(&mut self) ->&mut[u8] {
		&mut self.values
	}

	/// Set all the values stored in the sector
	pub fn set_values(&mut self, data: &[u8]) -> Result<(), String> {
		if (data.len() as u16) < self.len() {
			return Err(format!(
				"You cannot insert {} bytes in a sector of size {}.", 
				data.len(), 
				self.len())
			);
		}

		if (data.len() as u16) > self.len() {
			return Err(format!(
				"Smaller data of {} bytes to put in a sector of size {}.", 
				data.len(), 
				self.len())
			);
		}

		self.values[..].clone_from_slice(data);
		Ok(())
	}
}

#[derive(Default, PartialEq, Debug, Clone)]
pub struct TrackInformationList {
	pub(crate) list: Vec<TrackInformation>
}


impl TrackInformationList {
	fn from_buffer_and_disc_information(
		buffer: &[u8], 
		disc_info: &DiscInformation) -> TrackInformationList {

		let mut consummed_bytes:usize = 0;
		let mut list = Vec::new();

		for track_number in 0..disc_info.number_of_tracks{
			for side_nb in 0..disc_info.number_of_sides {
				// Size of the track data + header
				let current_track_size = disc_info.track_length(
					track_number, 
					side_nb) as usize;
				// TODO treat the case of unformatted tracks
				let track_buffer = &buffer[
					consummed_bytes as usize ..
					(consummed_bytes+current_track_size) as usize];
				list.push(TrackInformation::from_buffer(track_buffer));
				consummed_bytes += current_track_size;
			}
		}

		TrackInformationList {
			list
		}
	}

	fn to_buffer(&self, buffer: &mut Vec<u8>) {
		for track in self.list.iter() {
			let added_bytes = track.to_buffer(buffer);
		}
	}

	/// Add an empty track and return it. It is up to the caller to properly feed it
	pub fn add_empty_track(&mut self) -> &mut TrackInformation{
		let track = TrackInformation::default();
		self.list.push(track);
		self.list.last_mut().unwrap()
	}

	/// Returns the tracks for the given side
	pub fn tracks_for_side(&self, side: Side) -> impl Iterator<Item=&TrackInformation> {
		let side:u8 = side.into();
		self.list.iter()
			.filter(move |info| {
				info.side_number == side
			})
	}



}


#[derive(Default, PartialEq, Debug, Clone)]
pub struct ExtendedDsk {
	pub(crate) disc_information_bloc: DiscInformation,
	pub(crate) track_list: TrackInformationList
}



impl ExtendedDsk {
	/// open an extended dsk from an existing file
	pub fn open<P>(path: P) -> io::Result<ExtendedDsk> 
	where P:AsRef<Path>{
		// Read the whole file
		let buffer = {
			let mut f = File::open(path)?;
			let mut buffer = Vec::new();
			f.read_to_end(&mut buffer)?;
			buffer
		};

		Ok(Self::from_buffer(&buffer))
	}

	pub fn from_buffer(buffer: &[u8]) -> ExtendedDsk {
		let disc_info = DiscInformation::from_buffer(&buffer[..256]);

		println!("Disc info {:?} / total {} / nb_tracks {}", disc_info, disc_info.total_tracks_lengths(), disc_info.number_of_distinct_tracks());
		let track_list = TrackInformationList::from_buffer_and_disc_information(&buffer[256..], & disc_info);

		ExtendedDsk {
			disc_information_bloc: disc_info,
			track_list
		}
	}


	/// Save the dsk in a file one disc
	pub fn save<P>(&self, path: P) -> io::Result<()>
	where P:AsRef<Path>{
		let mut file_buffer = File::create(path)?;
		let mut memory_buffer = Vec::new();
		self.to_buffer(&mut memory_buffer);
		file_buffer.write_all(&memory_buffer)
	}

	/// Write the dsk in the buffer
	pub fn to_buffer(&self, buffer: &mut Vec<u8>) {
		self.disc_information_bloc.to_buffer(buffer);
		self.track_list.to_buffer( buffer);
	}


	pub fn is_double_sided(&self) -> bool {
		self.disc_information_bloc.is_double_sided()
	}

	// We assume we have the same number of tracks per side.
	// Need to be modified the day ot will not be the case.
	pub fn nb_tracks_per_side(&self) -> u8 {
		let val = if self.disc_information_bloc.is_single_sided() {
			self.track_list.list.len()
		}
		else {
			self.track_list.list.len()/2
		};
		val as u8
	}

	pub fn nb_sides(&self) -> u8 {
		self.disc_information_bloc.number_of_sides
	}


	pub fn get_track_information<S: Into<Side>>(&self, side: S, track: u8) -> Option<&TrackInformation> {
		let idx = self.get_track_idx(side.into(), track);
		self.track_list.list.get(idx)
	}


	pub fn get_track_information_mut(&mut self, side: Side, track: u8) -> Option<&mut TrackInformation> {
		let idx = self.get_track_idx(side.into(), track);
		self.track_list.list.get_mut(idx)
	}

	/// Search and returns the appropriate sector
	pub fn sector<S: Into<Side>>(&self,side: S, track: u8, sector_id: u8) -> Option<&Sector> {
		self.get_track_information(side.into(), track)
			.and_then(|track|{track.sector(sector_id)})
	}

	/// Search and returns the appropriate mutable sector 
	pub fn sector_mut<S: Into<Side>>(&mut self,side: S, track: u8, sector_id: u8) -> Option<&mut Sector> {
		self.get_track_information_mut(side.into(), track)
			.and_then(|track|{track.sector_mut(sector_id)})
	}

	fn get_track_idx(&self, side: Side, track: u8) -> usize {
		if self.disc_information_bloc.is_double_sided() {
			let side = match side {
				Side::SideA => 0,
				Side::SideB => 1,
				Side::Unspecified => panic!("You must specify a side for a double sided disc.")
			};
			track as usize * 2 + side
		}
		else {
			if let Side::SideB = side {
				panic!("You cannot select side B in a single sided disc");
			}
			track as usize
		}
	}

	/// Return the concatenated values of several consecutive sectors
	pub fn sectors_bytes<S: Into<Side>>(&self, side: S, track: u8, sector_id: u8, nb_sectors: u8) -> Option<Vec<u8>> {
		let mut res = Vec::new();
		let side = side.into();

		for count in 0..nb_sectors {
			match self.sector(side.clone(), track, sector_id+count) {
				None => return None,
				Some(s) => {
					res.extend(s.values.iter())
				}
			}
		}

		Some(res)
	}


	/// Compute the datasum for the given track
	pub fn data_sum(&self, side: Side) -> usize {
		self.track_list.tracks_for_side(side)
			.map(|t|{
				t.data_sum()
			})
			.sum()
	}

	/// Returns all the tracks
	pub fn tracks(&self) -> &[TrackInformation] {
		&self.track_list.list
	}

	/// Returns the number of tracks
	pub fn nb_tracks(&self) -> usize {
		self.tracks().len()
	}

	/// Return the smallest sector id over all tracks
	pub fn min_sector<S: Into<Side>>(&self, size: S) -> u8 {
		self.tracks().iter()
			.map(|t|{t.min_sector()})
			.min().unwrap()
	}
}