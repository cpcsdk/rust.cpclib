// typedef struct picfileformatheader_
// {
// unsigned char HEADERSIGNATURE[8]; // “HXCPICFE”
// unsigned char formatrevision; // Revision 0
// unsigned char number_of_track; // Number of track in the file
// unsigned char number_of_side; // Number of valid side (Not used by the emulator)
// unsigned char track_encoding; // Track Encoding mode
// (Used for the write support - Please see the list above)
// unsigned short bitRate; // Bitrate in Kbit/s. Ex : 250=250000bits/s
// Max value : 500
// unsigned short floppyRPM; // Rotation per minute (Not used by the emulator)
// unsigned char floppyinterfacemode; // Floppy interface mode. (Please see the list above.)
// unsigned char dnu; // Free
// unsigned short track_list_offset; // Offset of the track list LUT in block of 512 bytes
// (Ex: 1=0x200)
// unsigned char write_allowed; // The Floppy image is write protected ?
// unsigned char single_step; // 0xFF : Single Step – 0x00 Double Step mode
// unsigned char track0s0_altencoding; // 0x00 : Use an alternate track_encoding for track 0 Side 0
// unsigned char track0s0_encoding; // alternate track_encoding for track 0 Side 0
// unsigned char track0s1_altencoding; // 0x00 : Use an alternate track_encoding for track 0 Side 1
// unsigned char track0s1_encoding; // alternate track_encoding for track 0 Side 1
// }picfileformatheader;

use camino_tempfile::Builder;
use cpclib_common::camino::Utf8Path;
use hxcfe::{Hxcfe, Img, TrackEncoding};

use crate::builder::build_edsk_from_cfg;
use crate::cfg::DiscConfig;
use crate::disc::Disc;
use crate::edsk::{ExtendedDsk, Head};

#[derive(Debug)]
pub struct Hfe {
    img: Img
}

impl Hfe {}

impl Default for Hfe {
    fn default() -> Self {
        let cfg = crate::cfg::DiscConfig::single_head_data42_format();
        cfg.into()
    }
}

impl Disc for Hfe {
    fn open<P: AsRef<Utf8Path>>(fname: P) -> Result<Self, String> {
        let hxcfe = Hxcfe::get();
        hxcfe.load(fname.as_ref()).map(|img| Hfe { img })
    }

    fn save<P>(&self, path: P) -> Result<(), String>
    where P: AsRef<Utf8Path> {
        let path = path.as_ref();
        let format = match path.extension().unwrap().to_lowercase().as_str() {
            "dsk" | "edsk" => "AMSTRADCPC_DSK",
            "hfe" => "HXC_HFE",
            _ => return Err(format!("i do not know how to save {}", path))
        };
        self.img.save(path, format)
    }

    fn sector_read_bytes<S: Into<Head>>(
        &self,
        head: S,
        track: u8,
        sector_id: u8
    ) -> Option<Vec<u8>> {
        let hxcfe = Hxcfe::get();

        let head: i32 = head.into().into();
        assert!(head == 0 || head == 1);

        let sector_access = self.img.sector_access().unwrap();
        let cfg = sector_access.search_sector(
            head as _,
            track as _,
            sector_id as _,
            TrackEncoding::IsoIbmMfm
        )?;
        let data = cfg.read().to_vec();

        Some(data)
    }

    fn sector_write_bytes<S: Into<Head>>(
        &mut self,
        head: S,
        track: u8,
        sector_id: u8,
        bytes: &[u8]
    ) -> Result<(), String> {
        let head: i32 = head.into().into();
        let encoding = TrackEncoding::IsoIbmMfm;
        let sector_access = self.img.sector_access().unwrap();
        let mut cfg = sector_access
            .search_sector(head as _, track as _, sector_id as _, encoding)
            .ok_or_else(|| "sector not found".to_owned())?;

        cfg.write(encoding, bytes); // TODO handle error
        Ok(())
    }

    fn global_min_sector<S: Into<Head>>(&self, side: S) -> u8 {
        let s = side.into();
        let access = self.img.sector_access().unwrap();
        let mut min_sector = std::u8::MAX;
        for t in 0..(self.img.nb_tracks()) {
            for s in 0..self.img.nb_sides() {
                min_sector = self.track_min_sector(s as u8, t as _);
            }
        }

        min_sector as _
    }

    fn track_min_sector<S: Into<Head>>(&self, side: S, track: u8) -> u8 {
        let s = side.into().into();
        let access = self.img.sector_access().unwrap();
        let sca = access.all_track_sectors(s, track as _, TrackEncoding::IsoIbmMfm);
        let sca = match sca {
            Some(sca) => sca,
            None => {
                access
                    .all_track_sectors(s, track as _, TrackEncoding::IsoIbmFm)
                    .unwrap()
            },
        };

        (0..sca.nb_sectors())
            .map(|k| sca.sector_config(k).sector_id())
            .min()
            .unwrap() as _
    }

    fn nb_tracks_per_head(&self) -> u8 {
        self.img.nb_tracks_per_head() as _
    }
}

impl From<ExtendedDsk> for Hfe {
    // TODO do it WITHOUT saving a disc
    fn from(dsk: ExtendedDsk) -> Self {
        // Save the DSK on disc
        let tmp = Builder::new()
            .suffix(".dsk")
            .rand_bytes(6)
            .tempfile()
            .unwrap();
        let fname = tmp.into_temp_path();
        let fname = fname.to_path_buf();
        dsk.save(&fname).unwrap();

        // Reload it as an hfe
        Hfe::open(fname).unwrap()
    }
}

#[allow(missing_docs)]
// TODO implement directly without conversion from dsk
impl From<DiscConfig> for Hfe {
    fn from(config: DiscConfig) -> Self {
        Hfe::from(build_edsk_from_cfg(&config))
    }
}

#[allow(missing_docs)]
// TODO implement directly without conversion from dsk
impl From<&DiscConfig> for Hfe {
    fn from(config: &DiscConfig) -> Self {
        build_edsk_from_cfg(config).into()
    }
}

#[cfg(test)]
mod test {
    use super::Hfe;
    use crate::disc::Disc;

    #[test]
    fn load_hfe() {
        let _hfe = Hfe::open("tests/MOODY.HFE");
    }
}
