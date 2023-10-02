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

use crate::edsk::ExtendedDsk;

#[derive(Debug)]
struct PicFileFormatHeader {
    format_revision: u8,
    number_of_track: u8,
    number_of_side: u8,
    track_encoding: TrackEncoding,
    bit_rate: u16,
    floppy_rpm: u16,
    floppy_interface: FloppyInterface,
    track_list_offset: u16,
    write_allowed: u8,
    single_step: Step,
    track0s0_altencoding: TrackAltEncoding,
    track0s0_encoding: TrackEncoding,
    track0s1_altencoding: TrackAltEncoding,
    track0s1_encoding: TrackEncoding
}

#[repr(u8)]
#[derive(PartialEq, Debug, N)]
enum FloppyInterface {
    IBMPC_DD_FLOPPYMODE = 0x00,
    IBMPC_HD_FLOPPYMODE = 0x01,
    ATARIST_DD_FLOPPYMODE = 0x02,
    ATARIST_HD_FLOPPYMODE = 0x03,
    AMIGA_DD_FLOPPYMODE = 0x04,
    AMIGA_HD_FLOPPYMODE = 0x05,
    CPC_DD_FLOPPYMODE = 0x06,
    GENERIC_SHUGGART_DD_FLOPPYMODE = 0x07,
    IBMPC_ED_FLOPPYMODE = 0x08,
    MSX2_DD_FLOPPYMODE = 0x09,
    C64_DD_FLOPPYMODE = 0x0A,
    EMU_SHUGART_FLOPPYMODE = 0x0B,
    S950_DD_FLOPPYMODE = 0x0C,
    S950_HD_FLOPPYMODE = 0x0D,
    DISABLE_FLOPPYMODE = 0xFE
}

#[repr(u8)]
#[derive(PartialEq, Debug, N)]
enum TrackEncoding {
    ISOIBM_MFM_ENCODING = 0x00,
    AMIGA_MFM_ENCODING = 0x01,
    ISOIBM_FM_ENCODING = 0x02,
    EMU_FM_ENCODING = 0x03,
    UNKNOWN_ENCODING = 0xFF
}

#[repr(u8)]
#[derive(PartialEq, Debug, N)]
enum Step {
    Single = 0x00,
    Double = 0xFF
}

#[repr(u8)]
#[derive(PartialEq, Debug, N)]
enum TrackAltEncoding {
    Yes = 0x00,
    No = 0xFF
}

impl PicFileFormatHeader {
    pub fn from_buffer(buffer: &[u8]) -> Self {
        assert_eq!(buffer.len(), 512);

        assert_eq!(
            String::from_utf8_lossy(&buffer[..8]).to_ascii_uppercase(),
            "HXCPICFE".to_ascii_uppercase()
        );

        let mut i = 8;

        let format_revision = buffer[i];
        i += 1;
        let number_of_track = buffer[i];
        i += 1;
        let number_of_side = buffer[i];
        i += 1;
        let track_encoding = TrackEncoding::n(buffer[i]).unwrap();
        i += 1;
        let bit_rate = buffer[i] as u16 + 256 * (buffer[i + 1] as u16);
        i += 2;
        let floppy_rpm = buffer[i] as u16 + 256 * (buffer[i + 1] as u16);
        i += 2;
        let floppy_interface = FloppyInterface::n(buffer[i]).unwrap();
        i += 1;
        i += 1; // dnu
        let track_list_offset = buffer[i] as u16 + 256 * (buffer[i + 1] as u16);
        i += 2;
        let write_allowed = buffer[i];
        i += 1;
        let single_step = Step::n(buffer[i]).unwrap();
        i += 1;
        let track0s0_altencoding = TrackAltEncoding::n(buffer[i]).unwrap();
        i += 1;
        let track0s0_encoding = TrackEncoding::n(buffer[i]).unwrap();
        i += 1;
        let track0s1_altencoding = TrackAltEncoding::n(buffer[i]).unwrap();
        i += 1;
        let track0s1_encoding = TrackEncoding::n(buffer[i]).unwrap();
        i += 1;

        PicFileFormatHeader {
            format_revision,
            number_of_track,
            number_of_side,
            track_encoding,
            bit_rate,
            floppy_rpm,
            floppy_interface,
            track_list_offset,
            write_allowed,
            single_step,
            track0s0_altencoding,
            track0s0_encoding,
            track0s1_altencoding,
            track0s1_encoding
        }
    }
}

#[derive(Debug)]
struct PicTrack {
    offset: u16,
    track_len: u16
}

impl PicTrack {
    pub fn from_buffer(buffer: &[u8]) -> Self {
        assert_eq!(buffer.len(), 4);

        let mut i = 0;
        let offset = buffer[i] as u16 + 256 * (buffer[i + 1] as u16);
        i += 2;
        let track_len = buffer[i] as u16 + 256 * (buffer[i + 1] as u16);
        i += 2;

        PicTrack { offset, track_len }
    }

    pub fn offset(&self) -> u16 {
        self.offset
    }

    pub fn track_len(&self) -> u16 {
        self.track_len
    }

    pub fn nb_blocs(&self) -> u16 {
        let mut nb_blocs = self.track_len() / 512;
        if self.track_len() % 512 != 0 {
            nb_blocs += 1;
        }
        nb_blocs
    }

    pub fn track_allocated_space(&self) -> u16 {
        self.nb_blocs() * 512
    }
}

#[derive(Debug)]
struct PicTrackList(Vec<PicTrack>);
impl PicTrackList {
    pub fn from_buffer(buffer: &[u8], nb_track: u8) -> Self {
        assert_eq!(buffer.len(), 2 * 2 * (nb_track as usize));

        let elems = (0..nb_track as usize)
            .map(|i| PicTrack::from_buffer(&buffer[(i * 4)..((i + 1) * 4)]))
            .collect_vec();
        Self(elems)
    }

    pub fn list(&self) -> &[PicTrack] {
        &self.0
    }
}

#[derive(Debug)]
struct TrackBlock {
    side0: [u8; 256],
    side1: [u8; 256]
}

impl TrackBlock {
    pub fn from_buffer(buffer: &[u8]) -> Self {
        assert_eq!(512, buffer.len());

        let mut bloc = TrackBlock {
            side0: [0; 256],
            side1: [0; 256]
        };

        bloc.side0.clone_from_slice(&buffer[0..256]);
        bloc.side1.clone_from_slice(&buffer[256..512]);

        bloc
    }
}

#[derive(Debug)]
struct TrackData(Vec<TrackBlock>);
impl TrackData {
    pub fn from_buffer(buffer: &[u8], pt: &PicTrack) -> Self {
        let elems = (0..pt.nb_blocs() as usize)
            .map(|i| TrackBlock::from_buffer(&buffer[(i * 512)..((i + 1) * 512)]))
            .collect_vec();
        TrackData(elems)
    }
}

#[derive(Debug)]
struct TrackDataList(Vec<TrackData>);
impl TrackDataList {
    pub fn from_buffer(buffer: &[u8], list: &PicTrackList) -> Self {
        let elems = list
            .list()
            .iter()
            .map(|pt| {
                let allocated_size = pt.track_allocated_space();
                let offset = pt.offset();
                TrackData::from_buffer(
                    &buffer[offset as usize..(offset + allocated_size) as usize],
                    &pt
                )
            })
            .collect_vec();
        TrackDataList(elems)
    }
}

#[derive(Debug)]
pub struct Hfe {
    header: PicFileFormatHeader,
    offset_lut: PicTrackList,
    tracks_data: TrackDataList
}

impl Hfe {
    pub fn open<P: AsRef<Utf8Path>>(fname: P) -> Self {
        let p = fname.as_ref();
        let buffer = std::fs::read(p).unwrap();
        Self::from_buffer(&buffer)
    }

    pub fn from_buffer(buffer: &[u8]) -> Self {
        let header = PicFileFormatHeader::from_buffer(&buffer[..512]);

        let offset_buffer_len = (header.number_of_track as usize) * 4;
        assert!(offset_buffer_len < 1024);
        let offset_lut = PicTrackList::from_buffer(
            &buffer[512..(512 + offset_buffer_len)],
            header.number_of_track
        );

        let tracks_data =
            TrackDataList::from_buffer(&buffer[512 + offset_buffer_len..], &offset_lut);

        Self {
            header,
            offset_lut,
            tracks_data
        }
    }
}



impl From<ExtendedDsk> for Hfe {
    // huge inspiration from https://sourceforge.net/p/hxcfloppyemu/code/HEAD/tree/HxCFloppyEmulator/libhxcfe/trunk/sources/loaders/cpcdsk_loader/cpcdsk_loader.c#l129
    fn from(dsk: ExtendedDsk) -> Self {

        let nb_sector_per_track = 9;

        let header = PicFileFormatHeader {
            format_revision: 0,
            number_of_track: dsk.nb_tracks_per_head(),
            number_of_side: dsk.nb_heads(),
            track_encoding: TrackEncoding::ISOIBM_MFM_ENCODING,
            bit_rate: panic!("250000"),
            floppy_rpm: 300,
            floppy_interface: FloppyInterface::CPC_DD_FLOPPYMODE,
            track_list_offset: 0x100,
            write_allowed: 0xff,
            single_step: Step::Double,
            track0s0_altencoding: TrackAltEncoding::No,
            track0s0_encoding: TrackEncoding::ISOIBM_MFM_ENCODING,
            track0s1_altencoding: TrackAltEncoding::No,
            track0s1_encoding: TrackEncoding::ISOIBM_MFM_ENCODING,
        };

        for t in 0..header.number_of_track {
            for s in 0..header.number_of_side {
                let track = dsk.get_track_information(s, t).unwrap();
            }
        }
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
