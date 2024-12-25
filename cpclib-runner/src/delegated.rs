use std::collections::BTreeMap;
use std::fmt::Display;
use std::io::{Cursor, Read};
use std::ops::Deref;
use std::rc::Rc;

use bon::Builder;
use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::itertools::Itertools;
use directories::ProjectDirs;
use flate2::read::GzDecoder;
use scraper::{Html, Selector};
use tar::Archive;
use ureq;
use ureq::Response;
use xz2::read::XzDecoder;

use crate::event::EventObserver;
use crate::runner::runner::{ExternRunner, RunInDir, Runner};

static GITHUB_URL: &str = "https://github.com/";

/// Download a HTTP ressource
pub fn cpclib_download(url: &str) -> Result<Box<dyn Read + Send + Sync>, String> {
    Ok(ureq::get(url)
        .set("Cache-Control", "max-age=1")
        .set("From", "krusty.benediction@gmail.com")
        .set("User-Agent", "cpclib")
        .call()
        .map_err(|e| e.to_string())?
        .into_reader())
}

/// From the full release url page, get the url for the given release
pub fn github_get_assets_for_version_url<GI: GithubInformation>(
    info: &GI
) -> Result<String, String> {
    let url = dbg!(format!(
        "https://github.com/{}/{}/releases",
        info.owner(),
        info.project()
    ));

    // obtain the base dowload page
    let mut content = cpclib_download(&url)?;
    let mut html = String::new();
    content
        .read_to_string(&mut html)
        .map_err(|e| e.to_string())?;
    let document = Html::parse_document(&html);

    let selector = Selector::parse("a")
        .map_err(|e| e.to_string())
        .map_err(|e| e.to_string())?;

    for link in document.select(&selector) {
        let content = link.inner_html();
        let href = link.attr("href").unwrap();
        if content.contains(info.version_name()) && !href.contains("/tree/") {
            dbg!(&link, &href);
            return dbg!(Ok(
                format!("https://github.com{}", href).replace("/tag/", "/expanded_assets/")
            ));
        }
    }

    Err(format!("No download link found for {info}"))
}

#[derive(Default, bon::Builder)]
#[builder(on(String, into))]
pub struct MutiplatformUrls {
    pub linux: Option<String>,
    pub windows: Option<String>,
    pub macos: Option<String>
}

impl MutiplatformUrls {

    pub fn unique_url(url: &str) -> Self {
        MutiplatformUrls::builder()
            .linux(url)
            .windows(url)
            .macos(url)
            .build()
    }

    pub fn target_os_url(&self) -> Option<&String> {
        #[cfg(target_os = "windows")]
        return self.windows.as_ref();
        #[cfg(target_os = "macos")]
        return self.macosx.as_ref();
        #[cfg(target_os = "linux")]
        return self.linux.as_ref();
    }
}

pub trait CompilableInformation {
    /// Returns the list of commands to execute for the target os
    fn target_os_commands(&self) -> Option<&'static [&'static [&'static str]]>;

    /// Produces the function that executes the list of commands
    fn target_os_compiler<E: EventObserver + 'static>(&self) -> Option<Compiler<E>> {
        if let Some(commands) = self.target_os_commands() {
            let install: Box<dyn Fn(&Utf8Path, &E) -> Result<(), String>> =
                Box::new(|_path: &Utf8Path, o: &E| -> Result<(), String> {
                    for command in commands.iter() {
                        ExternRunner::default().inner_run(command, o)?;
                    }
                    Ok(())
                });
            let install = Compiler::from(install);
            Some(install)
        }
        else {
            None
        }
    }
}

pub trait DownloadableInformation {
    fn target_os_archive_format(&self) -> ArchiveFormat;
    fn target_os_postinstall<E: EventObserver + 'static>(&self) -> Option<PostInstall<E>> {
        None
    }
}

pub trait StaticInformation: DownloadableInformation {
    fn static_download_urls(&self) -> &'static MutiplatformUrls;

    fn target_os_url(&self) -> Option<&'static str> {
        self.static_download_urls()
            .target_os_url()
            .map(|s| s.as_str())
    }

    fn target_os_url_generator(&self) -> UrlGenerator {
        let url = self.target_os_url();
        let deferred: Box<dyn Fn() -> Result<String, String>> = Box::new(move || {
            url.ok_or_else(|| "No download url for current OS".to_string())
                .map(|s| s.to_owned())
        });
        deferred.into()
    }
}

pub trait ExecutableInformation {
    fn target_os_folder(&self) -> &'static str;
    fn target_os_exec_fname(&self) -> &'static str;
    fn target_os_run_in_dir(&self) -> RunInDir {
        RunInDir::default()
    }
}

pub trait DynamicUrlInformation: DownloadableInformation + Clone + 'static {
    fn dynamic_download_urls(&self) -> Result<MutiplatformUrls, String>;

    fn target_os_url_generator(&self) -> UrlGenerator {
        let cloned: Self = self.clone();
        let deferred: Box<dyn Fn() -> Result<String, String>> =
            Box::new(move || -> Result<String, String> {
                let inside: Self = cloned.clone();
                let urls = inside.dynamic_download_urls()?;
                urls.target_os_url()
                    .cloned()
                    .ok_or("No url for this os".to_string())
            });
        deferred.into()
    }
}

pub trait GithubInformation: DownloadableInformation + Display + Clone + 'static {
    fn project(&self) -> &'static str;
    fn owner(&self) -> &'static str;
    /// The name to search to obtain the assets link
    fn version_name(&self) -> &'static str;
    fn linux_key(&self) -> Option<&'static str> {
        None
    }
    fn windows_key(&self) -> Option<&'static str> {
        None
    }
    fn macos_key(&self) -> Option<&'static str> {
        None
    }

    // specific implementation of github
    fn target_os_url_generator(&self) -> UrlGenerator {
        let cloned = self.clone();
        let deferred: Box<dyn Fn() -> Result<String, String>> =
            Box::new(move || -> Result<String, String> {
                let inside = cloned.clone();
                let urls = inside.github_download_urls()?;
                urls.target_os_url()
                    .cloned()
                    .ok_or("No url for this os".to_string())
            });
        deferred.into()
    }

    fn github_download_urls(&self) -> Result<MutiplatformUrls, String> {
        let mut content = cpclib_download(&github_get_assets_for_version_url(self)?)?;
        let mut html = String::default();
        content
            .read_to_string(&mut html)
            .map_err(|e| e.to_string())?;
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

        let mut urls = MutiplatformUrls::default();

        if let Some(key) = self.linux_key() {
            urls.linux = Some(format!("{}/{}", GITHUB_URL, map.get(key).ok_or_else(|| format!("'{}' not found among {}", key, map.keys().map(|s| format!("'{s}'")).join(", ")))?));
        }
        if let Some(key) = self.windows_key() {
            urls.windows = Some(format!("{}/{}", GITHUB_URL, map.get(key).ok_or_else(|| format!("'{}' not found among {}", key, map.keys().map(|s| format!("'{s}'")).join(", ")))?));
        }
        if let Some(key) = self.macos_key() {
            urls.macos = Some(format!("{}/{}", GITHUB_URL, map.get(key).ok_or_else(|| format!("'{}' not found among {}", key, map.keys().map(|s| format!("'{s}'")).join(", ")))?));
        }

        Ok(urls)
    }
}

impl<G> From<&G> for UrlGenerator
where G: GithubInformation
{
    fn from(g: &G) -> Self {
        g.target_os_url_generator()
    }
}

pub trait HasConfiguration {
    fn configuration<E: EventObserver + 'static>(&self) -> DelegateApplicationDescription<E>;
}

pub trait GithubCompilableApplication:
    CompilableInformation + ExecutableInformation + GithubInformation + Default
{
    fn configuration<E: EventObserver + 'static>(&self) -> DelegateApplicationDescription<E> {
        DelegateApplicationDescription::builder()
            .download_fn_url(self) // we assume a modern CPU
            .folder(self.target_os_folder())
            .archive_format(self.target_os_archive_format())
            .exec_fname(self.target_os_exec_fname())
            .maybe_compile(self.target_os_compiler())
            .in_dir(self.target_os_run_in_dir())
            .maybe_post_install(self.target_os_postinstall())
            .build()
    }
}

pub trait GithubCompiledApplication: ExecutableInformation + GithubInformation + Default {
    fn configuration<E: EventObserver + 'static>(&self) -> DelegateApplicationDescription<E> {
        DelegateApplicationDescription::builder()
            .download_fn_url(self) // we assume a modern CPU
            .folder(self.target_os_folder())
            .archive_format(self.target_os_archive_format())
            .exec_fname(self.target_os_exec_fname())
            .in_dir(self.target_os_run_in_dir())
            .maybe_post_install(self.target_os_postinstall())
            .build()
    }
}

pub trait InternetStaticCompiledApplication:
    StaticInformation + ExecutableInformation + Default
{
    fn configuration<E: EventObserver + 'static>(&self) -> DelegateApplicationDescription<E> {
        DelegateApplicationDescription::builder()
            .download_fn_url(self.target_os_url_generator())
            .folder(self.target_os_folder())
            .archive_format(self.target_os_archive_format())
            .exec_fname(self.target_os_exec_fname())
            .in_dir(self.target_os_run_in_dir())
            .maybe_post_install(self.target_os_postinstall())
            .build()
    }
}

pub trait InternetDynamicCompiledApplication:
    DynamicUrlInformation + ExecutableInformation + Default
{
    fn configuration<E: EventObserver + 'static>(&self) -> DelegateApplicationDescription<E> {
        DelegateApplicationDescription::builder()
            .download_fn_url(self.target_os_url_generator())
            .folder(self.target_os_folder())
            .archive_format(self.target_os_archive_format())
            .exec_fname(self.target_os_exec_fname())
            .in_dir(self.target_os_run_in_dir())
            .maybe_post_install(self.target_os_postinstall())
            .build()
    }
}

#[derive(Clone)]
pub struct UrlGenerator(Rc<Box<dyn Fn() -> Result<String, String>>>);

impl From<Box<dyn Fn() -> String>> for UrlGenerator {
    fn from(value: Box<dyn Fn() -> String>) -> Self {
        let wrap = Box::new(move || Ok(value()));
        Self(Rc::new(wrap))
    }
}

impl From<Box<dyn Fn() -> Result<String, String>>> for UrlGenerator {
    fn from(value: Box<dyn Fn() -> Result<String, String>>) -> Self {
        Self(Rc::new(value))
    }
}

impl From<String> for UrlGenerator {
    fn from(value: String) -> Self {
        Self(Rc::new(Box::new(move || Ok(value.clone()))))
    }
}

impl From<&str> for UrlGenerator {
    fn from(value: &str) -> Self {
        let value = value.to_owned();
        value.into()
    }
}

impl Deref for UrlGenerator {
    type Target = Box<dyn Fn() -> Result<String, String>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct Compiler<E>(Rc<Box<dyn Fn(&Utf8Path, &E) -> Result<(), String>>>);
impl<E> From<Box<dyn Fn(&Utf8Path, &E) -> Result<(), String>>> for Compiler<E> {
    fn from(value: Box<dyn Fn(&Utf8Path, &E) -> Result<(), String>>) -> Self {
        Self(Rc::new(value))
    }
}

impl<E> Deref for Compiler<E> {
    type Target = Box<dyn Fn(&Utf8Path, &E) -> Result<(), String>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct PostInstall<E: EventObserver>(
    Rc<Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>>>
);

impl<E: EventObserver> From<Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>>>
    for PostInstall<E>
{
    fn from(value: Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>>) -> Self {
        Self(Rc::new(value))
    }
}

impl<E: EventObserver> Deref for PostInstall<E> {
    type Target = Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub enum ArchiveFormat {
    Raw,
    Tar,
    TarGz,
    TarXz,
    Zip,
    SevenZ
}

#[derive(Builder, Clone)]
pub struct DelegateApplicationDescription<E: EventObserver> {
    #[builder(into)]
    pub download_fn_url: UrlGenerator,
    pub folder: &'static str,
    pub exec_fname: &'static str,
    pub archive_format: ArchiveFormat,
    #[builder(into)]
    pub compile: Option<Compiler<E>>,
    #[builder(into)]
    pub post_install: Option<PostInstall<E>>,
    #[builder(default=RunInDir::CurrentDir)]
    pub in_dir: RunInDir
}

pub fn base_cache_folder() -> Utf8PathBuf {
    let proj_dirs = ProjectDirs::from("net.cpcscene", "benediction", "bnd build").unwrap();
    Utf8Path::from_path(proj_dirs.cache_dir())
        .unwrap()
        .to_owned()
}

pub fn clear_base_cache_folder() -> std::io::Result<()> {
    std::fs::remove_dir_all(base_cache_folder())
}

impl<E: EventObserver> DelegateApplicationDescription<E> {
    pub fn is_cached(&self) -> bool {
        self.cache_folder().exists()
    }

    pub fn cache_folder(&self) -> Utf8PathBuf {
        let base_cache = base_cache_folder();

        if !base_cache.exists() {
            std::fs::create_dir_all(&base_cache).unwrap();
        }

        base_cache.join(self.folder).try_into().unwrap()
    }

    pub fn exec_fname(&self) -> Utf8PathBuf {
        self.cache_folder().join(self.exec_fname)
    }

    pub fn install(&self, o: &E) -> Result<(), String> {
        self.inner_install(o).inspect_err(|e| {
            let dest = self.cache_folder();
            let _ = std::fs::remove_dir_all(dest); // ignore error
        })
    }

    fn inner_install(&self, o: &E) -> Result<(), String> {
        // get the file
        let dest = self.cache_folder();

        let resp = self
            .download(o)
            .map_err(|e| format!("Unable to download the expected file. {}", e))?;
        let mut input = resp.into_reader();

        // uncompress it
        match self.archive_format {
            ArchiveFormat::Raw => {
                o.emit_stdout(&format!(">> Save to {}", self.exec_fname()));
                let mut buffer = Vec::new();
                input.read_to_end(&mut buffer).unwrap();
                std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
                std::fs::write(self.exec_fname(), &buffer).map_err(|e| e.to_string())?;
            },
            ArchiveFormat::Tar => {
                o.emit_stdout(">> Open tar archive");
                let mut archive = Archive::new(input);
                archive.unpack(dest.clone()).map_err(|e| e.to_string())?;
            },
            ArchiveFormat::TarGz => {
                o.emit_stdout(">> Open targz archive");
                let gz = GzDecoder::new(input);
                let mut archive = Archive::new(gz);
                archive.unpack(dest.clone()).map_err(|e| e.to_string())?;
            },
            ArchiveFormat::TarXz => {
                o.emit_stdout(">> Open tarxz archive");
                let xz = XzDecoder::new(input);
                let mut archive = Archive::new(xz);
                archive.unpack(dest.clone()).map_err(|e| e.to_string())?;
            },
            ArchiveFormat::Zip => {
                o.emit_stdout(">> Unzip archive");
                let mut buffer = Vec::new();
                input.read_to_end(&mut buffer).unwrap();
                zip_extract::extract(Cursor::new(buffer), dest.as_std_path(), true)
                    .map_err(|e| e.to_string())?;
            },
            ArchiveFormat::SevenZ => {
                o.emit_stdout(">> Open 7z archive");
                let mut buffer = Vec::new();
                input.read_to_end(&mut buffer).unwrap();
                sevenz_rust::decompress(Cursor::new(buffer), dest.as_std_path())
                    .map_err(|e| e.to_string())?;
            }
        }

        if let Some(compile) = &self.compile {
            o.emit_stdout(">> Compile program");

            let cwd = std::env::current_dir()
                .map_err(|e| format!("Unable to get the current working directory {}.", e))?;
            std::env::set_current_dir(&dest)
                .map_err(|e| format!("Unable to set the current working directory {}.", e))?;
            let res = compile(&dest, o);
            std::env::set_current_dir(&cwd)
                .map_err(|e| format!("Unable to set the current working directory {}.", e))?;
            res
        }
        else {
            Ok(())
        }?;

        if let Some(post_install) = &self.post_install {
            o.emit_stdout(">> Apply post-installation");
            post_install(self)
        }
        else {
            Ok(())
        }
    }

    fn download(&self, o: &E) -> Result<Response, String> {
        let url = self.download_fn_url.deref()()?;
        o.emit_stdout(&format!(">> Download file {}", url));
        ureq::get(&url).call().map_err(|e| e.to_string())
    }
}

pub struct DelegatedRunner<E: EventObserver> {
    pub app: DelegateApplicationDescription<E>,
    pub cmd: String
}

impl<E: EventObserver> DelegatedRunner<E> {
    pub fn new(app: DelegateApplicationDescription<E>, cmd: String) -> Self {
        Self { app, cmd }
    }
}

impl<E: EventObserver + 'static> Runner for DelegatedRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let cfg = &self.app;

        // ensure the emulator exists
        if !cfg.is_cached() {
            o.emit_stdout("> Install application");
            cfg.install(o)?;
        }
        assert!(cfg.is_cached());

        // Build the command
        let mut command = Vec::with_capacity(1 + itr.len());
        let fname = cfg.exec_fname();

        #[cfg(target_os = "linux")]
        {
            if fname.as_str().to_lowercase().ends_with(".exe") {
                command.push("wine");
            }
        }

        command.push(fname.as_str());

        for arg in itr.iter() {
            command.push(arg.as_ref());
        }

        // Delegate it to the appropriate luncher
        ExternRunner::<E>::new(cfg.in_dir).inner_run(&command, o)
    }

    fn get_command(&self) -> &str {
        &self.cmd
    }
}
