use std::path::Path;

use cpclib_common::itertools::Itertools;

use crate::amsdos::{AmsdosError, AmsdosFile, AmsdosManagerMut};
use crate::edsk::Head;

pub trait Disc {
    fn open<P>(path: P) -> Result<Self, String>
    where
        Self: Sized,
        P: AsRef<Path>;
    fn save<P>(&self, path: P) -> Result<(), String>
    where P: AsRef<Path>;

    fn global_min_sector<S: Into<Head>>(&self, side: S) -> u8;
    fn track_min_sector<S: Into<Head>>(&self, side: S, track: u8) -> u8;
    fn nb_tracks_per_head(&self) -> u8;

    fn sector_read_bytes<S: Into<Head>>(
        &self,
        head: S,
        track: u8,
        sector_id: u8
    ) -> Option<Vec<u8>>;

    fn sector_write_bytes<S: Into<Head>>(
        &mut self,
        head: S,
        track: u8,
        sector_id: u8,
        bytes: &[u8]
    ) -> Result<(), String>;

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

    /// Add the file where it is possible with respect to amsdos format
    fn add_amsdos_file<H: Into<Head>>(
        &mut self,
        file: &AmsdosFile,
        head: H,
        system: bool,
        read_only: bool
    ) -> Result<(), AmsdosError>
    where
        Self: Sized
    {
        if !file.amsdos_filename().unwrap().is_valid() {
            return Err(AmsdosError::WrongFileName {
                msg: file.amsdos_filename().unwrap().filename()
            });
        }

        let mut manager = AmsdosManagerMut::new_from_disc(self, head);

        eprint!("{:?}", manager.catalog().all_entries().collect_vec());
        manager.add_file(&file, system, read_only)?;
        eprint!("{:?}", manager.catalog().all_entries().collect_vec());

        Ok(())
    }
}
