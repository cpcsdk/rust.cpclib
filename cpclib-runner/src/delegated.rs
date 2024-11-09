use std::io::{Cursor, Read};
use std::ops::Deref;

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use directories::ProjectDirs;
use flate2::read::GzDecoder;
use tar::Archive;
use ureq::Response;
use bon::Builder;

use crate::event::EventObserver;
use crate::runner::runner::{ExternRunner, RunInDir, Runner};


use ureq;

pub fn cpclib_download(url: &str) -> Result<String, String> {
	ureq::get(url)
			.set("Cache-Control", "max-age=1")
			.set("From", "krusty.benediction@gmail.com")
			.set("User-Agent", "cpclib")
			.call().map_err(|e| e.to_string())?
			.into_string().map_err(|e| e.to_string())
}


pub struct UrlGenerator(Box<dyn Fn()-> Result<String, String>>);

impl From<Box<dyn Fn()->String>> for UrlGenerator {
    fn from(value: Box<dyn Fn()->String>) -> Self {
        let wrap = Box::new(move || Ok(value()));
        Self(wrap)
    }
}


impl From<Box<dyn Fn()->Result<String, String>>> for UrlGenerator {
    fn from(value: Box<dyn Fn()->Result<String, String>>) -> Self {
        Self(value)
    }
}

impl From<String> for UrlGenerator {
    fn from(value: String) -> Self {
        Self(Box::new(move || Ok(value.clone())))
    }
}

impl From<&str> for UrlGenerator {
    fn from(value: &str) -> Self {
        let value = value.to_owned();
        value.into()
    }
}

impl Deref for UrlGenerator {

    type Target = Box<dyn Fn()->Result<String, String>> ;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Compiler<E>(Box<dyn Fn(&Utf8Path, &E) -> Result<(), String>>);
impl<E> From<Box<dyn Fn(&Utf8Path, &E) -> Result<(), String>>> for Compiler<E> {
    fn from(value: Box<dyn Fn(&Utf8Path, &E) -> Result<(), String>>) -> Self {
        Self(value)
    }
}

impl<E> Deref for Compiler<E> {
    type Target = Box<dyn Fn(&Utf8Path, &E) -> Result<(), String>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


pub struct PostInstall<E: EventObserver>(Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>> );


impl<E: EventObserver> From<Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>>> for PostInstall<E> {
fn from(value: Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>>) -> Self {
    Self(value)
}
}

impl<E: EventObserver> Deref for PostInstall<E> {
    type Target = Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>> ;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


pub enum ArchiveFormat {
    Raw,
    TarGz,
    Zip,
    SevenZ
}

#[derive(Builder)]
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
            ArchiveFormat::TarGz => {
                o.emit_stdout(">> Open targz archive");
                let gz = GzDecoder::new(input);
                let mut archive = Archive::new(gz);
                archive.unpack(dest.clone()).map_err(|e| e.to_string())?;
            },
            ArchiveFormat::Zip => {
                o.emit_stdout(">> Unzip archive");
                let mut buffer = Vec::new();
                input.read_to_end(&mut buffer).unwrap();
                zip_extract::extract(Cursor::new(buffer), dest.as_std_path(), true).map_err(|e| e.to_string())?;
            },
            ArchiveFormat::SevenZ => {
                o.emit_stdout(">> Open 7z archive");
                let mut buffer = Vec::new();
                input.read_to_end(&mut buffer).unwrap();
                sevenz_rust::decompress(Cursor::new(buffer), dest.as_std_path()).map_err(|e| e.to_string())?;
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
            o.emit_stdout(">> Does some post-installation stuffm");
            post_install(self)
        } else {
            Ok(())
        }
    }

    fn download(&self, o: &E) -> Result<Response, String> {
        let url = self.download_fn_url.deref()()?;
        o.emit_stdout(&format!(">> Download file {}", url));
        ureq::get(&url).call()
            .map_err(|e| e.to_string())
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
