/// Parser of the disc configuraiton used by the Arkos Loader
use nom;
use nom::types::CompleteStr;
use nom::{eol, hex_u32, space0, space1};


use itertools;
use itertools::Itertools;
use std::iter::Iterator;

use crate::disc::edsk::*;
use std::fmt;

use std::fs::File;
use std::io::Read;
use std::path::Path;

const DATA_FORMAT_CFG: &str = "
NbTrack = 40
NbSide = 1

[Track:0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39]
SectorSize = 512
Gap3 = 82
SectorID = 0xc1,0xc6,0xc2,0xc7,0xc3,0xc8,0xc4,0xc9,0xc5
sectorIDHead = 0,0,0,0,0,0,0,0,0
";

const DATA_FORMAT42_CFG: &str = "
NbTrack = 42
NbSide = 1

[Track:0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,41]
SectorSize = 512
Gap3 = 0x4e
SectorID = 0xc1,0xc6,0xc2,0xc7,0xc3,0xc8,0xc4,0xc9,0xc5
sectorIDHead = 0,0,0,0,0,0,0,0,0
";

/// Disk format configuration.
#[derive(Debug, PartialEq)]
pub struct DiscConfig {
    /// Number of tracks in the disc
    pub(crate) nb_tracks: u8,
    /// Number of sides in the disc (1 or 2)
    pub(crate) nb_sides: u8,
    /// List of tracks description
    pub(crate) track_groups: Vec<TrackGroup>,
}

impl From<&str> for DiscConfig {
    /// Generates the configuration from a &str. Panic in case of failure.
    /// The format corresponds to cpctools format from Ramlaid/Mortel.
    fn from(config: &str) -> DiscConfig {
        let (_, res) = parse_config(config.into()).unwrap();
        res
    }
}

impl DiscConfig {
    pub fn single_side_data_format() -> DiscConfig {
        DATA_FORMAT_CFG.into()
    }

    pub fn single_side_data42_format() -> DiscConfig {
        DATA_FORMAT42_CFG.into()
    }

    /// Create a configuration from the provided file
    pub fn new<P: AsRef<Path>>(p: P) -> std::io::Result<DiscConfig> {
        let mut content = String::new();
        let mut f = File::open(p.as_ref())?;
        f.read_to_string(&mut content)?;

        Ok(content.as_str().into())
    }
}

impl fmt::Display for DiscConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "NbTrack = {}", self.nb_tracks)?;
        writeln!(f, "NbSide = {}", self.nb_sides)?;

        for track_group in self.track_groups.iter() {
            write!(f, "\n{}", track_group)?;
        }

        Ok(())
    }
}

impl DiscConfig {
    /// SideA or SideB for a two sided dsk. Unspecified for a single sided disc
    pub fn track_information_for_track<S: Into<Side>>(
        &self,
        side: S,
        track: u8,
    ) -> Option<&TrackGroup> {
        let side = side.into();
        self.track_groups
            .iter()
            .find(move |info| info.side == side && info.tracks.iter().any(|&val| val == track))
    }

    pub fn track_idx_iterator(&self) -> impl Iterator<Item = (&Side, u8)> {
        let side_iterator = match self.nb_sides {
            2 => [Side::SideA, Side::SideB].iter(),
            1 => [Side::Unspecified].iter(),
            _ => unreachable!(),
        };
        let track_iterator = 0..self.nb_tracks;

        side_iterator.cartesian_product(track_iterator)
    }
}

#[derive(Debug, PartialEq)]
pub struct TrackGroup {
    /// Identifier of the tracks molded from this configuration
    pub(crate) tracks: Vec<u8>,
    /// Physical ide
    pub(crate) side: Side,
    /// Size of a sector
    pub(crate) sector_size: u16,
    pub(crate) gap3: u8,
    /// List of id of the sectors
    pub(crate) sector_id: Vec<u8>,
    /// List of logical side of the sectors
    pub(crate) sector_id_head: Vec<u8>,
}

impl fmt::Display for TrackGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let side_info = match self.side {
            Side::SideA => "-A",
            Side::SideB => "-B",
            Side::Unspecified => "",
        };
        let tracks_info = self.tracks.iter().map(|t| format!("{}", t)).join(",");
        let sector_id = self
            .sector_id
            .iter()
            .map(|t| format!("0x{:x}", t))
            .join(",");
        let sector_id_head = self
            .sector_id_head
            .iter()
            .map(|t| format!("{}", t))
            .join(",");

        writeln!(f, "[Track{}:{}]", side_info, tracks_info)?;
        writeln!(f, "SectorSize = {}", self.sector_size)?;
        writeln!(f, "Gap3 = 0x{:x}", self.gap3)?;
        writeln!(f, "SectorID = {}", sector_id)?;
        writeln!(f, "SectorIDHead = {}", sector_id_head)?;

        Ok(())
    }
}

impl TrackGroup {
    /// Return the sector size in the format expected by a DSK
    pub fn sector_size_dsk_format(&self) -> u8 {
        convert_real_sector_size_to_fdc_sector_size(self.sector_size)
    }

    pub fn sector_size_human_readable(&self) -> u16 {
        self.sector_size
    }

    pub fn gap3(&self) -> u8 {
        self.gap3
    }

    #[deprecated]
    pub fn nb_sectors(&self) -> usize {
        self.number_of_sectors()
    }

    pub fn number_of_sectors(&self) -> usize {
        self.sector_id.len()
    }
}

impl TrackInformationList {
    pub fn to_cfg(&self, double_sided: bool) -> Vec<TrackGroup> {
        let mut single = self
            .list
            .iter()
            .map(|t| t.to_cfg(double_sided))
            .collect::<Vec<_>>();

        // elements need to be sorted before using group_by
        single.sort_by_key(|item| {
            (
                item.side,
                item.sector_size,
                item.gap3,
                item.sector_id.clone(),
                item.sector_id_head.clone(),
            )
        });
        // group_by
        let mut grouped = single
            .iter()
            .group_by(|item| {
                (
                    item.side,
                    item.sector_size,
                    item.gap3,
                    item.sector_id.clone(),
                    item.sector_id_head.clone(),
                )
            })
            .into_iter()
            .map(|(k, group)| {
                let mut tracks = group.map(|item| item.tracks[0]).collect::<Vec<u8>>();
                tracks.sort();
                TrackGroup {
                    tracks,
                    side: k.0,
                    sector_size: k.1,
                    gap3: k.2,
                    sector_id: k.3,
                    sector_id_head: k.4,
                }
            })
            .collect::<Vec<TrackGroup>>();

        // Sorted the result
        grouped.sort_by_key(|item| (item.side, item.tracks[0]));

        grouped
    }
}

/// Extend TrackInformation with the ability to extract its configuration
impl TrackInformation {
    pub fn to_cfg(&self, double_sided: bool) -> TrackGroup {
        let tracks = vec![self.track_number];
        let side: Side = if double_sided {
            self.side_number.into()
        } else {
            Side::Unspecified
        };
        let sector_size = convert_fdc_sector_size_to_real_sector_size(self.sector_size);
        let gap3 = self.gap3_length;

        let sector_id = self
            .sector_information_list
            .sectors
            .iter()
            .map(|s| s.sector_information_bloc.sector_id)
            .collect::<Vec<_>>();
        let sector_id_head = self
            .sector_information_list
            .sectors
            .iter()
            .map(|s| s.sector_information_bloc.side)
            .collect::<Vec<_>>();

        // XXX ensure the size of each sector corresponds to the given size
        // XXX if test fails, maube it is necessary to make another test
        self.sector_information_list
            .sectors
            .iter()
            .for_each(|s| assert_eq!(s.sector_information_bloc.sector_size, self.sector_size));

        TrackGroup {
            tracks,
            side,
            sector_size,
            gap3,
            sector_id,
            sector_id_head,
        }
    }
}

impl ExtendedDsk {
    /// Generate a configuration from the dsk
    pub fn to_cfg(&self) -> DiscConfig {
        DiscConfig {
            nb_tracks: self.nb_tracks_per_side(),
            nb_sides: self.nb_sides(),
            track_groups: self.track_list.to_cfg(2 == self.nb_sides()),
        }
    }
}

impl From<&ExtendedDsk> for DiscConfig {
    fn from(dsk: &ExtendedDsk) -> DiscConfig {
        dsk.to_cfg()
    }
}

named!(value<CompleteStr<'_>, u16>, alt!(hex | dec));

named!(
    list_of_values<CompleteStr<'_>, Vec<u16>>,
    separated_list!(tag!(","), value)
);

fn from_hex(input: CompleteStr<'_>) -> Result<u16, std::num::ParseIntError> {
    u16::from_str_radix(&input, 16)
}

fn from_dec(input: CompleteStr<'_>) -> Result<u16, std::num::ParseIntError> {
    u16::from_str_radix(&input, 10)
}

fn is_hex_digit(c: char) -> bool {
    c.is_digit(16)
}

fn is_dec_digit(c: char) -> bool {
    c.is_digit(10)
}

named!(
    hex<CompleteStr<'_>, u16>,
    do_parse!(
        tag!("0x") >> value: map_res!(take_while_m_n!(1, 2, is_hex_digit), from_hex) >> (value)
    )
);

named!(
    dec<CompleteStr<'_>, u16>,
    map_res!(take_while!(is_dec_digit), from_dec)
);

named_args!(value_of_key<'a>(key: &str)<CompleteStr<'a>, u16>,
	do_parse!(
		space0 >>
		tag_no_case!(key) >>
		space0 >>
		tag!("=") >>
		space0 >>
		val: value >>
		space0 >>
		opt!(eol) >>
		(
			val
		)
	)
);

named_args!(list_of_key<'a>(key: &str)<CompleteStr<'a>, Vec<u16>>,
	do_parse!(
		space0 >>
		tag_no_case!(key) >>
		space0 >>
		tag!("=") >>
		space0 >>
		vals: list_of_values >>
		space0 >>
		opt!(eol) >>
		(
			vals
		)
	)
);

named!(
    empty_line<CompleteStr<'_>, ()>,
    do_parse!(space0 >> eol >> (()))
);

named!(
    track_group_sided<CompleteStr<'_>, TrackGroup>,
    do_parse!(
        side: alt! (
	
	delimited!(
		tag_no_case!("[Track-"), 
		alt!(
			tag_no_case!("A") => {|_|{Side::SideA}} |
			tag_no_case!("B") => {|_|{Side::SideB}}
		),
	 tag_no_case!(":")
 ) |

 tag_no_case!("[Track:")  => {|_| {Side::Unspecified}})
            >> tracks: list_of_values
            >> tag_no_case!("]")
            >> many0!(empty_line)
            >> sector_size: call!(value_of_key, "SectorSize")
            >> many0!(empty_line)
            >> gap3: call!(value_of_key, "Gap3")
            >> many0!(empty_line)
            >> sector_id: call!(list_of_key, "SectorId")
            >> sector_id_head: call!(list_of_key, "SectorIdHead")
            >> (TrackGroup {
                tracks: tracks.iter().map(|v| *v as u8).collect::<Vec<u8>>(),
                side: side,
                sector_size,
                gap3: gap3 as u8,
                sector_id: sector_id.iter().map(|&v| v as u8).collect::<Vec<_>>(),
                sector_id_head: sector_id_head.iter().map(|&v| v as u8).collect::<Vec<_>>(),
            })
    )
);

named!(pub parse_config<CompleteStr<'_>, DiscConfig>,
  do_parse!(
		many0!(empty_line) >>
	  nb_tracks: call!(value_of_key, "NbTrack") >>
		many0!(empty_line) >>
	  nb_sides: call!(value_of_key, "NbSide") >>
		track_groups: fold_many1!(
			 preceded!(
			  many0!(empty_line),
			  track_group_sided
		   ),
			 Vec::new(),
			 |mut acc: Vec<_>, item|{
				 acc.push(item);
				 acc
			 }
		 ) >>
	(
		DiscConfig {
			nb_tracks: nb_tracks as _,
			nb_sides: nb_sides as _,
			track_groups
		}
	)

  )
);

#[cfg(test)]
mod tests {

    use crate::disc::cfg::*;

    #[test]
    fn parse_decimal() {
        let res = dec("10 ".into());
        assert!(res.is_ok());

        let res = dec("10".into());
        assert!(res.is_ok());
    }

    #[test]
    fn parse_hexadecimal() {
        let res = hex("10 ".into());
        assert!(res.is_err());

        let res = hex("0x10 ".into());
        assert!(res.is_ok());
    }

    #[test]
    fn parse_value() {
        let res = value("0x10 ".into());
        assert!(res.is_ok());

        let res = value("10 ".into());
        assert!(res.is_ok());
    }

    #[test]
    fn parse_list_value() {
        let res = list_of_values("0x10 ".into());
        assert!(res.is_ok());
        let (_next, res) = res.unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], 0x10);

        let res = list_of_values("10,11 ".into());
        assert!(res.is_ok());
        let (_next, res) = res.unwrap();
        assert_eq!(res.len(), 2);
        assert_eq!(res[0], 10);
        assert_eq!(res[1], 11);
    }

    #[test]
    fn test_value_of_key() {
        let res = value_of_key("NbTrack = 80".into(), "NbTrack");
        println!("{:?}", &res);
        assert!(res.is_ok());
    }

}
