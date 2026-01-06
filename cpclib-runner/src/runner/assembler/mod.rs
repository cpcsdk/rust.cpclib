pub mod rasm;
pub mod sjasmplus;
pub mod uz80;
pub mod vasm;

use cpclib_common::event::EventObserver;
pub use rasm::{RASM_CMD, RasmVersion};
pub use sjasmplus::{SJASMPLUS_CMD, SjasmplusVersion};
use uz80::{UZ80_CMD, Uz80Version};
pub use vasm::{VASM_CMD, VasmVersion};

use crate::delegated::{
    DelegateApplicationDescription, GithubCompilableApplication, InternetStaticCompiledApplication
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExternAssembler {
    Rasm(RasmVersion),
    Sjasmplus(SjasmplusVersion),
    Uz80(Uz80Version),
    Vasm(VasmVersion)
}

impl ExternAssembler {
    pub fn get_command(&self) -> &str {
        match self {
            ExternAssembler::Rasm(_) => RASM_CMD,
            ExternAssembler::Vasm(_) => VASM_CMD,
            ExternAssembler::Uz80(_) => UZ80_CMD,
            ExternAssembler::Sjasmplus(_) => SJASMPLUS_CMD
        }
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        match self {
            ExternAssembler::Rasm(r) => r.configuration(),
            ExternAssembler::Sjasmplus(r) => r.configuration(),
            ExternAssembler::Uz80(u) => u.configuration(),
            ExternAssembler::Vasm(r) => r.configuration()
        }
    }
}
#[cfg(test)]
mod test {
    use super::RasmVersion;
    use crate::delegated::{GithubInformation, StaticInformation};
    use cpclib_common::network;
    use crate::runner::assembler::{SjasmplusVersion, VasmVersion};

    #[test]
    fn test_download_rasm() {
        let urls = RasmVersion::default().github_download_urls().unwrap();
        assert!(network::download(dbg!(&urls.linux.unwrap())).is_ok());
        assert!(network::download(dbg!(&urls.windows.unwrap())).is_ok());
    }

    #[test]
    fn test_download_sjasmplus() {
        let urls = SjasmplusVersion::default().github_download_urls().unwrap();
        assert!(network::download(dbg!(&urls.linux.unwrap())).is_ok());
        assert!(network::download(dbg!(&urls.windows.unwrap())).is_ok());
    }

    #[test]
    fn test_download_vasm() {
        let urls = VasmVersion::default().static_download_urls();
        assert!(network::download(dbg!(urls.linux.as_ref().unwrap())).is_ok());
        assert!(network::download(dbg!(urls.windows.as_ref().unwrap())).is_ok());
    }
}
