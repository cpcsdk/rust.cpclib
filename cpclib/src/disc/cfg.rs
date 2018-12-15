/// Parser of the disc configuraiton used by the Arkos Loader


use nom;
use nom::{hex_u32, space0, space1, eol};
use nom::types::CompleteStr;
use std::str::FromStr;

#[derive(Debug)]
pub struct DiscConfig {
	nb_track: u16,
	nb_side: u16
}


named!(value<CompleteStr, u16>,
alt!(hex|dec)
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


named!(empty_line<CompleteStr, ()>,
 do_parse!(
	 space0 >>
	 eol >>
	 (
		 ()
	 )
 )
);


named!(pub parse_config<CompleteStr, DiscConfig>,
  do_parse!(
		opt!(empty_line) >>
	  nb_track: call!(value_of_key, "NbTrack") >>
	  nb_side: call!(value_of_key, "NbSide") >>
	(
		DiscConfig {
			nb_track,
			nb_side
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
