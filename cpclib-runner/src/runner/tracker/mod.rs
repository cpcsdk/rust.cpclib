use cpclib_common::event::EventObserver;
use at3::{At3Version, AT_CMD};

use crate::delegated::{DelegateApplicationDescription, InternetStaticCompiledApplication};

pub mod at3;


#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Tracker {
	At3(At3Version)
}


impl Tracker {
	pub fn get_command(&self) -> &str {
		match self {
			Tracker::At3(_) => AT_CMD,
		}
	}

	pub fn configuration<E: EventObserver +'static>(&self) -> DelegateApplicationDescription<E> {
		match self {
			Tracker::At3(v) => v.configuration(),
		}
	}
}


#[cfg(test)]
mod test {
    use crate::{delegated::{cpclib_download, StaticInformation}, runner::tracker::at3::At3Version};

    #[test]
    fn test_download_at3() {
        let urls = At3Version::default().static_download_urls();
        assert!(cpclib_download(dbg!(urls.linux.as_ref().unwrap())).is_ok());
        assert!(cpclib_download(dbg!(urls.windows.as_ref().unwrap())).is_ok());
    }
}