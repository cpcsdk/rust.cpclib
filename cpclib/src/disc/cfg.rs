/// Parser of the disc configuraiton used by the Arkos Loader


use nom;
use nom::{hex_u32, space0, space1, eol };
use nom::types::CompleteStr;
use std::str::FromStr;

#[derive(Debug)]
pub struct DiscConfig {
	nb_track: u16,
	nb_side: u16,
	track_groups: Vec<TrackGroup>
}


#[derive(Debug)]
pub enum Side {
	SideA,
	SideB
}


#[derive(Debug)]
pub struct TrackGroup {
	tracks: Vec<u16>,
	side: Side,
	sector_size: u16,
	gap3: u16,
	sector_id: Vec<u16>,
	sector_id_head: Vec<u16>,
}



named!(value<CompleteStr, u16>,
alt!(hex|dec)
);


named!(list_of_values<CompleteStr, Vec<u16>>,
	separated_list!(
		tag!(","),
		value
	)
);



fn from_hex(input: CompleteStr) -> Result<u16, std::num::ParseIntError> {
  u16::from_str_radix(&input, 16)
}

fn is_hex_digit(c: char) -> bool {
  c.is_digit(16)
}


fn is_dec_digit(c: char) -> bool {
  c.is_digit(10)
}


named!(hex<CompleteStr, u16>,
 do_parse!(
	 tag!("0x") >>
  value: map_res!(
	  take_while_m_n!(1, 2, is_hex_digit), 
	  from_hex
  ) >>
  (
	  value
  )
 )
);

named!(dec<CompleteStr, u16>,
  map_res!(
	  take_while!(is_dec_digit), 
	  from_hex
  )
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





named!(empty_line<CompleteStr, ()>,
 do_parse!(
	 space0 >>
	 eol >>
	 (
		 ()
	 )
 )
);



named!(track_group_sided<CompleteStr, TrackGroup>,
do_parse!(
	side: delimited!(
		tag_no_case!("[Track-"), 
		alt!(
			tag_no_case!("A") => {|_|{Side::SideA}} |
			tag_no_case!("B") => {|_|{Side::SideB}}
		),
	 tag_no_case!(":")
 ) >>
 tracks: list_of_values >>
 tag_no_case!("]") >>
	many0!(empty_line) >>
	sector_size: call!(value_of_key, "SectorSize") >>
		many0!(empty_line) >>
		gap3: call!(value_of_key, "Gap3") >>
		many0!(empty_line) >>
		sector_id: call!(list_of_key, "SectorId") >>
		sector_id_head: call!(list_of_key, "SectorIdHead") >>
	(
		TrackGroup{
			tracks: tracks,
			side: side,
			sector_size,
			gap3,
			sector_id,
			sector_id_head
		}
	)
)
);


named!(pub parse_config<CompleteStr, DiscConfig>,
  do_parse!(
		many0!(empty_line) >>
	  nb_track: call!(value_of_key, "NbTrack") >>
		many0!(empty_line) >>
	  nb_side: call!(value_of_key, "NbSide") >>
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
			nb_track,
			nb_side,
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
 fn test_value_of_key() {
	 let res = value_of_key("NbTrack = 80".into(), "NbTrack");
	 println!("{:?}", &res);
	 assert!(res.is_ok());
 }

}
