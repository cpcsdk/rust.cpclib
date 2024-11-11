pub mod rasm;
pub mod sjasmplus;
pub mod vasm;

use cpclib_common::event::EventObserver;
pub use rasm::{RasmVersion, RASM_CMD};
pub use sjasmplus::{SjasmplusVersion, SJASMPLUS_CMD};
pub use vasm::{VasmVersion, VASM_CMD};

use crate::delegated::{
    DelegateApplicationDescription, GithubCompilableApplication, InternetStaticCompiledApplication
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExternAssembler {
    Rasm(RasmVersion),
    Sjasmplus(SjasmplusVersion),
    Vasm(VasmVersion)
}

impl ExternAssembler {
    pub fn get_command(&self) -> &str {
        match self {
            ExternAssembler::Rasm(_) => RASM_CMD,
            ExternAssembler::Vasm(_) => VASM_CMD,
            ExternAssembler::Sjasmplus(_) => SJASMPLUS_CMD
        }
    }

    pub fn configuration<E: EventObserver + 'static>(&self) -> DelegateApplicationDescription<E> {
        match self {
            ExternAssembler::Rasm(r) => r.configuration(),
            ExternAssembler::Sjasmplus(r) => r.configuration(),
            ExternAssembler::Vasm(r) => r.configuration()
        }
    }
}
#[cfg(test)]
mod test {
    use super::RasmVersion;
    use crate::delegated::{cpclib_download, GithubInformation, StaticInformation};
    use crate::runner::assembler::{SjasmplusVersion, VasmVersion};

    #[test]
    fn test_download_rasm() {
        let urls = RasmVersion::default().github_download_urls().unwrap();
        assert!(cpclib_download(dbg!(&urls.linux.unwrap())).is_ok());
        assert!(cpclib_download(dbg!(&urls.windows.unwrap())).is_ok());
    }

    #[test]
    fn test_download_sjasmplus() {
        let urls = SjasmplusVersion::default().github_download_urls().unwrap();
        assert!(cpclib_download(dbg!(&urls.linux.unwrap())).is_ok());
        assert!(cpclib_download(dbg!(&urls.windows.unwrap())).is_ok());
    }

    #[test]
    fn test_download_vasm() {
        let urls = VasmVersion::default().static_download_urls();
        assert!(cpclib_download(dbg!(urls.linux.as_ref().unwrap())).is_ok());
        assert!(cpclib_download(dbg!(urls.windows.as_ref().unwrap())).is_ok());
    }
}
