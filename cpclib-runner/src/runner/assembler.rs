use std::collections::BTreeMap;

#[cfg(target_os = "linux")]
use cpclib_common::camino::Utf8Path;
use scraper::{Html, Selector};

#[cfg(target_os = "linux")]
use crate::delegated::Compiler;
use crate::delegated::{github_download_urls, github_get_assets_for_version_url, ArchiveFormat, DelegateApplicationDescription, GithubUrls};
use crate::event::EventObserver;
#[cfg(target_os = "linux")]
use crate::runner::runner::Runner;
#[cfg(target_os = "linux")]
use crate::runner::ExternRunner;

pub const RASM_CMD: &str = "rasm";

static RASM_REPO_URL: &str = "https://github.com/EdouardBERGE/rasm";


fn rasm_get_assets_url(version: RasmVersion) -> Result<String, String> {
    github_get_assets_for_version_url(RASM_REPO_URL, version.name())
}

fn rasm_download_urls(version: RasmVersion) -> Result<GithubUrls, String> {
    github_download_urls(
        RASM_REPO_URL,
        version.name(),
        Some("Source code (zip)"),
        Some("rasm_x64.exe"),
        None
    )
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExternAssembler {
    Rasm(RasmVersion)
}

impl ExternAssembler {
    pub fn get_command(&self) -> &str {
        match self {
            ExternAssembler::Rasm(_) => RASM_CMD
        }
    }

    pub fn configuration<E: EventObserver +'static>(&self) -> DelegateApplicationDescription<E> {
        match self {
            ExternAssembler::Rasm(r) => r.configuration()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RasmVersion {
    Consolidation2024 // V2_2_5
}

impl Default for RasmVersion {
    fn default() -> Self {
        Self::Consolidation2024
    }
}

impl RasmVersion {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Consolidation2024 => "Consolidation"
        }
    }

    pub fn folder(&self) -> &'static str {
        match self {
            Self::Consolidation2024 => "rasm_consolidation"
        }
    }

    pub fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(target_os="windows")]
        return "rasm_w64.exe";
        #[cfg(target_os="macos")]
        unimplemented!();
        #[cfg(target_os="linux")]
        return "rasm";

    }
}




// Here we need to regularly look at rasm release file. because files often disapppear
cfg_match! {
    cfg(target_os = "linux") =>
    {
        impl RasmVersion {
            pub fn configuration<E:EventObserver + 'static>(&self) -> DelegateApplicationDescription<E> {
 {
                let install : Box<dyn Fn(&Utf8Path, &E) -> Result<(), String>> = Box::new(|_path: &Utf8Path, o: &E| -> Result<(), String>{
                    let command = vec!["make"];
                    ExternRunner::default().inner_run(&command, o)?;

                    let command = vec!["mv", "rasm.exe", "rasm"];
                    ExternRunner::default().inner_run(&command, o)?;

                    Ok(())
                });
                let install = Compiler::from(install);

                let version_cloned = self.clone();
                let get_url = move || -> Result<String, String> {
                    rasm_download_urls(version_cloned.clone())
                        .map(|urls| urls.linux.unwrap())
                };
                let get_url: Box<dyn Fn() -> Result<String,String>>  = Box::new(get_url);

                DelegateApplicationDescription::builder()
                    .download_fn_url(get_url) // we assume a modern CPU
                    .folder(self.folder())
                    .archive_format(ArchiveFormat::Zip)
                    .exec_fname(self.target_os_exec_fname())
                    .compile(install)
                    .build()
                }

            }
        }

    }

    cfg(target_os = "windows") => {
        impl RasmVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        
                let version_cloned = self.clone();
                let get_url = move || -> Result<String, String> {
                    rasm_download_urls(version_cloned.clone())
                        .map(|urls| urls.target_os_url().unwrap().clone())
                };
                let get_url: Box<dyn Fn() -> Result<String,String>>  = Box::new(get_url);
        
                DelegateApplicationDescription::builder()
                    .download_fn_url(get_url) // we assume a modern CPU
                    .folder(self.folder())
                    .archive_format(ArchiveFormat::Raw)
                    .exec_fname(self.target_os_exec_fname())
                    .build()
            }
        }
    }

    cfg(target_os = "macos") =>
    {

    }
    _ => {
    }
}

#[cfg(test)]
mod test {
    use super::{rasm_download_urls, RasmVersion};
    use crate::delegated::cpclib_download;


    #[test]
    fn test_download_rasm() {
        let urls= rasm_download_urls(RasmVersion::Consolidation2024).unwrap();

        assert!(cpclib_download(dbg!(&urls.linux.unwrap())).is_ok());
        assert!(cpclib_download(dbg!(&urls.windows.unwrap())).is_ok());
    }
}
