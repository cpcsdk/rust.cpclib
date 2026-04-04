use std::fmt::Display;

#[cfg(target_os = "linux")]
use cpclib_common::camino::Utf8Path;

use crate::delegated::{ArchiveFormat, Compiler, DelegateApplicationDescription};
use crate::event::EventObserver;
#[cfg(target_os = "linux")]
use crate::runner::ExternRunner;
#[cfg(target_os = "linux")]
use crate::runner::Runner;

pub const TWO_CDT_CMD: &str = "2cdt";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum TwoCdtVersion {
    #[default]
    V1
}

impl TwoCdtVersion {
    pub fn get_command(&self) -> &str {
        TWO_CDT_CMD
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        #[cfg(target_os = "windows")]
        let exec_fname: &'static str = "2cdt.exe";
        #[cfg(not(target_os = "windows"))]
        let exec_fname: &'static str = "2cdt";

        DelegateApplicationDescription::builder()
            .download_fn_url("https://cpctech.cpcwiki.de/download/2cdt.zip")
            .folder("2cdt")
            .archive_format(ArchiveFormat::Zip)
            .exec_fname(exec_fname)
            .build()
    }
}

impl Display for TwoCdtVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "2cdt")
    }
}
