use cpclib_common::event::EventObserver;
use disark::{DisarkVersion, DISARK_CMD};

use crate::delegated::{DelegateApplicationDescription, InternetStaticCompiledApplication};

pub mod disark;


#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExternDisassembler {
	Disark(DisarkVersion)
}


impl ExternDisassembler {
	pub fn get_command(&self) -> &str {
		match self {
			ExternDisassembler::Disark(disark_version) => DISARK_CMD,
		}
	}

	pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
		match self {
			ExternDisassembler::Disark(v) => v.configuration(),
		}
	}
}


#[cfg(test)]
mod test {
    use crate::delegated::{cpclib_download, StaticInformation};
    use crate::runner::disassembler::disark::DisarkVersion;

    #[test]
    fn test_download_disark() {
        let urls = DisarkVersion::default().static_download_urls();
        assert!(cpclib_download(dbg!(urls.linux.as_ref().unwrap())).is_ok());
        assert!(cpclib_download(dbg!(urls.windows.as_ref().unwrap())).is_ok());
    }
}