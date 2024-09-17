use std::io::{Cursor, Read};

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use directories::ProjectDirs;
use flate2::read::GzDecoder;
use tar::Archive;
use ureq::Response;

use crate::event::EventObserver;
use crate::runner::runner::{ExternRunner, Runner};

pub enum ArchiveFormat {
    Raw,
    TarGz,
    Zip
}

pub struct DelegateApplicationDescription<E: EventObserver> {
    pub download_url: &'static str,
    pub folder: &'static str,
    pub exec_fname: &'static str,
    pub archive_format: ArchiveFormat,
    pub compile: Option<Box<dyn Fn(&Utf8Path, &E) -> Result<(), String>>>
}

pub fn base_cache_foder() -> Utf8PathBuf {
    let proj_dirs = ProjectDirs::from("net.cpcscene", "benediction", "bnd build").unwrap();
    Utf8Path::from_path(proj_dirs.cache_dir())
        .unwrap()
        .to_owned()
}

pub fn clear_base_cache_folder() -> std::io::Result<()> {
    std::fs::remove_dir_all(base_cache_foder())
}

impl<E: EventObserver> DelegateApplicationDescription<E> {
    pub fn is_cached(&self) -> bool {
        self.cache_folder().exists()
    }

    pub fn cache_folder(&self) -> Utf8PathBuf {
        let base_cache = base_cache_foder();

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
                std::fs::create_dir_all(&dest);
                std::fs::write(self.exec_fname(), &buffer).map_err(|e| e.to_string())?;
            },
            ArchiveFormat::TarGz => {
                o.emit_stdout(">> Open archive");
                let gz = GzDecoder::new(input);
                let mut archive = Archive::new(gz);
                archive.unpack(dest.clone()).unwrap();
            },
            ArchiveFormat::Zip => {
                o.emit_stdout(">> Unzip archive");
                let mut buffer = Vec::new();
                input.read_to_end(&mut buffer).unwrap();
                zip_extract::extract(Cursor::new(buffer), dest.as_std_path(), true).unwrap();
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
        }
    }

    fn download(&self, o: &E) -> Result<Response, ureq::Error> {
        o.emit_stdout(&format!(">> Download file {}", self.download_url));
        ureq::get(self.download_url).call()
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

impl<E: EventObserver> Runner for DelegatedRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let cfg = &self.app;

        // ensure the emulator exists
        if !cfg.is_cached() {
            o.emit_stdout("> Install application");
            cfg.install(o);
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
        ExternRunner::<E>::default().inner_run(&command, o)
    }

    fn get_command(&self) -> &str {
        &self.cmd
    }
}
