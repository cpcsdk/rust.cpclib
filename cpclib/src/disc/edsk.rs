// http://www.cpcwiki.eu/index.php/Format:DSK_disk_image_file_format


use std::path::Path;
use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::string::ToString;
use itertools::zip;

#[derive(Debug)]
struct DiscInformation {
	creator_name: String, 
	number_of_tracks: u8, 
	number_of_sides: u8, 
	track_size_table: Vec<u8> // XXX dor standard DSK only one value is provided It should be duplicated there
}


impl DiscInformation {
	fn from_buffer(buffer: &[u8]) -> DiscInformation {
		assert_eq!(buffer.len(), 256);
		assert_eq!(
			String::from_utf8_lossy(&buffer[..34]).to_ascii_uppercase(),
			 "EXTENDED CPC DSK File\r\nDisk-Info\r\n".to_ascii_uppercase()
		);

		let creator_name = String::from_utf8_lossy(&buffer[0x22..0x2f+1]);
		let number_of_tracks = buffer[0x30];
		let number_of_sides = buffer[0x31];
		let track_size_table = &buffer[0x34..(0x34+number_of_tracks*number_of_sides+1)as usize];


		assert!( number_of_sides == 1 || number_of_sides == 2);

		DiscInformation {
			creator_name: creator_name.to_string(),
			number_of_tracks,
			number_of_sides,
			track_size_table:  track_size_table.to_vec()
		}
	}


	pub fn is_double_sided(&self) -> bool {
		self.number_of_sides == 2
	}

	pub fn is_single_sided(&self) -> bool{
		! self.is_double_sided()
	}

	/// Returns the length of the track including the track information block
	pub fn track_length(&self, track: u8, side: u8) -> u16{
		assert!(side <= self.number_of_sides);

		let track = track as usize;
		let side = side as usize;
		let idx = if self.is_single_sided() {
			track
		}
		else {
			track*2 + (side-1)
		};

		self.track_length_at_idx(idx)
	}


	pub fn is_formatted(&self, track: u8, side: u8) -> bool{
		self.track_length(track, side) > 0
	}

	fn track_length_at_idx(&self, idx: usize) -> u16 {
		256 * (self.track_size_table[idx] as u16)
	}

	pub fn total_tracks_lengths(&self) -> usize {
		(0..self.number_of_distinct_tracks())
			.into_iter()
			.map(|idx: usize|{
				self.track_length_at_idx(idx) as usize
			})
			.sum::<usize>() 
	}


	pub fn number_of_distinct_tracks(&self) -> usize {
		self.track_size_table.len()
	}

}

#[derive(Debug)]
struct TrackInformation {
	track_number: u8,
	side_number: u8,
	sector_size: u8, // XXX check if really needed to be stored
	number_of_sectors: u8,
	gap3_length: u8,
	filler_byte: u8,
	data_rate: DataRate,
	recording_mode: RecordingMode,
	sector_information_list: SectorInformationList,
}

impl TrackInformation {

	pub fn corresponds_to(&self, track: u8, side: u8) -> bool {
		self.track_number == track && self.side_number == side
	}

	pub fn from_buffer(buffer: &[u8]) -> TrackInformation {
		assert_eq!(
			String::from_utf8_lossy(&buffer[..0xc]).to_ascii_uppercase(), "Track-info\r\n".to_ascii_uppercase());
		
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
		};

		println!("Size: {}", track_info.total_size());

		track_info

	}

	pub fn total_size(&self) -> usize {
		self.sector_information_list.sectors.iter()
			.map(|info|{
				info.sector_information_bloc.data_length as usize
			})
			.sum()
	}


	pub fn sector(&self, sector: u8) -> Option<&Sector> {
		self.sector_information_list.sector(sector)
	}
}


#[derive(Debug)]
enum DataRate {
	Unknown = 0,
	SingleOrDoubleDensity = 1,
	HighDensity = 2,
	ExtendedDensity = 3
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

#[derive(Debug)]
enum RecordingMode {
	Unknown = 0,
	FM = 1,
	MFM = 2
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

#[derive(Debug)]
pub struct SectorInformation {
	track: u8,
	side: u8,
	sector_id: u8,
	sector_size: u8,
	fdc_status_register_1: u8,
	fdc_status_register_2: u8,
	data_length: u16, // in bytes, little endian notation
}


impl SectorInformation {
	fn from_buffer(buffer : &[u8]) -> SectorInformation {
		let info = SectorInformation {
			track: buffer[0x00],
			side: buffer[0x01],
			sector_id: buffer[0x02],
			sector_size: buffer[0x03],
			fdc_status_register_1: buffer[0x04],
			fdc_status_register_2: buffer[0x05],
			data_length: buffer[0x06] as u16 + (buffer[0x07] as u16 *  256)
		};

		println!("{:?}", info);



		info
	}


}


#[derive(Debug)]
struct SectorInformationList {
	//sectors: Vec<Sector>
	sectors: Vec<Sector>,
}

impl SectorInformationList {
	fn from_buffer(buffer: &[u8], number_of_sectors: u8) -> SectorInformationList {

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
		consummed_bytes = 231; // XXX No idea why we use this value !! there is no explanation in the documentation
		for sector in list_info.iter() {
			let current_sector_size = sector.data_length as usize;
			let current_buffer = &buffer[consummed_bytes.. current_sector_size + consummed_bytes];
			list_data.push(current_buffer.to_vec());
			consummed_bytes += current_sector_size;

			println!("sector sum {}", current_buffer.iter().map(|val|{*val as usize}).sum::<usize>());
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

#[derive(Debug)]
pub struct Sector {
	sector_information_bloc: SectorInformation,
	values: Vec<u8>
}


impl Sector  {
	/// Number of bytes stored in the sector
	pub fn len(&self) -> u16 {
		self.values.len() as u16
	}
}

struct TrackInformationList {
	list: Vec<TrackInformation>
}


impl TrackInformationList {
	fn from_buffer_and_disc_information(buffer: &[u8], disc_info: &DiscInformation) -> TrackInformationList {

		let mut consummed_bytes:usize = 0;
		let mut list = Vec::new();

		for track_number in 0..disc_info.number_of_tracks{
			for side_nb in 0..disc_info.number_of_sides {
				let current_track_size = disc_info.track_length(track_number, side_nb) as usize;
				assert!(current_track_size>255);
				println!("Track: {} - Side: {} - Length: 0x{:x}/{}", track_number, side_nb, current_track_size, current_track_size);
				let track_buffer = &buffer[consummed_bytes as usize ..(consummed_bytes+current_track_size) as usize];
				list.push(TrackInformation::from_buffer(track_buffer));


				consummed_bytes += current_track_size;
			}
		}

		TrackInformationList {
			list
		}

	}


}



pub struct ExtendedDsk {
	disc_information_bloc: DiscInformation,
	track_list: TrackInformationList
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

		let disc_info = DiscInformation::from_buffer(&buffer[..256]);

		println!("Disc info {:?} / total {} / nb_tracks {}", disc_info, disc_info.total_tracks_lengths(), disc_info.number_of_distinct_tracks());
		let track_list = TrackInformationList::from_buffer_and_disc_information(&buffer[256..], & disc_info);

		Ok(ExtendedDsk {
			disc_information_bloc: disc_info,
			track_list
		})


	}


	/// Search and returns the appropriate sector
	pub fn sector(&self, track: u8, sector: u8, side: u8) -> Option<&Sector> {

		let idx = if self.disc_information_bloc.is_double_sided() {
			track as usize * 2 + side as usize
		}
		else {
			assert_eq!(side, 0);
			track as usize
		};

		self.track_list.list[idx].sector(sector)
	}


	/// Return the concatenated vlaues of several consecutive sectors
	pub fn sectors_bytes(&self, track: u8, sector: u8, nb_sectors: u8, side: u8) -> Option<Vec<u8>> {
		let mut res = Vec::new();

		for count in 0..nb_sectors {
			match self.sector(track, sector+count, side) {
				None => return None,
				Some(s) => {
					res.extend(s.values.iter())
				}
			}

		}

		Some(res)
	}
}