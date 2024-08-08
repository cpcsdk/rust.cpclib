use std::io::{Cursor, Read};

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::itertools::Itertools;
use directories::ProjectDirs;
use flate2::read::GzDecoder;
use tar::Archive;
use ureq::Response;

use crate::runners::r#extern::ExternRunner;
use crate::runners::Runner;
use crate::task::{ACE_CMDS, CPCEC_CMDS, WINAPE_CMDS};

pub enum ArchiveFormat {
    Raw,
    TarGz,
    Zip
}

pub struct DelegateApplicationDescription {
    pub download_url: &'static str,
    pub folder: &'static str,
    pub exec_fname: &'static str,
    pub archive_format: ArchiveFormat,
    pub compile: Option<Box<dyn Fn(&Utf8Path) -> Result<(), String>>>
}

impl DelegateApplicationDescription {
    pub fn is_cached(&self) -> bool {
        self.cache_folder().exists()
    }

    pub fn cache_folder(&self) -> Utf8PathBuf {
        let proj_dirs = ProjectDirs::from("net.cpcscene", "benediction", "bnd build").unwrap();
        let base_cache = proj_dirs.cache_dir();

        if !base_cache.exists() {
            std::fs::create_dir_all(base_cache);
        }

        base_cache.join(self.folder).try_into().unwrap()
    }

    pub fn exec_fname(&self) -> Utf8PathBuf {
        self.cache_folder().join(self.exec_fname)
    }

    pub fn install(&self) -> Result<(), String> {
        // get the file
        let dest = self.cache_folder();

        println!(">> Download file");
        let resp = self.download().unwrap();
        let mut input = resp.into_reader();

        // uncompress it
        match self.archive_format {
            ArchiveFormat::Raw => {
                println!(">> Save to {}", self.exec_fname());
                let mut buffer = Vec::new();
                input.read_to_end(&mut buffer).unwrap();
                std::fs::create_dir_all(&dest);
                std::fs::write(self.exec_fname(), &buffer).map_err(|e| e.to_string())?;
            },
            ArchiveFormat::TarGz => {
                println!(">> Open archive");
                let gz = GzDecoder::new(input);
                let mut archive = Archive::new(gz);
                archive.unpack(dest.clone()).unwrap();
            },
            ArchiveFormat::Zip => {
                println!(">> Unzip archive");
                let mut buffer = Vec::new();
                input.read_to_end(&mut buffer).unwrap();
                zip_extract::extract(Cursor::new(buffer), dest.as_std_path(), true).unwrap();
            }
        }

        if let Some(compile) = &self.compile {
            println!(">> Compile program");

            let cwd = std::env::current_dir()
                .map_err(|e| format!("Unable to get the current working directory {}.", e))?;
            std::env::set_current_dir(&dest)
                .map_err(|e| format!("Unable to set the current working directory {}.", e))?;
            let res = compile(&dest);
            std::env::set_current_dir(&cwd)
                .map_err(|e| format!("Unable to set the current working directory {}.", e))?;
            res
        }
        else {
            Ok(())
        }
    }

    fn download(&self) -> Result<Response, ureq::Error> {
        ureq::get(self.download_url).call()
    }
}

pub struct DelegatedRunner {
    pub(crate) app: DelegateApplicationDescription,
    pub(crate) cmd: String
}

impl Runner for DelegatedRunner {
    fn inner_run<S: AsRef<str>>(&self, itr: &[S]) -> Result<(), String> {
        let cfg = &self.app;

        // ensure the emulator exists
        if !cfg.is_cached() {
            println!("> Install application");
            cfg.install();
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
        ExternRunner::default().inner_run(&command)
    }

    fn get_command(&self) -> &str {
        &self.cmd
    }
}
