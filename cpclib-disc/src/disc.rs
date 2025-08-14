use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use cpclib_common::camino::Utf8Path;

use crate::amsdos::{
    AmsdosAddBehavior, AmsdosError, AmsdosFile, AmsdosFileName, AmsdosManagerMut,
    AmsdosManagerNonMut
};
use crate::edsk::Head;

pub struct Sector<'d, D: Disc + Sized> {
    data: Box<[u8]>,
    head: Head,
    track: u8,
    sector_id: u8,
    disc: &'d mut D
}


impl<'d, D:Disc> Deref for Sector<'d, D> {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'d, D:Disc> Drop for Sector<'d, D> {
    fn drop(&mut self) {
        self.disc.sector_write_bytes(
            self.head, 
            self.track, 
            self.sector_id, 
            &self.data
        ).expect("There is a bug somwhere");
    }
}

impl<'d, D:Disc> Sector<'d, D> {
    pub fn set_values(&mut self, data: &[u8]) -> Result<(), String> {
        if data.len() < self.len() as usize {
            return Err(format!(
                "You cannot insert {} bytes in a sector of size {}.",
                data.len(),
                self.len()
            ));
        }

        if data.len() > self.len() as usize {
            return Err(format!(
                "Smaller data of {} bytes to put in a sector of size {}.",
                data.len(),
                self.len()
            ));
        }

        self.data[..].clone_from_slice(data);
        Ok(())
    }

    pub fn set_value(&mut self, idx: usize, value: u8) {
        self.data[idx] = value;
    }
}

pub trait Disc {
    fn open<P>(path: P) -> Result<Self, String>
    where
        Self: Sized,
        P: AsRef<Utf8Path>;
    fn save<P>(&self, path: P) -> Result<(), String>
    where P: AsRef<Utf8Path>;

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


    /// The sector is modified right back in the dsk once it is dropped
    fn sector_mut<S: Into<Head>>(
        &mut self,
        head: S,
        track: u8,
        sector_id: u8
    ) -> Option<Sector<'_, Self>> where Self: Sized{
        let head: Head = head.into();
        let data = self.sector_read_bytes(head, track, sector_id)?;
        Some(Sector {
            data: data.into(),
            head,
            track,
            sector_id,
            disc: self
        })
    }

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
        read_only: bool,
        behavior: AmsdosAddBehavior
    ) -> Result<(), AmsdosError>
    where
        Self: Sized
    {
        let fname = file.amsdos_filename().unwrap()?;
        if !fname.is_valid() {
            return Err(AmsdosError::WrongFileName {
                msg: fname.filename()
            });
        }

        let mut manager = AmsdosManagerMut::new_from_disc(self, head);

        manager.add_file(file, None, system, read_only, behavior)?;

        Ok(())
    }

    fn add_ascii_file<H: Into<Head>>(
        &mut self,
        file: &AmsdosFile,
        fname: &AmsdosFileName,
        head: H,
        system: bool,
        read_only: bool,
        behavior: AmsdosAddBehavior
    ) -> Result<(), AmsdosError>
    where
        Self: Sized
    {
        if !fname.is_valid() {
            return Err(AmsdosError::WrongFileName {
                msg: fname.filename()
            });
        }

        let mut manager = AmsdosManagerMut::new_from_disc(self, head);

        manager.add_file(file, Some(fname), system, read_only, behavior)?;

        Ok(())
    }

    fn get_amsdos_file<H: Into<Head>, F: Into<AmsdosFileName>>(
        &self,
        head: H,
        filename: F
    ) -> Result<Option<AmsdosFile>, AmsdosError>
    where
        Self: Sized
    {
        let filename: AmsdosFileName = filename.into();

        if !filename.is_valid() {
            Err(AmsdosError::WrongFileName {
                msg: format!("{filename:?}")
            })
        }
        else {
            let manager = AmsdosManagerNonMut::new_from_disc(self, head);

            Ok(manager.get_file(filename))
        }
    }
}

impl<T: Disc> Disc for Box<T> {
    fn open<P>(path: P) -> Result<Self, String>
    where
        Self: Sized,
        P: AsRef<Utf8Path>
    {
        Ok(Box::new(T::open(path)?))
    }

    fn save<P>(&self, path: P) -> Result<(), String>
    where P: AsRef<Utf8Path> {
        self.deref().save(path)
    }

    fn global_min_sector<S: Into<Head>>(&self, side: S) -> u8 {
        self.deref().global_min_sector(side)
    }

    fn track_min_sector<S: Into<Head>>(&self, side: S, track: u8) -> u8 {
        self.deref().track_min_sector(side, track)
    }

    fn nb_tracks_per_head(&self) -> u8 {
        self.deref().nb_tracks_per_head()
    }

    fn sector_read_bytes<S: Into<Head>>(
        &self,
        head: S,
        track: u8,
        sector_id: u8
    ) -> Option<Vec<u8>> {
        self.deref().sector_read_bytes(head, track, sector_id)
    }

    fn sector_write_bytes<S: Into<Head>>(
        &mut self,
        head: S,
        track: u8,
        sector_id: u8,
        bytes: &[u8]
    ) -> Result<(), String> {
        self.deref_mut()
            .sector_write_bytes(head, track, sector_id, bytes)
    }
}
