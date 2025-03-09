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

#[cfg(test)]
mod test {
    use crate::delegated::{cpclib_download, StaticInformation};
    use crate::runner::tracker::at3::At3Version;

    #[test]
    fn test_download_at3() {
        let urls = At3Version::default().static_download_urls();
        assert!(cpclib_download(dbg!(urls.linux.as_ref().unwrap())).is_ok());
        assert!(cpclib_download(dbg!(urls.windows.as_ref().unwrap())).is_ok());
    }


    #[test]
    #[ignore]
    fn test_download_chipnsfx() {
        let urls = Chipnsfx::default().static_download_urls();
        assert!(cpclib_download(dbg!(urls.linux.as_ref().unwrap())).is_ok());
        assert!(cpclib_download(dbg!(urls.windows.as_ref().unwrap())).is_ok());
    }
}
