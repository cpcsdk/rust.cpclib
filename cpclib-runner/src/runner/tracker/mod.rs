use at3::extra::*;
use at3::{At3Version, AT_CMD};
use chipnsfx::{ChipnsfxVersion, CHIPNSFX_CMD};
use cpclib_common::event::EventObserver;

use crate::delegated::{DelegateApplicationDescription, InternetStaticCompiledApplication};

pub mod at3;
pub mod chipnsfx;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Tracker {
    At3(At3Version),
    Chipnsfx(ChipnsfxVersion)
}

impl Tracker {
    pub fn get_command(&self) -> &str {
        match self {
            Tracker::At3(_) => AT_CMD,
            Tracker::Chipnsfx(_) => CHIPNSFX_CMD
        }
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        match self {
            Tracker::At3(v) => v.configuration(),
            Tracker::Chipnsfx(v) => v.configuration()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SongConverter {
    SongToAkm(SongToAkm),
    SongToAkg(SongToAkg),
    SongToAky(SongToAky),
    SongToEvents(SongToEvents),
    SongToRaw(SongToRaw),
    SongToSoundEffects(SongToSoundEffects),
    SongToVgm(SongToVgm),
    SongToWav(SongToWav),
    SongToYm(SongToYm)
}

impl SongConverter {
    pub fn get_command(&self) -> &str {
        match self {
            Self::SongToAkg(_) => SongToAkg::CMD,
            Self::SongToAkm(_) => SongToAkm::CMD,
            Self::SongToAky(_) => SongToAky::CMD,
            Self::SongToEvents(_) => SongToEvents::CMD,
            Self::SongToRaw(_) => SongToRaw::CMD,
            Self::SongToSoundEffects(_) => SongToSoundEffects::CMD,
            Self::SongToVgm(_) => SongToVgm::CMD,
            Self::SongToWav(_) => SongToWav::CMD,
            Self::SongToYm(_) => SongToYm::CMD
        }
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        match self {
            Self::SongToAkg(v) => v.configuration(),
            Self::SongToAkm(v) => v.configuration(),
            Self::SongToAky(v) => v.configuration(),
            Self::SongToEvents(v) => v.configuration(),
            Self::SongToRaw(v) => v.configuration(),
            Self::SongToSoundEffects(v) => v.configuration(),
            Self::SongToVgm(v) => v.configuration(),
            Self::SongToWav(v) => v.configuration(),
            Self::SongToYm(v) => v.configuration()
        }
    }
}

#[cfg(test)]
mod test {
    use crate::delegated::{cpclib_download, StaticInformation};
    use crate::runner::tracker::at3::At3Version;
    use crate::runner::tracker::chipnsfx::ChipnsfxVersion;

    #[test]
    fn test_download_at3() {
        let urls = At3Version::default().static_download_urls();
        assert!(cpclib_download(dbg!(urls.linux.as_ref().unwrap())).is_ok());
        assert!(cpclib_download(dbg!(urls.windows.as_ref().unwrap())).is_ok());
    }

    #[test]
    #[ignore]
    fn test_download_chipnsfx() {
        let urls = ChipnsfxVersion::default().static_download_urls();
        assert!(cpclib_download(dbg!(urls.linux.as_ref().unwrap())).is_ok());
        assert!(cpclib_download(dbg!(urls.windows.as_ref().unwrap())).is_ok());
    }
}
