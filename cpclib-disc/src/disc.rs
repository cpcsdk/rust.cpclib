use std::{io, path::Path};

use camino::Utf8Path;

use crate::edsk::Head;

pub trait Disc {
    fn open<P>(path: P) -> Result<Self, String> 
	where Self: Sized,
	P: AsRef<Path>;

	fn save<P>(&self, path: P) ->  Result<(), String> 
        where P: AsRef<Path>;

	fn global_min_sector<S: Into<Head>>(&self, side: S)-> u8;
	fn track_min_sector<S: Into<Head>>(&self, side: S, track: u8)->u8;
	fn nb_tracks_per_head(&self) -> u8;

	fn sector_read_bytes<S: Into<Head>>(
		&self,
		head: S,
		track: u8,
		sector_id: u8,
	) -> Option<Vec<u8>>; 

	fn sector_write_bytes<S: Into<Head>>(
		&mut self,
		head: S,
		track: u8,
		sector_id: u8,
		bytes: &[u8]
	)  -> Result<(), String> ;

	/// Return the concatenated values of several consecutive sectors
	/// None if it tries to read an inexistant sector
	fn consecutive_sectors_read_bytes<S: Into<Head> + Clone>(
		&self,
		head: S,
		track: u8,
		sector_id: u8,
		nb_sectors: u8
	) -> Option<Vec<u8>> {
		

		let mut res = Vec::new();

		for count in 0..nb_sectors {
			match self.sector_read_bytes(head.clone(), track, sector_id + count) {
				None => return None,
				Some(s) => res.extend(s)
			}
		}

		Some(res)
	}
}