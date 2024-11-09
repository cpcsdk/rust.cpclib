use std::collections::BTreeMap;

#[cfg(target_os = "linux")]
use cpclib_common::camino::Utf8Path;
use scraper::{Html, Selector};

#[cfg(target_os = "linux")]
use crate::delegated::Compiler;
use crate::delegated::{cpclib_download, ArchiveFormat, DelegateApplicationDescription};
use crate::event::EventObserver;
#[cfg(target_os = "linux")]
use crate::runner::runner::Runner;
#[cfg(target_os = "linux")]
use crate::runner::ExternRunner;

pub const RASM_CMD: &str = "rasm";

static GITHUB_URL: &str = "https://github.com/";
static RASM_DOWNLOAD_URL: &str = "https://github.com/EdouardBERGE/rasm/releases/";

// Get the asset link that contains the dowload links for a given rasm version.
// the probelm is that this link is dynamic depending on the update (for a given name of rasm, you have several version, and only the latest one is publicly visible with a dynamic link)
fn rasm_get_version_url(version: RasmVersion) -> Result<String, String> {
    let version = version.name();

    // obtain the base dowload page
    let html = cpclib_download(RASM_DOWNLOAD_URL)?;
    let document = Html::parse_document(&html);

    let selector = Selector::parse("a")
        .map_err(|e| e.to_string())
        .map_err(|e| e.to_string())?;

    for link in document.select(&selector) {
        let content = link.inner_html();
        if content.contains(version) {
            return Ok(format!("{}{}", GITHUB_URL, link.attr("href").unwrap()));
        }
    }

    Err(format!("No download link found for {version}"))
}

fn rasm_get_assets_version_url(version: RasmVersion) -> Result<String, String> {
    let release_url = rasm_get_version_url(version)?;
    Ok(release_url.replace("/tag/", "/expanded_assets/"))
}

fn rasm_download_fn_urls_lin_win(version: RasmVersion) -> Result<(String, String), String> {
    let html = cpclib_download(&rasm_get_assets_version_url(version)?)?;
    let document = Html::parse_document(&html);
    let selector = Selector::parse("a")
        .map_err(|e| e.to_string())
        .map_err(|e| e.to_string())?;

    let mut map = BTreeMap::new();
    for element in document.select(&selector) {
        let name = element.text().collect::<String>();
        let name = name
            .replace("\n", "")
            .replace("\t", " ")
            .replace("    ", " ");
        let name = name.trim();
        map.insert(name.to_owned(), element.attr("href").unwrap().trim());
    }

    let windows_url = format!("{}/{}", GITHUB_URL, map.get("rasm_x64.exe").unwrap());
    let linux_url = format!("{}/{}", GITHUB_URL, map.get("Source code (zip)").unwrap());

    Ok((linux_url, windows_url))
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

    pub fn configuration<E: EventObserver + 'static>(&self) -> DelegateApplicationDescription<E> {
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
}

// Here we need to regularly look at rasm release file. because files often disapppear
cfg_match! {
    cfg(target_os = "linux") =>
    {
        impl RasmVersion {
            pub fn configuration<E:EventObserver +'static>(&self) -> DelegateApplicationDescription<E> {
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
                let mut get_url = move || -> Result<String, String> {
                    rasm_download_fn_urls_lin_win(version_cloned.clone())
                        .map(|urls| urls.0)
                };
                let get_url: Box<dyn Fn() -> Result<String,String>>  = Box::new(get_url);

                DelegateApplicationDescription::builder()
                    .download_fn_url(get_url) // we assume a modern CPU
                    .folder(self.folder())
                    .archive_format(ArchiveFormat::Zip)
                    .exec_fname("rasm")
                    .compile(install)
                    .build()
                }

            }
        }

    }
    cfg(target_os = "windows") =>
    {
        impl RasmVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {

                let version_cloned = self.clone();
                let mut get_url = move || -> Result<String, String> {
                    rasm_download_fn_urls_lin_win(version_cloned.clone())
                        .map(|urls| urls.1)
                };
                let get_url: Box<dyn Fn() -> Result<String,String>>  = Box::new(get_url);

                DelegateApplicationDescription::builder()
                    .download_fn_url(get_url) // we assume a modern CPU
                    .folder(self.folder())
                    .archive_format(ArchiveFormat::Raw)
                    .exec_fname("rasm_w64.exe")
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
    use super::{rasm_download_fn_urls_lin_win, RasmVersion};
    use crate::delegated::cpclib_download;
    use crate::runner::assembler::{rasm_get_assets_version_url, rasm_get_version_url};

    #[test]
    fn test_rasm_get_version_url() {
        assert!(dbg!(rasm_get_version_url(RasmVersion::Consolidation2024)).is_ok());
    }

    #[test]
    fn test_rasm_get_assets_version_url() {
        assert!(dbg!(rasm_get_assets_version_url(RasmVersion::Consolidation2024)).is_ok());
    }

    #[test]
    fn test_download_rasm() {
        let (lin, win) = rasm_download_fn_urls_lin_win(RasmVersion::Consolidation2024).unwrap();

        assert!(cpclib_download(dbg!(&lin)).is_ok());
        assert!(cpclib_download(dbg!(&win)).is_ok());
    }
}
