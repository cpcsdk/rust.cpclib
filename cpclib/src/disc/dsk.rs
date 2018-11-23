// http://www.cpcwiki.eu/index.php/Format:DSK_disk_image_file_format

struct DiscInformation {
	creator_name: String, 
	number_of_tracks: u8, 
	number_of_sides: u8, 
	track_size_table: Vec<u8> // XXX dor standard DSK only one value is provided It should be duplicated there
}

struct TrackInformation {
	track_number: u8,
	side_number: u8,
	sector_size: u8, // XXX check if really needed to be stored
	number_of_sectors: u8,
	gap3_length: u8,
	filler_byte: u8,
	data_rate: DataRate,
	recording_mode: u8,
	sector_information_list: Vec<SectorInformation>
}


enum DataRate {
	Unknown = 0,
	SingleOrDoubleDensity = 1,
	HighDensity = 2,
	ExtendedDensity = 3
}

enum RecordingMode {
	Unknown = 0,
	FM = 1,
	MFM = 2
}

struct SectorInformation {
	track: u8,
	side: u8,
	sector_id: u8,
	sector_size: u8,
	fdc_status_register_1: u8,
	fdc_status_register_2: u8,
	data_length: u16, // in bytes, little endian notation
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

struct Track {
	track_information_bloc: TrackInformation,
	sectors: SectorData
}

struct SectorData {
	sectors: Vec<Sector>
}

struct TrackData {
	tracks: Vec<Track>
}

struct Sector {
	sector_information_bloc: SectorInformation,
	values: Vec<u8>
}

struct Dsk {
	disc_information_bloc: DiscInformation
}