use cpclib_common::event::EventObserver;
use disark::{DISARK_CMD, DisarkVersion};

use crate::delegated::{DelegateApplicationDescription, InternetStaticCompiledApplication};

pub mod disark;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExternDisassembler {
    Disark(DisarkVersion)
}

impl ExternDisassembler {
    pub fn get_command(&self) -> &str {
        match self {
            ExternDisassembler::Disark(_disark_version) => DISARK_CMD
        }
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        match self {
            ExternDisassembler::Disark(v) => v.configuration()
        }
    }
}

#[cfg(test)]
mod test {
    use cpclib_common::network;

    use crate::delegated::StaticInformation;
    use crate::runner::disassembler::disark::DisarkVersion;

    #[test]
    fn test_download_disark() {
        let urls = DisarkVersion::default().static_download_urls();
        assert!(network::download(dbg!(urls.linux.as_ref().unwrap())).is_ok());
        assert!(network::download(dbg!(urls.windows.as_ref().unwrap())).is_ok());
    }
}
