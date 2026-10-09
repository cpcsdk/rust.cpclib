use std::collections::BTreeMap;

use cpclib_common::camino::Utf8PathBuf;
use cpclib_common::network;
use directories::BaseDirs;
use scraper::{Html, Selector};

use crate::delegated::{
    ArchiveFormat, DownloadableInformation, DynamicUrlInformation, ExecutableInformation,
    InternetDynamicCompiledApplication, MutiplatformUrls
};

pub const ACE_CMD: &str = "ace";

const ACE_URL: &str = "https://roudoudou.com/ACE-DL";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum AceVersion {
    #[default]
    UnknownLastVersion, // directly parse the webpage
    Bnd4,      // 2024/10/26
    ZenSummer, // 2024/08/18
    WakePoint  // 2024/06/21
}

/// Which of the download page's links to take on Linux. The page names its
/// builds by distribution (`Ubuntu 26.04 LTS`, `Ubuntu 24.04 LTS`, `Debian
/// 12`, and once with an ` (AVX2)` suffix): the one of the running
/// distribution (`os_release` is the content of `/etc/os-release`; its `ID`
/// and `VERSION_ID`) wins, an
/// AVX2 build over a plain one, then Ubuntu 24.04, any Ubuntu, any Debian - the
/// page's wording is not ours to rely on exactly.
fn linux_download_key<'a>(
    keys: impl Iterator<Item = &'a str> + Clone,
    os_release: &str
) -> Option<&'a str> {
    let field = |name: &str| {
        os_release
            .lines()
            .find_map(|l| l.strip_prefix(name)?.strip_prefix('='))
            .map(|v| v.trim().trim_matches('"').to_string())
    };
    // `ID` (`ubuntu`, `debian`) rather than `NAME` (`Debian GNU/Linux`): it is
    // the one that reads like the page's own labels
    let host = match (field("ID"), field("VERSION_ID")) {
        (Some(id), Some(version)) if !id.is_empty() => {
            let mut chars = id.chars();
            let name: String = chars
                .next()
                .into_iter()
                .flat_map(char::to_uppercase)
                .chain(chars)
                .collect();
            Some(format!("{name} {version}"))
        },
        _ => None
    };

    let preferred = |prefix: &str| {
        let mut matching = keys.clone().filter(|k| k.starts_with(prefix));
        let first = matching.next()?;
        Some(
            std::iter::once(first)
                .chain(matching)
                .find(|k| k.contains("AVX2"))
                .unwrap_or(first)
        )
    };

    host.as_deref()
        .and_then(preferred)
        .or_else(|| preferred("Ubuntu 24.04"))
        .or_else(|| preferred("Ubuntu"))
        .or_else(|| preferred("Debian"))
}

impl DownloadableInformation for AceVersion {
    fn target_os_archive_format(&self) -> crate::delegated::ArchiveFormat {
        #[cfg(target_os = "windows")]
        return ArchiveFormat::Zip;
        #[cfg(target_os = "linux")]
        return ArchiveFormat::TarGz;
        #[cfg(target_os = "haiku")]
        return ArchiveFormat::Zip;
        #[cfg(target_os = "macos")]
        return ArchiveFormat::Zip;
    }
}
impl DynamicUrlInformation for AceVersion {
    fn dynamic_download_urls(&self) -> Result<MutiplatformUrls, String> {
        match self {
            Self::UnknownLastVersion => {
                let mut content = network::download(ACE_URL)?;
                let mut html = String::new();
                content
                    .read_to_string(&mut html)
                    .map_err(|e| e.to_string())?;

                let document = Html::parse_document(&html);
                let selector = Selector::parse("#dl td a")
                    .map_err(|e| e.to_string())
                    .map_err(|e| e.to_string())?;

                let mut map = BTreeMap::new();
                for element in document.select(&selector) {
                    map.insert(element.inner_html(), element.attr("href").unwrap());
                }

                let macos = map
                    .get("All versions")
                    .map(|url| format!("{ACE_URL}/{url}"));
                let windows = map
                    .get("x64 (64 bits)")
                    .map(|url| format!("{ACE_URL}/{url}"));
                let linux = linux_download_key(
                    map.keys().map(String::as_str),
                    &std::fs::read_to_string("/etc/os-release").unwrap_or_default()
                )
                .and_then(|key| map.get(key))
                .map(|url| format!("{ACE_URL}/{url}"));

                Ok(MutiplatformUrls {
                    linux,
                    windows,
                    macos,
                    haiku: None
                })
            },
            AceVersion::Bnd4 => {
                Ok(MutiplatformUrls {
                    linux: Some("https://roudoudou.com/ACE-DL/BZen.tar.gz".to_string()),
                    windows: Some("https://roudoudou.com/ACE-DL/W64bnd4.zip".to_string()),
                    macos: None,
                    haiku: None
                })
            },
            AceVersion::ZenSummer => {
                Ok(MutiplatformUrls {
                    linux: Some("https://roudoudou.com/ACE-DL/LinuxZenSummer.tar.gz".to_string()),
                    windows: Some("https://roudoudou.com/ACE-DL/Win64Summer.zip".to_string()),
                    macos: None,
                    haiku: None
                })
            },
            AceVersion::WakePoint => {
                Ok(MutiplatformUrls {
                    linux: Some("https://roudoudou.com/ACE-DL/BZen.tar.gz".to_string()),
                    windows: Some("https://roudoudou.com/ACE-DL/BWIN64.zip".to_string()),
                    macos: Some("https://roudoudou.com/ACE-DL/BMAC.zip".to_string()),
                    haiku: None
                })
            },
        }
    }
}

impl ExecutableInformation for AceVersion {
    fn target_os_folder(&self) -> &'static str {
        match self {
            AceVersion::UnknownLastVersion => "UnknownLastAceVersion",
            AceVersion::Bnd4 => "AceWakePoint",
            AceVersion::ZenSummer => "AceBnd4",
            AceVersion::WakePoint => "AceZenSummer"
        }
    }

    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        return "AceDL.exe";
        #[cfg(target_os = "linux")]
        return "AceDL";
        #[cfg(target_os = "haiku")]
        return "ACE";
        #[cfg(target_os = "macos")]
        return "AceDL.app/Contents/MacOS/AceDL";
    }
}

impl InternetDynamicCompiledApplication for AceVersion {}

impl AceVersion {
    pub fn config_file(&self) -> Utf8PathBuf {
        let p = match self {
            Self::ZenSummer | Self::Bnd4 | Self::UnknownLastVersion => {
                BaseDirs::new()
                    .unwrap()
                    .config_local_dir()
                    .join("ACE-DL_futuristics/config.cfg")
            },
            _ => unimplemented!()
        };

        Utf8PathBuf::from_path_buf(p).unwrap()
    }
}

impl AceVersion {
    pub fn screenshots_folder(&self) -> Utf8PathBuf {
        let conf = self.configuration::<()>();

        conf.cache_folder().join("export").join("screenshot")
    }

    pub fn roms_folder(&self) -> Utf8PathBuf {
        let conf = self.configuration::<()>();

        conf.cache_folder().join("media").join("rom")
    }

    pub fn albireo_folder(&self) -> Utf8PathBuf {
        let conf = self.configuration::<()>();

        conf.cache_folder().join("media").join("albireo1")
    }
}

#[cfg(test)]
mod tests {
    use super::linux_download_key;

    const NEW_PAGE: &[&str] = &["Ubuntu 26.04 LTS", "Ubuntu 24.04 LTS", "Debian 12"];
    const OLD_PAGE: &[&str] = &["Ubuntu 24.04 LTS", "Ubuntu 24.04 LTS (AVX2)", "Debian 12"];

    fn os(id: &str, version: &str) -> String {
        format!(
            "PRETTY_NAME=\"x\"\nNAME=\"Whatever GNU/Linux\"\nID={id}\nVERSION_ID=\"{version}\"\n"
        )
    }

    fn key<'a>(keys: &[&'a str], os: &str) -> Option<&'a str> {
        linux_download_key(keys.iter().copied(), os)
    }

    #[test]
    fn the_running_distribution_is_taken() {
        assert_eq!(
            key(NEW_PAGE, &os("ubuntu", "26.04")),
            Some("Ubuntu 26.04 LTS")
        );
        assert_eq!(
            key(NEW_PAGE, &os("ubuntu", "24.04")),
            Some("Ubuntu 24.04 LTS")
        );
        assert_eq!(key(NEW_PAGE, &os("debian", "12")), Some("Debian 12"));
    }

    #[test]
    fn an_avx2_build_wins_over_a_plain_one() {
        let key = linux_download_key(OLD_PAGE.iter().copied(), &os("ubuntu", "24.04"));
        assert_eq!(key, Some("Ubuntu 24.04 LTS (AVX2)"));
    }

    #[test]
    fn an_unlisted_distribution_falls_back_to_ubuntu_24_04() {
        let key = linux_download_key(NEW_PAGE.iter().copied(), &os("fedora", "42"));
        assert_eq!(key, Some("Ubuntu 24.04 LTS"));
        // and an unreadable os-release too
        assert_eq!(
            linux_download_key(NEW_PAGE.iter().copied(), ""),
            Some("Ubuntu 24.04 LTS")
        );
        assert_eq!(linux_download_key(["Windows"].into_iter(), ""), None);
    }
}
