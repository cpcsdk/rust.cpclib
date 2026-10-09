use cpclib_common::network;
use scraper::{Html, Selector};

use crate::delegated::{
    ArchiveFormat, DelegateApplicationDescription, PostInstallFn, UrlGenerator
};
use crate::event::EventObserver;

/// Where GrafX2 lists its current builds: every release is a set of GitLab job
/// artifacts, whose addresses (the job numbers) change each time - so they are
/// read from here rather than written down, as they were (and went dead).
const DOWNLOAD_PAGE: &str = "https://grafx2.gitlab.io/grafX2/";

/// The Linux AppImage and the Windows (SDL2) zip the download page links to.
fn download_links(html: &str) -> (Option<String>, Option<String>) {
    let document = Html::parse_document(html);
    let links: Vec<&str> = Selector::parse("a")
        .map(|selector| {
            document
                .select(&selector)
                .filter_map(|a| a.attr("href"))
                .collect()
        })
        .unwrap_or_default();
    let find = |wanted: &dyn Fn(&str) -> bool| {
        links
            .iter()
            .find(|l| l.starts_with("https://") && wanted(l))
            .map(|l| l.to_string())
    };
    (
        find(&|l| l.ends_with("-x86_64.AppImage")),
        find(&|l| l.contains("/grafx2-sdl2-") && l.ends_with("-win32.zip"))
    )
}

/// The address of this OS's current build: `linux` or `windows` of
/// [`download_links`].
fn current_download_url(windows: bool) -> Result<String, String> {
    use std::io::Read;

    let mut html = String::new();
    network::download(DOWNLOAD_PAGE)?
        .read_to_string(&mut html)
        .map_err(|e| e.to_string())?;
    let (linux, win) = download_links(&html);
    (if windows { win } else { linux })
        .ok_or_else(|| format!("{DOWNLOAD_PAGE} does not link a GrafX2 build for this OS"))
}

pub const GRAFX2_CMD: &str = "grafx2";
pub const DOWNLOAD_URL_V2_9_MACOS: &str = "https://pulkomandy.tk/projects/GrafX2/downloads/71";
#[derive(Default)]
pub enum Grafx2Version {
    #[default]
    V2_9
}

impl Grafx2Version {
    pub fn get_command(&self) -> &str {
        GRAFX2_CMD
    }
}

impl Grafx2Version {
    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        let url: UrlGenerator = match self {
            #[cfg(target_os = "windows")]
            Grafx2Version::V2_9 => {
                let generator: Box<dyn Fn() -> Result<String, String>> =
                    Box::new(|| current_download_url(true));
                generator.into()
            },
            #[cfg(target_os = "linux")]
            Self::V2_9 => {
                let generator: Box<dyn Fn() -> Result<String, String>> =
                    Box::new(|| current_download_url(false));
                generator.into()
            },
            #[cfg(target_os = "macos")]
            Self::V2_9 => DOWNLOAD_URL_V2_9_MACOS.into(),
            #[cfg(target_os = "haiku")]
            Self::V2_9 => "cmd:grafx2".into(),
            #[allow(unreachable_patterns)]
            _ => unreachable!()
        };

        // not named after the version: the build it fetches is the current one
        let folder = match self {
            Grafx2Version::V2_9 => "grafx2_latest"
        };

        #[cfg(target_os = "windows")]
        let exec = "bin/grafx2-sdl2.exe";
        #[cfg(target_os = "linux")]
        let exec = "GrafX2.AppImage";
        #[cfg(target_os = "macos")]
        let exec = "GrafX2";
        #[cfg(target_os = "haiku")]
        let exec = "grafx2";

        #[cfg(target_os = "windows")]
        let archive_format = ArchiveFormat::Zip;
        #[cfg(target_os = "linux")]
        let archive_format = ArchiveFormat::Raw;
        #[cfg(target_os = "macos")]
        let archive_format = ArchiveFormat::Zip;
        #[cfg(target_os = "haiku")]
        let archive_format = ArchiveFormat::Raw;

        let builder = DelegateApplicationDescription::builder()
            .download_fn_url(url) // we assume a modern CPU
            .folder(folder)
            .archive_format(archive_format)
            .exec_fname(exec);

        // On linux it is needed to add execution right to the downloaded appimage
        #[cfg(target_os = "linux")]
        let builder = {
            let post_install: Box<PostInstallFn<E>> = Box::new(
                |desc: &DelegateApplicationDescription<E>, _o: &E| -> Result<(), String> {
                    use std::os::unix::fs::PermissionsExt;

                    let app_image = desc.exec_fname();
                    let mut perms = fs_err::metadata(&app_image).unwrap().permissions();
                    let mode = perms.mode() | 0o100; // Add execution mode
                    perms.set_mode(mode);
                    let _ = fs_err::set_permissions(&app_image, perms);
                    Ok(())
                }
            );
            builder.post_install(post_install)
        };

        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::download_links;

    const PAGE: &str = r#"<html><body>
      <a href="https://appimage.org/">AppImage</a>
      <a href="https://gitlab.com/GrafX2/grafX2/-/jobs/1/artifacts/raw/grafx2-2.9.3276-src.tgz">sources</a>
      <a href="https://gitlab.com/GrafX2/grafX2/-/jobs/1/artifacts/raw/GrafX2-2.9.3276-x86_64.AppImage">Linux</a>
      <a href="https://gitlab.com/GrafX2/grafX2/-/jobs/2/artifacts/raw/grafx2-sdl-2.9.3276-win32.zip">SDL1</a>
      <a href="https://gitlab.com/GrafX2/grafX2/-/jobs/2/artifacts/raw/grafx2-sdl2-2.9.3276-win32.zip">SDL2</a>
      <a href="https://gitlab.com/GrafX2/grafX2/-/jobs/2/artifacts/raw/install/grafx2-sdl2-2.9.3276.win32.exe">installer</a>
    </body></html>"#;

    #[test]
    fn the_appimage_and_the_sdl2_zip_are_picked() {
        let (linux, windows) = download_links(PAGE);
        assert_eq!(
            linux.as_deref(),
            Some(
                "https://gitlab.com/GrafX2/grafX2/-/jobs/1/artifacts/raw/GrafX2-2.9.3276-x86_64.AppImage"
            )
        );
        assert_eq!(
            windows.as_deref(),
            Some(
                "https://gitlab.com/GrafX2/grafX2/-/jobs/2/artifacts/raw/grafx2-sdl2-2.9.3276-win32.zip"
            )
        );
        assert_eq!(download_links("<html></html>"), (None, None));
    }
}
