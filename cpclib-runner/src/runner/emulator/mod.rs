pub mod ace;
pub mod amspirit;
pub mod cpcec;
pub mod sugarbox;
pub mod winape;

use std::path::absolute;

use cpclib_common::{camino::{Utf8Path, Utf8PathBuf}, event::EventObserver};




pub use ace::*;
pub use amspirit::*;
pub use cpcec::*;
pub use sugarbox::*;
pub use winape::*;

use crate::delegated::{DelegateApplicationDescription, GithubCompilableApplication, GithubCompiledApplication, InternetDynamicCompiledApplication, InternetStaticCompiledApplication};





#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Emulator {
    Ace(AceVersion),
    Amspirit(AmspiritVersion),
    Cpcec(CpcecVersion),
    Winape(WinapeVersion),
    SugarBoxV2(SugarBoxV2Version)
}

impl Default for Emulator {
    fn default() -> Self {
        Emulator::Ace(AceVersion::default())
    }
}

impl Emulator {
    pub fn ace_version(&self) -> Option<&AceVersion> {
        match self {
            Emulator::Ace(v) => Some(v),
            _ => None
        }
    }

    pub fn is_ace(&self) -> bool {
        matches!(self, Emulator::Ace(_))
    }

    pub fn is_amspirit(&self) -> bool {
        matches!(self, Emulator::Amspirit(_))
    }

    pub fn is_cpcec(&self) -> bool {
        matches!(self, Emulator::Cpcec(_))
    }

    pub fn is_winape(&self) -> bool {
        matches!(self, Emulator::Winape(_))
    }

    pub fn get_command(&self) -> &str {
        match self {
            Emulator::Ace(_) => ACE_CMD,
            Emulator::Amspirit(_) => AMSPIRIT_CMD,
            Emulator::Cpcec(_) => CPCEC_CMD,
            Emulator::Winape(_) => WINAPE_CMD,
            Emulator::SugarBoxV2(_)=> SUGARBOX_V2_CMD
        }
    }

    pub fn window_name_corresponds(&self, window_name: &str) -> bool {
        let window_name = window_name.trim();
        match self {
            Emulator::Ace(_) => window_name.starts_with("ACE-DL -"),
            Emulator::Cpcec(_) => window_name.starts_with("CPCEC "),
            Emulator::Winape(_) => window_name.starts_with("Windows Amstrad Plus"),
            Emulator::Amspirit(_) => window_name.starts_with("AMSpiriT"),
            Emulator::SugarBoxV2(_) => unimplemented!()
        }
    }

    pub fn screenshots_folder(&self) -> Utf8PathBuf {
        match self {
            Emulator::Ace(v) => v.screenshots_folder(),
            _ => unimplemented!()
        }
    }

    pub fn roms_folder(&self) -> Utf8PathBuf {
        match self {
            Emulator::Ace(v) => v.roms_folder(),
            Emulator::Cpcec(v) => v.roms_folder(),
            Emulator::Winape(v) => v.roms_folder(),
            _ => unimplemented!()
        }
    }

    pub fn albireo_folder(&self) -> Utf8PathBuf {
        match self {
            Emulator::Ace(v) => v.albireo_folder(),
            _ => unimplemented!()
        }
    }

    /// Handle filename to make them work properly using wine
    pub fn wine_compatible_fname(&self, p: &Utf8Path) -> Result<Utf8PathBuf, String> {
        let abspath = absolute(p).map_err(|e| e.to_string())?;
        let abspath = Utf8PathBuf::from_path_buf(abspath).map_err(|e| "File error".to_owned())?;
        if cfg!(target_os = "windows") {
            Ok(abspath)
        }
        else {
            Ok(("Z:".to_owned() + abspath.as_str()).into())
        }
    }
}






impl Emulator {
    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        match self {
            Emulator::Ace(v) => v.configuration(),
            Emulator::Cpcec(v) => v.configuration(),
            Emulator::Winape(v) => v.configuration(),
            Emulator::Amspirit(v) => v.configuration(),
            Emulator::SugarBoxV2(v) => v.configuration()
        }
    }
}







#[cfg(test)]
mod test {
    use crate::{delegated::{cpclib_download, DynamicUrlInformation, GithubInformation}, runner::emulator::AceVersion};

    use super::{SugarBoxV2Version, WinapeVersion};

    #[test]
    fn test_download_ace() {
        let urls= AceVersion::default().dynamic_download_urls().unwrap();
        assert!(cpclib_download(dbg!(urls.linux.as_ref().unwrap())).is_ok());
        assert!(cpclib_download(dbg!(urls.windows.as_ref().unwrap())).is_ok());
    }

    #[test]
    fn test_download_sugarbox() {
        let urls = SugarBoxV2Version::default().github_download_urls().unwrap();
        assert!(cpclib_download(dbg!(urls.linux.as_ref().unwrap())).is_ok());
        assert!(cpclib_download(dbg!(urls.windows.as_ref().unwrap())).is_ok());

    }

    #[test]
    fn test_download_winape() {
    }

}
