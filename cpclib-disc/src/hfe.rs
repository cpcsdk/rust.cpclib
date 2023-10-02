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

use camino::Utf8Path;
use cpclib_common::itertools::Itertools;
use enumn::N;
use hxcfe::{Hxcfe, Img};
use hxcfe::TrackEncoding;
use crate::{edsk::{ExtendedDsk, Head}, disc::Disc};

#[derive(Debug)]
pub struct Hfe {
    img: Img
}

impl Hfe {
    pub fn open<P: AsRef<Utf8Path>>(fname: P) -> Result<Self, String> {
        let hxcfe = Hxcfe::get();
        hxcfe.load(fname.as_ref())
            .map(|img| Hfe{img})
    }
}


impl Disc for Hfe {
    fn sector_read_bytes<S: Into<Head>>(
		&self,
		head: S,
		track: u8,
		sector_id: u8,
	) -> Option<Vec<u8>> {
        let hxcfe = Hxcfe::get();

        let head: i32 = head.into().into();
        assert!(head==0 || head==1);


        let sector_access = self.img.sector_access().unwrap();
        let cfg = sector_access.search_sector(head as _, track as _, sector_id as _, TrackEncoding::IsoIbmMfm)?;
        let data = cfg.read().to_vec();

        Some(data)
    }

    fn sector_write_bytes<S: Into<Head>>(
		    &mut self,
		    head: S,
		    track: u8,
		    sector_id: u8,
		    bytes: &[u8]
	    )  -> Result<(), String>  {
            let head: i32 = head.into().into();
            let encoding = TrackEncoding::IsoIbmMfm;
            let sector_access = self.img.sector_access().unwrap();
            let mut cfg = sector_access.search_sector(head as _, track as _, sector_id as _, encoding).ok_or_else(|| "sector not found".to_owned())?;
            
            cfg.write(encoding, bytes); // TODO handle error
            Ok(())
          
    }

    fn min_sector<S: Into<Head>>(&self, side: S)-> u8 {
        let s = side.into();
        let access = self.img.sector_access().unwrap();
        let mut min_sector = std::i32::MAX;
        for t in 0..(*self.img.floppydisk).floppyNumberOfTrack {
            for s in 0..(*self.img.floppydisk).floppyNumberOfSide {
                let mut rec_mode = 2;  // MFM
                let sca = access.all_track_sectors(t,s,TrackEncoding::IsoIbmMfm);
                let sca = match sca {
                    Some(sca) => sca,
                    None => {
                        rec_mode = 1; // FM
                        access.all_track_sectors(t,s,TrackEncoding::IsoIbmFm).unwrap()
                    }
                };

                for k in 0..sca.nb_sectors() {
                    
                }

            }

       }

       todo!()
    }
}


impl From<ExtendedDsk> for Hfe {
    // huge inspiration from https://sourceforge.net/p/hxcfloppyemu/code/HEAD/tree/HxCFloppyEmulator/libhxcfe/trunk/sources/loaders/cpcdsk_loader/cpcdsk_loader.c#l129
    fn from(dsk: ExtendedDsk) -> Self {
       todo!()
    }
}

#[cfg(test)]
mod test {
    use super::Hfe;

    #[test]
    fn load_hfe() {
        let _hfe = Hfe::open("tests/MOODY.HFE");
    }
}
