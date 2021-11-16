use std::fmt;
use std::fs::File;
use std::io::Read;
use std::iter::Iterator;
use std::path::Path;
use std::str::FromStr;

use cpclib_common::itertools;
use cpclib_common::nom::branch::*;
use cpclib_common::nom::bytes::complete::*;
use cpclib_common::nom::character::complete::*;
use cpclib_common::nom::combinator::*;
use cpclib_common::nom::lib::std::convert::Into;
use cpclib_common::nom::multi::*;
use cpclib_common::nom::sequence::*;
/// Parser of the disc configuraiton used by the Arkos Loader
use cpclib_common::nom::*;
use custom_error::custom_error;
use itertools::Itertools;

use crate::edsk::*;

const DATA_FORMAT_CFG: &str = "
NbTrack = 40
NbHead = 1

[Track:0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39]
SectorSize = 512
Gap3 = 82
SectorID = 0xc1,0xc6,0xc2,0xc7,0xc3,0xc8,0xc4,0xc9,0xc5
sectorIDHead = 0,0,0,0,0,0,0,0,0
";

const DATA_FORMAT42_CFG: &str = "
NbTrack = 42
NbHead = 1

[Track:0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,41]
SectorSize = 512
Gap3 = 0x4e
SectorID = 0xc1,0xc6,0xc2,0xc7,0xc3,0xc8,0xc4,0xc9,0xc5
sectorIDHead = 0,0,0,0,0,0,0,0,0
";

custom_error! {
#[allow(missing_docs)]
/// Errors specifics to the configuration manipulation
pub DiscConfigError
    IOError{source: std::io::Error} = "IO error: {source}.",
    ParseError{msg: String}            = "Parse error: {msg}"
}

/// Disk format configuration.
#[derive(Debug, PartialEq)]
pub struct DiscConfig {
    /// Number of tracks in the disc
    pub(crate) nb_tracks: u8,
    /// Number of heads in the disc (1 or 2)
    pub(crate) nb_heads: u8,
    /// List of tracks description
    pub(crate) track_groups: Vec<TrackGroup>
}

impl FromStr for DiscConfig {
    type Err = DiscConfigError;

    /// Generates the configuration from a &str. Panic in case of failure.
    /// The format corresponds to cpctools format from Ramlaid/Mortel.
    fn from_str(config: &str) -> Result<Self, Self::Err> {
        match parse_config(config.into()) {
            Ok((next, res)) => {
                if next.trim().is_empty() {
                    Ok(res)
                }
                else {
                    Err(DiscConfigError::ParseError {
                        msg: format!(
                            "Bug in the parser, there is still content to parse: {}",
                            next
                        )
                    })
                }
            }
            Err(error) => {
                Err(DiscConfigError::ParseError {
                    msg: format!("{:?}", error)
                })
            }
        }
    }
}

#[allow(missing_docs)]
impl DiscConfig {
    pub fn single_head_data_format() -> Self {
        Self::from_str(DATA_FORMAT_CFG).unwrap()
    }

    pub fn single_head_data42_format() -> Self {
        Self::from_str(DATA_FORMAT42_CFG).unwrap()
    }

    /// Create a configuration from the provided file
    pub fn new<P: AsRef<Path>>(p: P) -> Result<Self, DiscConfigError> {
        let mut content = String::new();
        let mut f = File::open(p.as_ref())?;
        f.read_to_string(&mut content)?;

        Self::from_str(content.as_str())
    }
}

#[allow(missing_docs)]
impl fmt::Display for DiscConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "NbTrack = {}", self.nb_tracks)?;
        writeln!(f, "NbHead = {}", self.nb_heads)?;

        for track_group in &self.track_groups {
            write!(f, "\n{}", track_group)?;
        }

        Ok(())
    }
}

#[allow(missing_docs)]
impl DiscConfig {
    /// A or B for a two head dsk. Unspecified for a single head disc
    pub fn track_information_for_track<S: Into<Head>>(
        &self,
        head: S,
        track: u8
    ) -> Option<&TrackGroup> {
        let head = head.into();
        self.track_groups
            .iter()
            .find(move |info| info.head == head && info.tracks.iter().any(|&val| val == track))
    }

    pub fn track_idx_iterator(&self) -> impl Iterator<Item = (&Head, u8)> {
        let head_iterator = match self.nb_heads {
            2 => [Head::A, Head::B].iter(),
            1 => [Head::Unspecified].iter(),
            _ => unreachable!()
        };
        let track_iterator = 0..self.nb_tracks;

        head_iterator.cartesian_product(track_iterator)
    }
}

#[allow(missing_docs)]
impl DiscConfig {
    /// return a disc configuration where each groups contains only one track
    /// TODO find a better name
    pub fn explode(&self) -> Self {
        let mut groups = Vec::new();
        for track_group in &self.track_groups {
            for track in &track_group.tracks {
                groups.push(TrackGroup {
                    tracks: vec![*track],
                    head: track_group.head,
                    sector_size: track_group.sector_size,
                    gap3: track_group.gap3,
                    sector_id: track_group.sector_id.clone(),
                    sector_id_head: track_group.sector_id_head.clone()
                });
            }
        }

        groups.sort_by_key(|group| group.tracks[0]);

        Self {
            nb_tracks: self.nb_tracks,
            nb_heads: self.nb_heads,
            track_groups: groups
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
/// Desribes tracks for a given group of tracks
pub struct TrackGroup {
    /// Identifier of the tracks molded from this configuration
    pub(crate) tracks: Vec<u8>,
    /// Physical ide
    pub(crate) head: Head,
    /// Size of a sector
    pub(crate) sector_size: u16,
    pub(crate) gap3: u8,
    /// List of id of the sectors
    pub(crate) sector_id: Vec<u8>,
    /// List of logical head of the sectors
    pub(crate) sector_id_head: Vec<u8>
}

#[allow(missing_docs)]
impl fmt::Display for TrackGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let head_info = match self.head {
            Head::A => "-A",
            Head::B => "-B",
            Head::Unspecified => ""
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

        writeln!(f, "[Track{}:{}]", head_info, tracks_info)?;
        writeln!(f, "SectorSize = {}", self.sector_size)?;
        writeln!(f, "Gap3 = 0x{:x}", self.gap3)?;
        writeln!(f, "SectorID = {}", sector_id)?;
        writeln!(f, "SectorIDHead = {}", sector_id_head)?;

        Ok(())
    }
}

#[allow(missing_docs)]
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

    pub fn sector_id_at(&self, idx: usize) -> u8 {
        self.sector_id[idx]
    }
}

#[allow(missing_docs)]
impl TrackInformationList {
    pub fn to_cfg(&self, double_head: bool) -> Vec<TrackGroup> {
        let mut single = self
            .list
            .iter()
            .map(|t| t.to_cfg(double_head))
            .collect::<Vec<_>>();

        // elements need to be sorted before using group_by
        single.sort_by_key(|item| {
            (
                item.head,
                item.sector_size,
                item.gap3,
                item.sector_id.clone(),
                item.sector_id_head.clone()
            )
        });
        // group_by
        let mut grouped = single
            .iter()
            .group_by(|item| {
                (
                    item.head,
                    item.sector_size,
                    item.gap3,
                    item.sector_id.clone(),
                    item.sector_id_head.clone()
                )
            })
            .into_iter()
            .map(|(k, group)| {
                let mut tracks = group.map(|item| item.tracks[0]).collect::<Vec<u8>>();
                tracks.sort();
                TrackGroup {
                    tracks,
                    head: k.0,
                    sector_size: k.1,
                    gap3: k.2,
                    sector_id: k.3,
                    sector_id_head: k.4
                }
            })
            .collect::<Vec<TrackGroup>>();

        // Sorted the result
        grouped.sort_by_key(|item| (item.head, item.tracks[0]));

        grouped
    }
}

/// Extend TrackInformation with the ability to extract its configuration
#[allow(missing_docs)]
impl TrackInformation {
    pub fn to_cfg(&self, double_head: bool) -> TrackGroup {
        let tracks = vec![self.track_number];
        let head: Head = if double_head {
            self.head_number.into()
        }
        else {
            Head::Unspecified
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
            .map(|s| s.sector_information_bloc.head)
            .collect::<Vec<_>>();

        // XXX ensure the size of each sector corresponds to the given size
        // XXX if test fails, maube it is necessary to make another test
        self.sector_information_list
            .sectors
            .iter()
            .for_each(|s| assert_eq!(s.sector_information_bloc.sector_size, self.sector_size));

        TrackGroup {
            tracks,
            head,
            sector_size,
            gap3,
            sector_id,
            sector_id_head
        }
    }
}

#[allow(missing_docs)]
impl ExtendedDsk {
    /// Generate a configuration from the dsk
    pub fn to_cfg(&self) -> DiscConfig {
        DiscConfig {
            nb_tracks: self.nb_tracks_per_head(),
            nb_heads: self.nb_heads(),
            track_groups: self.track_list.to_cfg(2 == self.nb_heads())
        }
    }
}

#[allow(missing_docs)]
impl From<&ExtendedDsk> for DiscConfig {
    fn from(dsk: &ExtendedDsk) -> Self {
        dsk.to_cfg()
    }
}

fn number(input: &str) -> IResult<&str, u16> {
    alt((hex, dec))(input)
}

fn list_of_values(input: &str) -> IResult<&str, Vec<u16>> {
    separated_list0(tag(","), number)(input)
}

fn from_hex(input: &str) -> Result<u16, std::num::ParseIntError> {
    u16::from_str_radix(&input, 16)
}

fn from_dec(input: &str) -> Result<u16, std::num::ParseIntError> {
    u16::from_str_radix(&input, 10)
}

fn is_hex_digit(c: char) -> bool {
    c.is_digit(16)
}

fn is_dec_digit(c: char) -> bool {
    c.is_digit(10)
}

fn hex(input: &str) -> IResult<&str, u16> {
    preceded(
        tag("0x"),
        map_res(take_while_m_n(1, 2, is_hex_digit), from_hex)
    )(input)
}

fn dec(input: &str) -> IResult<&str, u16> {
    map_res(take_while(is_dec_digit), from_dec)(input)
}

fn value_of_key<'a>(key: &'static str) -> impl Fn(&'a str) -> IResult<&'a str, u16> {
    move |input: &'a str| {
        delimited(
            tuple((space0, tag_no_case(key), space0, tag("="), space0)),
            number,
            tuple((space0, opt(line_ending)))
        )(input)
    }
}

fn list_of_key<'a>(key: &'static str) -> impl Fn(&'a str) -> IResult<&'a str, Vec<u16>> {
    move |input: &'a str| {
        delimited(
            tuple((space0, tag_no_case(key), space0, tag("="), space0)),
            list_of_values,
            tuple((space0, opt(line_ending)))
        )(input)
    }
}

fn empty_line(input: &str) -> IResult<&str, ()> {
    value((), tuple((space0, line_ending)))(input)
}

fn track_group_head(input: &str) -> IResult<&str, TrackGroup> {
    let (input, head) = alt((
        delimited(
            tag_no_case("[Track-"),
            alt((
                value(Head::A, tag_no_case("A")),
                value(Head::B, tag_no_case("B"))
            )),
            tag_no_case(":")
        ),
        value(Head::Unspecified, tag_no_case("[Track:"))
    ))(input)?;

    let (input, tracks) =
        terminated(list_of_values, tuple((tag_no_case("]"), many0(empty_line))))(input)?;

    // TODO modify the remaining part in order to allow any order

    let (input, sector_size) = terminated(value_of_key("SectorSize"), many0(empty_line))(input)?;

    let (input, gap3) = terminated(value_of_key("Gap3"), many0(empty_line))(input)?;

    let (input, sector_id) = list_of_key("SectorId")(input)?;

    let (input, sector_id_head) = list_of_key("SectorIdHead")(input)?;

    Ok((
        input,
        TrackGroup {
            tracks: tracks.iter().map(|v| *v as u8).collect::<Vec<u8>>(),
            head: head,
            sector_size,
            gap3: gap3 as u8,
            sector_id: sector_id.iter().map(|&v| v as u8).collect::<Vec<_>>(),
            sector_id_head: sector_id_head.iter().map(|&v| v as u8).collect::<Vec<_>>()
        }
    ))
}

/// TODO allow to write the information in a different order
pub fn parse_config(input: &str) -> IResult<&str, DiscConfig> {
    let (input, nb_tracks) = preceded(many0(empty_line), value_of_key("NbTrack"))(input)?;

    let (input, nb_heads) = preceded(
        many0(empty_line),
        alt((value_of_key("NbHead"), value_of_key("NbSide")))
    )(input)?;

    let (input, track_groups) = fold_many1(
        preceded(many0(empty_line), track_group_head),
        || Vec::new(),
        |mut acc: Vec<_>, item| {
            acc.push(item);
            acc
        }
    )(input)?;

    Ok((
        input,
        DiscConfig {
            nb_tracks: nb_tracks as _,
            nb_heads: nb_heads as _,
            track_groups
        }
    ))
}

#[cfg(test)]
mod tests {
    use crate::cfg::*;

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
        let res = number("0x10 ".into());
        assert!(res.is_ok());

        let res = number("10 ".into());
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
        let res = value_of_key("NbTrack")("NbTrack = 80".into());
        println!("{:?}", &res);
        assert!(res.is_ok());
    }
}
