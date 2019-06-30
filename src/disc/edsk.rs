// http://www.cpcwiki.eu/index.php/Format:DSK_disk_image_file_format

use itertools::zip;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::string::ToString;

use delegate::delegate;
use getset::Getters;
use bitflags::bitflags;

/// Computes the sector size as expected by the FDC from a human readable sector size
pub fn convert_real_sector_size_to_fdc_sector_size(mut size: u16) -> u8 {
    let mut n = 0;
    while size > 0x80 {
        size = size >> 1;
        n += 1
    }

    n as _
}

/// Computes the sector size as expected by a human from the FDC
pub fn convert_fdc_sector_size_to_real_sector_size(size: u8) -> u16 {
    0x80 << size
}

#[derive(Debug, PartialEq, Copy, Clone, Ord, PartialOrd, Eq)]
/// Symbolises the head of a disc.
pub enum Head {
    /// Side A of the disc for double sided discs
    HeadA,
    /// Side B of the disc for double sided discs
    HeadB,
    /// Side not specified for single sided discs. Should be deprecated in favor of HeadA
    Unspecified,
}

#[allow(missing_docs)] 
impl From<u8> for Head {
    fn from(val: u8) -> Head {
        match val {
            0 => Head::HeadA,
            1 => Head::HeadB,
            _ => Head::Unspecified,
        }
    }
}

#[allow(missing_docs)] 
impl Into<u8> for Head {
    fn into(self) -> u8 {
        match self {
            Head::HeadA => 0,
            Head::HeadB => 1,
            Head::Unspecified => 0,
        }
    }
}

#[allow(missing_docs)] 
impl Into<u8> for &Head {
    fn into(self) -> u8 {
        match *self {
            Head::HeadA => 0,
            Head::HeadB => 1,
            Head::Unspecified => 0,
        }
    }
}

/// Disc image files consist of a 0x100-byte disc info block and for each track a 0x100-byte track info block, followed by the data for every sector in that track. The new extended disk format is intended for some copy protected disks. Parts which are new in the extended format are marked with "*E*" (from our "Extended DISK Format Proposal, Rev.5").
///
///
/// The Disc Information block
/// Byte (Hex): 	Meaning:
/// 00 - 21 	"MV - CPCEMU Disk-File\r\nDisk-Info\r\n"
/// ("MV - CPC" is characteristic)
///
/// *E* -- "EXTENDED CPC DSK File\r\n\Disk-Info\r\n"
/// ("EXTENDED" is characteristic)
/// 22 - 2F 	unused (0)
///
/// *E* -- DSK creator (name of the utility)
/// (no ending \0 needed!)
/// 30 	number of tracks (40, 42, maybe 80)
/// 31 	number of heads (1 or 2)
/// 32 - 33 	size of one track (including 0x100-byte track info)
/// With 9 sectors * 0x200 bytes + 0x100 byte track info = 0x1300.
///
/// *E* -- unused (0)
/// 34 - FF 	unused (0)
///
/// *E* -- high bytes of track sizes for all tracks
/// (computed in the same way as 32-33 for the normal format).
///
///     For single sided formats the table contains track sizes of just one side, otherwise for two alternating sides.
///     A size of value 0 indicates an unformatted track.
///     Actual track data length = table value * 256.
///     Keep in mind that the image contains additional 256 bytes for each track info.
#[derive(Getters, Debug, Default, PartialEq, Clone)]
pub struct DiscInformation {
    /// Specific to edsk
    #[get = "pub"]
    pub(crate) creator_name: String,
    /// Number of tracks
    #[get = "pub"]
    pub(crate) number_of_tracks: u8,
    /// Number of heads
    #[get = "pub"]
    pub(crate) number_of_heads: u8,
    #[get = "pub"]
    /// high bytes of track sizes for all tracks
    pub(crate) track_size_table: Vec<u8>, // XXX for standard DSK only one value is provided It should be duplicated there
}

#[allow(missing_docs)] 
impl DiscInformation {
    fn creator_name_as_bytes(&self) -> [u8; 14] {
        let mut data = [0; 14];
        for (idx, byte) in self.creator_name.as_bytes()[0..14].iter().enumerate() {
            data[idx] = *byte;
        }
        data
    }

    /// Build an eDSK from a buffer of bytes
    ///  TODO manage the case of standard dsk
    pub fn from_buffer(buffer: &[u8]) -> DiscInformation {
        assert_eq!(buffer.len(), 256);
        assert_eq!(
            String::from_utf8_lossy(&buffer[..34]).to_ascii_uppercase(),
            "EXTENDED CPC DSK File\r\nDisk-Info\r\n".to_ascii_uppercase()
        );

        let creator_name = String::from_utf8_lossy(&buffer[0x22..=0x2f]);
        let number_of_tracks = buffer[0x30];
        let number_of_heads = buffer[0x31];
        let track_size_table = &buffer[0x34..(0x34 + number_of_tracks * number_of_heads) as usize];

        assert!(number_of_heads == 1 || number_of_heads == 2);

        DiscInformation {
            creator_name: creator_name.to_string(),
            number_of_tracks,
            number_of_heads,
            track_size_table: track_size_table.to_vec(),
        }
    }

    fn to_buffer(&self, buffer: &mut Vec<u8>) {
        buffer.extend_from_slice("EXTENDED CPC DSK File\r\nDisk-Info\r\n".as_bytes());
        assert_eq!(buffer.len(), 34);

        buffer.extend_from_slice(&self.creator_name_as_bytes());
        assert_eq!(buffer.len(), 34 + 14);

        buffer.push(self.number_of_tracks);
        buffer.push(self.number_of_heads);
        assert_eq!(buffer.len(), 34 + 14 + 1 + 1);

        // XXX missing size of a track
        buffer.push(0);
        buffer.push(0);
        assert_eq!(buffer.len(), 34 + 14 + 1 + 1 + 2);

        buffer.extend_from_slice(&self.track_size_table);
        assert_eq!(
            buffer.len(),
            34 + 14 + 1 + 1 + 2 + self.track_size_table.len()
        );

        assert!(buffer.len() <= 256);
        // ensure we use 256 bytes
        buffer.resize(256, 0x00);
        assert_eq!(buffer.len(), 256);

        // DEBUG mode XXX To remove
        let from_buffer = Self::from_buffer(&buffer);
        assert_eq!(self, &from_buffer);
    }

    /// Check if the dsk is double sided
    pub fn is_double_head(&self) -> bool {
        self.number_of_heads == 2
    }

    /// Check if the dsk is single sided
    pub fn is_single_head(&self) -> bool {
        !self.is_double_head()
    }

    /// Returns the length of the track including the track information block
    pub fn track_length(&self, track: u8, head: u8) -> u16 {
        assert!(head < self.number_of_heads);

        let track = track as usize;
        let head = head as usize;
        let idx = if self.is_single_head() {
            track
        } else {
            track * 2 + head
        };

        self.track_length_at_idx(idx)
    }

    /// Check if the wanted track is formatted
    pub fn is_formatted(&self, track: u8, head: u8) -> bool {
        self.track_length(track, head) > 0
    }

    /// Get the lenght of the required track
    pub fn track_length_at_idx(&self, idx: usize) -> u16 {
        256 * u16::from(self.track_size_table[idx])
    }

    /// Sum all the tracks lenght
    pub fn total_tracks_lengths(&self) -> usize {
        (0..self.number_of_distinct_tracks())
            .map(|idx: usize| self.track_length_at_idx(idx) as usize)
            .sum::<usize>()
    }

    /// Provide the number of different tracks
    pub fn number_of_distinct_tracks(&self) -> usize {
        self.track_size_table.len()
    }
}

/// Byte (Hex) 	Meaning:
/// 00 - 0C 	Track-Info\r\n
/// 0D - 0F 	unused (0)
/// 10 	track number (0 to number of tracks-1)
/// 11 	head number (0 or 1)
/// 12 - 13 	unused (0)
/// Format track parameters:
/// 14 	BPS (bytes per sector) (2 for 0x200 bytes)
/// 15 	SPT (sectors per track) (9, at the most 18)
/// 16 	GAP#3 format (gap for formatting; 0x4E)
/// 17 	Filling byte (filling byte for formatting; 0xE5)
/// Sector info (for every sector at a time):
/// 18+i 	track number (sector ID information)
/// 19+i 	head number (sector ID information)
/// 1A+i 	sector number (sector ID information)
/// 1B+i 	BPS (sector ID information)
/// 1C+i 	state 1 error code (0)
/// 1D+i 	state 2 error code (0)
/// 1E+i,1F+i 	unused (0)
///
/// *E* -- sector data length in bytes (little endian notation).
/// This allows different sector sizes in a track.
/// It is computed as (0x0080 << real_BPS).
///
/// Annotations:
///
///   - The sector data must follow the track information block in the order of the sector IDs. No track or sector may be omitted.
///   - With double sided formats, the tracks are alternating, e.g. track 0 head 0, track 0 head 1, track 1 ...
///   - Use CPCTRANS to copy CPC discs into this format.

#[allow(missing_docs)] 
#[derive(Getters, Debug, Default, PartialEq, Clone)]
pub struct TrackInformation {
    /// track number (0 to number of tracks-1)
    #[get = "pub"]
    pub(crate) track_number: u8,
    /// head number (0 or 1)
    #[get = "pub"]
    pub(crate) head_number: u8,
    #[get = "pub"]
    /// BPS (bytes per sector) (2 for 0x200 bytes)
    pub(crate) sector_size: u8, // XXX check if really needed to be stored
    /// SPT (sectors per track) (9, at the most 18)
    #[get = "pub"]
    pub(crate) number_of_sectors: u8,
    /// GAP#3 format (gap for formatting; 0x4E)
    #[get = "pub"]
    pub(crate) gap3_length: u8,
    /// Filling byte (filling byte for formatting; 0xE5)
    #[get = "pub"]
    pub(crate) filler_byte: u8,
    /// Returns the data rate
    #[get = "pub"]
    pub(crate) data_rate: DataRate,
    /// Returns the recordingmode
    #[get = "pub"]
    pub(crate) recording_mode: RecordingMode,
    /// List of sectors
    #[get = "pub"]
    pub(crate) sector_information_list: SectorInformationList,
    /// The size taken by the track + header in the dsk. This is a duplicated information obtained in the DiscInformation bloc
    #[get = "pub"]
    pub(crate) track_size: u16,
}

#[allow(missing_docs)] 
impl TrackInformation {
    /// TODO find a nicer (with Either ?) way to manipulate unformatted tracks
    pub fn unformatted() -> TrackInformation {
        Default::default()
    }

    /// Returns the real size of the track (i.e. after removing the header)
    pub fn real_track_size(&self) -> u16 {
        self.track_size() - 256
    }
}

#[allow(missing_docs)] 
impl TrackInformation {
    #[deprecated(
        note = "Note sure it should be used as each sector store this information and different sizes are possible"
    )]
    pub fn sector_size_human_readable(&self) -> u16 {
        convert_fdc_sector_size_to_real_sector_size(self.sector_size)
    }

    /// Returns the ID of the sector following this one
    pub fn next_sector_id(&self, sector: u8) -> Option<u8> {
        for idx in 0..(self.number_of_sectors() - 1) {
            let current_sector = &self.sector_information_list.sectors[idx as usize];
            let next_sector = &self.sector_information_list.sectors[idx as usize + 1];

            if *current_sector.sector_id() == sector {
                return Some(*next_sector.sector_id());
            }
        }

        None
    }

    /// Fail if the track has no sector
    pub fn min_sector(&self) -> u8 {
        self.sector_information_list
            .sectors()
            .iter()
            .map(|s| s.sector_information_bloc.sector_id)
            .min()
            .unwrap()
    }

    /// Compute the sum of data contained by all the sectors.
    /// Only serves for debug purposes
    pub fn data_sum(&self) -> usize {
        self.sector_information_list
            .sectors
            .iter()
            .map(|s| s.data_sum())
            .sum::<usize>()
    }

    pub fn corresponds_to(&self, track: u8, head: u8) -> bool {
        self.track_number == track && self.head_number == head
    }

    pub fn from_buffer(buffer: &[u8]) -> TrackInformation {
        if String::from_utf8_lossy(&buffer[..0xc]).to_ascii_uppercase()
            != "Track-info\r\n".to_ascii_uppercase()
        {
            panic!(
                "Track buffer does not seem coherent\n{:?}...",
                &buffer[..0xc]
            );
        }

        let track_size = buffer.len() as u16;
        let track_number = buffer[0x10];
        let head_number = buffer[0x11];
        let sector_size = buffer[0x14];
        let number_of_sectors = buffer[0x15];
        let gap3_length = buffer[0x16];
        let filler_byte = buffer[0x17];
        let data_rate: DataRate = buffer[0x12].into();
        let recording_mode = buffer[0x13].into();

        println!(
            "Track {} Head {} sector_size {} nb_sectors {} gap length {:x}, filler_byte {:x}",
            track_number, head_number, sector_size, number_of_sectors, gap3_length, filler_byte
        );
        let sector_information_list =
            SectorInformationList::from_buffer(&buffer[0x18..], number_of_sectors);

        let track_info = TrackInformation {
            track_number,
            head_number,
            sector_size,
            number_of_sectors,
            gap3_length,
            filler_byte,
            data_rate,
            recording_mode,
            sector_information_list,
            track_size,
        };

        assert!(track_info.track_size != 0);

        assert_eq!(
            track_info.real_track_size(),
            track_info.compute_track_size() as u16,
            "Wrong track_info {:?}",
            track_info
        );
        track_info
    }

    /// http://www.cpcwiki.eu/index.php/Format:DSK_disk_image_file_format#TRACK_INFORMATION_BLOCK_2
    ///
    /// offset 	description 	bytes
    /// 00 - 0b 	"Track-Info\r\n" 	12
    /// 0c - 0f 	unused 	4
    /// 10 	track number 	1
    /// 11 	Head number 	1
    /// 12 - 13 	unused 	2
    /// 14 	sector size 	1
    /// 15 	number of sectors 	1
    /// 16 	GAP#3 length 	1
    /// 17 	filler byte 	1
    /// 18 - xx 	Sector Information List 	xx
    ///
    /// Extensions
    /// offset 	description 	bytes
    /// 12 	Data rate. (See note 1 and note 3) 	1
    /// 13 	Recording mode. (See note 2 and note 3) 	1
    pub fn to_buffer(&self, buffer: &mut Vec<u8>) {
        let start_size = buffer.len();

        // low byte MUST be null
        assert_eq!(start_size % 256, 0);

        // 00 - 0b 	"Track-Info\r\n" 	12
        buffer.extend_from_slice(&"Track-Info\r\n".as_bytes()[..12]);
        assert_eq!(buffer.len() - start_size, 12);

        // 0c - 0f 	unused 	4
        buffer.push(0);
        buffer.push(0);
        buffer.push(0);
        buffer.push(0);

        // 10 	track number 	1
        buffer.push(self.track_number);

        // 11 	Head number 	1
        buffer.push(self.head_number);

        // 12 	Data rate. (See note 1 and note 3) 	1
        buffer.push(self.data_rate.clone().into());

        // 13 	Recording mode. (See note 2 and note 3) 	1
        buffer.push(self.recording_mode.clone().into());

        // 14 	sector size 	1
        buffer.push(self.sector_size);

        // 15 	number of sectors 	1
        buffer.push(self.number_of_sectors);

        // 16 	GAP#3 length 	1
        buffer.push(self.gap3_length);

        // 17 	filler byte 	1
        buffer.push(self.filler_byte);

        assert_eq!(buffer.len() - start_size, 0x18);

        // 18 - xx 	Sector Information List 	x
        // Inject sectors information list
        self.sector_information_list.sectors.iter().for_each(|s| {
            s.sector_information_bloc.to_buffer(buffer);
        });

        // Ensure next position has a low byte value of 0
        let added_bytes = buffer.len() - start_size;
        let missing_bytes = 256 - added_bytes;
        buffer.resize(buffer.len() + missing_bytes, 0);
        assert_eq!(buffer.len() % 256, 0);

        // Inject sectors information data
        self.sector_information_list.sectors.iter().for_each(|s| {
            buffer.extend_from_slice(&s.values);
        });

        /*
            // TODO find why this coded was previously present as it raise issues
            // Ensure the size is correct
            let added_bytes = (buffer.len() - start_size) as u16;
            assert!(
                added_bytes <= self.track_size,
                format!("{} != {}", added_bytes, self.track_size)
            );
            let missing_bytes = self.track_size - added_bytes;
            if missing_bytes != 0 {
                buffer.resize(buffer.len() + missing_bytes as usize, 0);
            }
        */
        // Add padding
        assert!(buffer.len() % 256 == 0);
    }

    /// TODO remove this method or set it private
    pub fn total_size(&self) -> usize {
        self.sector_information_list
            .sectors
            .iter()
            .map(|info| dbg!(info.sector_information_bloc.data_length) as usize)
            .sum()
    }

    /// Track size has it should be written in the DSK
    pub fn compute_track_size(&self) -> usize {
        let size = self.total_size();
        if size % 256 == 0 {
            size
        } else {
            let mut s = dbg!(size);
            // TODO implement an efficient version
            while s % 256 != 0 {
                s = s + 1;
            }
            s
        }
    }

    delegate! {
        target self.sector_information_list {
            pub fn sector(&self, sector_id: u8) -> Option<&Sector>;
            pub fn sector_mut(&mut self, sector_id: u8) -> Option<&mut Sector>;
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(missing_docs)] 
pub enum DataRate {
    Unknown = 0,
    SingleOrDoubleDensity = 1,
    HighDensity = 2,
    ExtendedDensity = 3,
}

#[allow(missing_docs)] 
impl Default for DataRate {
    fn default() -> Self {
        DataRate::Unknown
    }
}

#[allow(missing_docs)] 
impl From<u8> for DataRate {
    fn from(b: u8) -> DataRate {
        match b {
            0 => DataRate::Unknown,
            1 => DataRate::SingleOrDoubleDensity,
            2 => DataRate::HighDensity,
            3 => DataRate::ExtendedDensity,
            _ => unreachable!(),
        }
    }
}

#[allow(missing_docs)] 
impl Into<u8> for DataRate {
    fn into(self) -> u8 {
        match self {
            DataRate::Unknown => 0,
            DataRate::SingleOrDoubleDensity => 1,
            DataRate::HighDensity => 2,
            DataRate::ExtendedDensity => 3,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(missing_docs)] 
pub enum RecordingMode {
    Unknown = 0,
    FM = 1,
    MFM = 2,
}

#[allow(missing_docs)] 
impl Default for RecordingMode {
    fn default() -> Self {
        RecordingMode::Unknown
    }
}

#[allow(missing_docs)] 
impl From<u8> for RecordingMode {
    fn from(b: u8) -> RecordingMode {
        match b {
            0 => RecordingMode::Unknown,
            1 => RecordingMode::FM,
            2 => RecordingMode::MFM,
            _ => unreachable!(),
        }
    }
}

#[allow(missing_docs)] 
impl Into<u8> for RecordingMode {
    fn into(self) -> u8 {
        match self {
            RecordingMode::Unknown => 0,
            RecordingMode::FM => 1,
            RecordingMode::MFM => 2,
            _ => unreachable!(),
        }
    }
}

#[derive(Getters, Debug, Default, PartialEq, Clone, Copy)]
#[allow(missing_docs)] 
pub struct SectorInformation {
    /// track (equivalent to C parameter in NEC765 commands)
    #[get = "pub"]
    pub(crate) track: u8,
    /// Head (equivalent to H parameter in NEC765 commands)
    #[get = "pub"]
    pub(crate) head: u8,
    /// sector ID (equivalent to R parameter in NEC765 commands)
    #[get = "pub"]
    pub(crate) sector_id: u8,
    /// sector size (equivalent to N parameter in NEC765 commands)
    #[get = "pub"]
    pub(crate) sector_size: u8,
    /// FDC status register 1 (equivalent to NEC765 ST1 status register)
    #[get = "pub"]
    pub(crate) fdc_status_register_1: u8,
    /// FDC status register 2 (equivalent to NEC765 ST2 status register)
    #[get = "pub"]
    pub(crate) fdc_status_register_2: u8,
    /// actual data length in bytes
    #[get = "pub"]
    pub(crate) data_length: u16, // in bytes, little endian notation
}

#[allow(missing_docs)] 
impl SectorInformation {
    /// Return the real size of the sector
    pub fn len(&self) -> usize {
        self.sector_size as usize * 256
    }

    pub fn from_buffer(buffer: &[u8]) -> SectorInformation {
        let info = SectorInformation {
            track: buffer[0x00],
            head: buffer[0x01],
            sector_id: buffer[0x02],
            sector_size: buffer[0x03],
            fdc_status_register_1: buffer[0x04],
            fdc_status_register_2: buffer[0x05],
            data_length: u16::from(buffer[0x06]) + (u16::from(buffer[0x07]) * 256),
        };
        info
    }

    /// 00 	track (equivalent to C parameter in NEC765 commands) 	1
    /// 01 	Head (equivalent to H parameter in NEC765 commands) 	1
    /// 02 	sector ID (equivalent to R parameter in NEC765 commands) 	1
    /// 03 	sector size (equivalent to N parameter in NEC765 commands) 	1
    /// 04 	FDC status register 1 (equivalent to NEC765 ST1 status register) 	1
    /// 05 	FDC status register 2 (equivalent to NEC765 ST2 status register) 	1
    /// 06 - 07 	actual data length in bytes 	2
    pub fn to_buffer(&self, buffer: &mut Vec<u8>) {
        buffer.push(self.track);
        buffer.push(self.head);
        buffer.push(self.sector_id);
        buffer.push(self.sector_size);
        buffer.push(self.fdc_status_register_1);
        buffer.push(self.fdc_status_register_2);

        // Specific for extended image
        buffer.push((self.data_length % 256) as u8);
        buffer.push((self.data_length / 256) as u8);
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
#[allow(missing_docs)] 
pub struct SectorInformationList {
    //sectors: Vec<Sector>
    pub(crate) sectors: Vec<Sector>,
}

#[allow(missing_docs)] 
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
        dbg!(&list_info);

        // Get the data
        consummed_bytes = 256 - 0x18; // Skip the unused bytes
        for sector in list_info.iter() {
            let current_sector_size = sector.data_length as usize;
            let current_buffer = &buffer[consummed_bytes..consummed_bytes + current_sector_size];
            let sector_bytes = current_buffer.to_vec();
            assert_eq!(sector_bytes.len(), current_sector_size);
            list_data.push(sector_bytes);
            consummed_bytes += current_sector_size;
        }

        // merge them
        let info_drain = list_info.drain(..);
        let data_drain = list_data.drain(..);
        let sectors = zip(info_drain, data_drain)
            .map(|(info, data)| {
                assert_eq!(info.data_length as usize, data.len());
                Sector {
                    sector_information_bloc: info,
                    values: data,
                }
            })
            .collect::<Vec<Sector>>();

        SectorInformationList { sectors }
    }

    pub fn sector(&self, sector_id: u8) -> Option<&Sector> {
        self.sectors
            .iter()
            .find(|sector| sector.sector_information_bloc.sector_id == sector_id)
    }

    /// Returns the sector that correspond to the requested id
    pub fn sector_mut(&mut self, sector_id: u8) -> Option<&mut Sector> {
        self.sectors
            .iter_mut()
            .find(|sector| sector.sector_information_bloc.sector_id == sector_id)
    }

    /// Fill the information list with sectors corresponding to the provided arguments
    pub fn fill_with(
        &mut self,
        ids: &[u8],
        heads: &[u8],
        track_number: u8,
        sector_size: u8,
        filler_byte: u8,
    ) {
        assert_eq!(ids.len(), heads.len());
        assert_eq!(self.len(), 0);

        for idx in 0..ids.len() {
            let mut sector = Sector::default();

            sector.sector_information_bloc.track = track_number;
            sector.sector_information_bloc.sector_size = sector_size;
            sector.sector_information_bloc.sector_id = ids[idx];
            sector.sector_information_bloc.head = heads[idx];

            let data_size = convert_fdc_sector_size_to_real_sector_size(
                sector.sector_information_bloc.sector_size,
            ) as usize;
            sector.values.resize(data_size, filler_byte);
            sector.sector_information_bloc.data_length = sector.values.len() as u16;

            self.add_sector(sector);
        }
    }
}

bitflags! {
    struct FdcStatusRegister1: u8 {
        const END_OF_CYLINDER = 1<<7;
        const DATA_ERROR = 1<<5;
        const NO_DATA = 1<<2;
        const MISSING_ADDRESS_MARK = 1<<0;
    }
}

bitflags! {
    struct FdcStatusRegister2: u8 {
        const CONTROL_MARK = 1<<5;
        const DATA_ERROR_IN_DATA_FIELD = 1<<5;
        const MISSING_ADDRESS_MARK_IN_DATA_FIELD = 1<<0;
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
#[allow(missing_docs)] 
pub struct Sector {
    #[get]
    pub(crate) sector_information_bloc: SectorInformation,
    /// Some DSK seem to have a vector with not the right size. In tFor this reason, it is better to not give access to it directly
    pub(crate) values: Vec<u8>,
}

#[allow(missing_docs)] 
impl Sector {
    /// Number of bytes stored in the sector
    pub fn real_size(&self) -> u16 {
        self.values.len() as u16
    }

    pub fn len(&self) -> u16 {
        self.sector_information_bloc.len() as u16
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn data_sum(&self) -> usize {
        self.values().iter().map(|&v| v as usize).sum::<usize>()
    }

    pub fn values(&self) -> &[u8] {
        &self.values[..self.len() as usize]
    }

    pub fn values_mut(&mut self) -> &mut [u8] {
        let idx = dbg!(self.len() as usize);
        &mut self.values[..idx]
    }

    /// Set all the values stored in the sector
    pub fn set_values(&mut self, data: &[u8]) -> Result<(), String> {
        if (data.len() as u16) < self.len() {
            return Err(format!(
                "You cannot insert {} bytes in a sector of size {}.",
                data.len(),
                self.len()
            ));
        }

        if (data.len() as u16) > self.len() {
            return Err(format!(
                "Smaller data of {} bytes to put in a sector of size {}.",
                data.len(),
                self.len()
            ));
        }

        self.values[..].clone_from_slice(data);
        Ok(())
    }

    delegate! {
        target self.sector_information_bloc {
            pub fn sector_id(&self) -> &u8;
        }
    }
}

#[derive(Default, PartialEq, Debug, Clone)]
#[allow(missing_docs)] 
pub struct TrackInformationList {
    pub(crate) list: Vec<TrackInformation>,
}

#[allow(missing_docs)] 
impl TrackInformationList {
    fn from_buffer_and_disc_information(
        buffer: &[u8],
        disc_info: &DiscInformation,
    ) -> TrackInformationList {
        let mut consummed_bytes: usize = 0;
        let mut list = Vec::new();

        for track_number in 0..disc_info.number_of_tracks {
            for head_nb in 0..disc_info.number_of_heads {
                // Size of the track data + header
                let current_track_size = disc_info.track_length(track_number, head_nb) as usize;
                let track_buffer = &buffer
                    [consummed_bytes..(consummed_bytes + current_track_size)];
                if current_track_size > 0 {
                    list.push(TrackInformation::from_buffer(track_buffer));
                } else {
                    eprintln!("Track {} is unformatted", track_number);
                    list.push(TrackInformation::unformatted());
                }
                consummed_bytes += current_track_size;
            }
        }

        TrackInformationList { list }
    }

    /// Write the track list in the given buffer
    fn to_buffer(&self, buffer: &mut Vec<u8>) {
        for track in self.list.iter() {
            let _added_bytes = track.to_buffer(buffer);
        }
    }

    /// Add an empty track and return it. It is up to the caller to properly feed it
    pub fn add_empty_track(&mut self) -> &mut TrackInformation {
        let track = TrackInformation::default();
        self.list.push(track);
        self.list.last_mut().unwrap()
    }

    /// Returns the tracks for the given head
    pub fn tracks_for_head(&self, head: Head) -> impl Iterator<Item = &TrackInformation> {
        let head: u8 = head.into();
        self.list
            .iter()
            .filter(move |info| info.head_number == head)
    }

    /// Returns the track following this one
    pub fn next_track(&self, track: &TrackInformation) -> Option<&TrackInformation> {
        for idx in 0..(self.list.len() - 1) {
            let current_track = &self.list[idx];
            let next_track = &self.list[idx + 1];

            if current_track == track  {
                return Some(next_track);
            }
        }

        None
    }
}

#[derive(Default, PartialEq, Debug, Clone)]
#[allow(missing_docs)] 
pub struct ExtendedDsk {
    pub(crate) disc_information_bloc: DiscInformation,
    pub(crate) track_list: TrackInformationList,
}

#[allow(missing_docs)] 
impl ExtendedDsk {
    /// open an extended dsk from an existing file
    pub fn open<P>(path: P) -> io::Result<ExtendedDsk>
    where
        P: AsRef<Path>,
    {
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

        println!(
            "Disc info {:?} / total {} / nb_tracks {}",
            disc_info,
            disc_info.total_tracks_lengths(),
            disc_info.number_of_distinct_tracks()
        );
        let track_list =
            TrackInformationList::from_buffer_and_disc_information(&buffer[256..], &disc_info);

        ExtendedDsk {
            disc_information_bloc: disc_info,
            track_list,
        }
    }

    /// Add the file in consecutive sectors
    pub fn add_file_sequentially(
        &mut self,
        head: u8,
        track: u8,
        sector: u8,
        buffer: &[u8],
    ) -> Result<(u8, u8, u8), String> {
        let mut pos = (head, track, sector);
        let mut consummed = 0;
        while consummed < buffer.len() {
            let current_sector = self
                .sector_mut(pos.0.clone(), pos.1.clone(), pos.2.clone())
                .ok_or("Sector not found".to_owned())?;

            let sector_size = current_sector.len() as usize;
            let current_data = &buffer[consummed..consummed + sector_size];
            current_sector.set_values(current_data)?;
            consummed += sector_size;

            let next_pos = self
                .next_position(pos.0.clone(), pos.1.clone(), pos.2.clone())
                .ok_or("No more position available".to_owned())?;
            pos = next_pos;
        }

        Ok(pos)
    }

    /// Compute the next sector position if possible
    /// XXX check if Head should be the logic or physical one
    /// XXX the two behaviors are mixed there ...
    fn next_position(&self, head: u8, track: u8, sector: u8) -> Option<(u8, u8, u8)> {
        // Retrieve the current track and exit if does not exist
        let current_track = self.get_track_information(
            head.clone(),
            /// Physical
            track,
        )?;

        // Return the next sector if exist
        if let Some(next_sector) = current_track.next_sector_id(sector.clone()) {
            return Some((head.clone(), track.clone(), next_sector));
        }

        // Search the next track
        let next_track = self.track_list.next_track(&current_track)?;

        Some((
            *next_track.head_number(), // XXX  logical
            *next_track.track_number(),
            next_track.min_sector(),
        ))
    }

    /// Save the dsk in a file one disc
    pub fn save<P>(&self, path: P) -> io::Result<()>
    where
        P: AsRef<Path>,
    {
        let mut file_buffer = File::create(path)?;
        let mut memory_buffer = Vec::new();
        self.to_buffer(&mut memory_buffer);
        file_buffer.write_all(&memory_buffer)
    }

    /// Write the dsk in the provided buffer
    pub fn to_buffer(&self, buffer: &mut Vec<u8>) {
        self.disc_information_bloc.to_buffer(buffer);
        self.track_list.to_buffer(buffer);
    }

    pub fn is_double_head(&self) -> bool {
        self.disc_information_bloc.is_double_head()
    }

    // We assume we have the same number of tracks per Head.
    // Need to be modified the day ot will not be the case.
    pub fn nb_tracks_per_head(&self) -> u8 {
        let val = if self.disc_information_bloc.is_single_head() {
            self.track_list.list.len()
        } else {
            self.track_list.list.len() / 2
        };
        val as u8
    }

    #[deprecated]
    pub fn nb_tracks_per_side(&self) -> u8 {
        self.nb_tracks_per_head()
    }

    pub fn nb_heads(&self) -> u8 {
        self.disc_information_bloc.number_of_heads
    }

    pub fn get_track_information<S: Into<Head>>(
        &self,
        head: S,
        track: u8,
    ) -> Option<&TrackInformation> {
        let idx = self.get_track_idx(head.into(), track);
        self.track_list.list.get(idx)
    }

    pub fn get_track_information_mut(
        &mut self,
        head: Head,
        track: u8,
    ) -> Option<&mut TrackInformation> {
        let idx = self.get_track_idx(head.into(), track);
        self.track_list.list.get_mut(idx)
    }

    /// Search and returns the appropriate sector
    pub fn sector<S: Into<Head>>(&self, head: S, track: u8, sector_id: u8) -> Option<&Sector> {
        self.get_track_information(head.into(), track)
            .and_then(|track| track.sector(sector_id))
    }

    /// Search and returns the appropriate mutable sector
    pub fn sector_mut<S: Into<Head>>(
        &mut self,
        head: S,
        track: u8,
        sector_id: u8,
    ) -> Option<&mut Sector> {
        self.get_track_information_mut(head.into(), track)
            .and_then(|track| track.sector_mut(sector_id))
    }

    fn get_track_idx(&self, head: Head, track: u8) -> usize {
        if self.disc_information_bloc.is_double_head() {
            let head = match head {
                Head::HeadA => 0,
                Head::HeadB => 1,
                Head::Unspecified => panic!("You must specify a Head for a double Headed disc."),
            };
            track as usize * 2 + head
        } else {
            if let Head::HeadB = head {
                panic!("You cannot select Head B in a single Headd disc");
            }
            track as usize
        }
    }

    /// Return the concatenated values of several consecutive sectors
    pub fn sectors_bytes<S: Into<Head>>(
        &self,
        head: S,
        track: u8,
        sector_id: u8,
        nb_sectors: u8,
    ) -> Option<Vec<u8>> {
        let mut res = Vec::new();
        let head = head.into();

        for count in 0..nb_sectors {
            match self.sector(head.clone(), track, sector_id + count) {
                None => return None,
                Some(s) => res.extend(s.values.iter()),
            }
        }

        Some(res)
    }

    /// Return all the bytes of the given track
    pub fn track_bytes<H: Into<Head>>(&self, head: H, track: u8) -> Option<Vec<u8>> {
        match self.get_track_information(head, track) {
            Some(track) => {
                let mut bytes = Vec::new();
                for sector in track.sector_information_list.sectors() {
                    bytes.extend(sector.values().iter());
                }
                Some(bytes)
            }
            _ => None,
        }
    }

    /// Compute the datasum for the given track
    pub fn data_sum(&self, head: Head) -> usize {
        self.track_list
            .tracks_for_head(head)
            .map(|t| t.data_sum())
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
    pub fn min_sector<S: Into<Head>>(&self, _size: S) -> u8 {
        self.tracks().iter().map(|t| t.min_sector()).min().unwrap()
    }
}
