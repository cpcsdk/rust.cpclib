extern crate path_absolutize;

use curl::easy::{Easy, Form};
use curl::Error;
use path_absolutize::*;

use std::fs;

use std::io::prelude::*;
use std::path::Path;

extern crate custom_error;
use custom_error::custom_error;

custom_error! {pub XferError
    ConnectionError{source: Error} = "There is a connection error with the Cpc Wifi.",
    ConnectionError2{source: reqwest::Error} = "There is a connection error with the Cpc Wifi.",

    CdError{from: String, to: String} = @ {
        format!(
            "Unable to move in {}. Current working directory is {}.",
            from, to)
    },
    InternalError{reason: String} = @ {
        format!("Internal error: {}", reason)
    }
}

#[derive(Debug)]
pub struct M4File {
    fname: String,
    unknown: String,
    size: String,
}

impl From<&str> for M4File {
    fn from(line: &str) -> M4File {
        let mut splitted = line.split(',');
        M4File {
            fname: splitted.next().unwrap().into(),
            unknown: splitted.next().unwrap().into(),
            size: splitted.next().unwrap().into(),
        }
    }
}

pub struct M4FilesList {
    cwd: String,
    files: Vec<M4File>,
}

impl From<&str> for M4FilesList {
    fn from(buffer: &str) -> M4FilesList {
        let mut iter = buffer.lines();
        let mut path = iter.next().unwrap();
        if path == "//" {
            path = "/";
        }
        let files = iter.map(|s| s.into()).collect::<Vec<M4File>>();
        M4FilesList {
            cwd: path.into(),
            files: files,
        }
    }
}

impl M4FilesList {
    pub fn cwd(&self) -> &String {
        &self.cwd
    }

    pub fn nb_files(&self) -> usize {
        self.files.len()
    }

    // TODO do not give acces to the vector
    pub fn files(&self) -> &Vec<M4File> {
        &self.files
    }
}

pub enum AmsdosFileType {
    Basic,
    Protected,
    Binary,
}

impl AmsdosFileType {
    pub fn code(&self) -> u8 {
        match self {
            &AmsdosFileType::Basic => 0,
            &AmsdosFileType::Protected => 1,
            &AmsdosFileType::Binary => 2,
        }
    }
}

/// Bridget the the CPC Wifi card
pub struct CpcXfer {
    hostname: String,
}

impl CpcXfer {
    pub fn new(hostname: &str) -> CpcXfer {
        CpcXfer {
            hostname: String::from(hostname),
        }
    }

    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    /// Return the appropriate uri
    fn uri(&self, path: &str) -> String {
        format!("http://{}/{}", self.hostname, path)
    }

    /// Make a simple query
    fn simple_query(&self, query: &[(&str, &str)]) -> reqwest::Result<reqwest::Response> {
        reqwest::Client::new()
            .get(&self.uri("config.cgi"))
            .query(query)
            .header("User-Agent", "User-Agent: cpcxfer")
            .send()
    }

    /// Reset the M4
    pub fn reset_m4(&self) -> Result<(), XferError> {
        self.simple_query(&[("mres", "")])?;
        Ok(())
    }

    /// Reset the Cpc
    pub fn reset_cpc(&self) -> Result<(), XferError> {
        self.simple_query(&[("cres", "")])?;
        Ok(())
    }

    /// Run the file from the current selected path
    /// TODO debug this
    pub fn run_rom_current_path(&self, fname: &str) -> Result<(), XferError> {
        self.simple_query(&[("run", fname)])?;
        Ok(())
    }

    /// Run the file whose complete path is provided
    pub fn run(&self, path: &str) -> Result<(), XferError> {
        let absolute = self.absolute_path(path)?;
        self.simple_query(&[("run2", &absolute)])?;
        Ok(())
    }

    /// Remove the file whose complete path is provided
    pub fn rm(&self, path: &str) -> Result<(), XferError> {
        self.simple_query(&[("rm", path)])?;
        Ok(())
    }

    /// upload a file on the M4
    pub fn upload<P>(
        &self,
        path: P,
        m4_path: &str,
        header: Option<(AmsdosFileType, u16, u16)>,
    ) -> Result<(), XferError>
    where
        P: AsRef<Path>,
    {
        self.upload_impl(path.as_ref(), m4_path, header)
    }

    pub fn upload_impl(
        &self,
        path: &Path,
        m4_path: &str,
        header: Option<(AmsdosFileType, u16, u16)>,
    ) -> Result<(), XferError> {
        let local_fname = path.to_str().unwrap();

        if m4_path.len() > 255 {
            panic!(
                "{} path is too long (should be limited to 255 chars)",
                m4_path
            );
        }
        let _file_contents = fs::read(local_fname).expect("Unable to read PC file");

        let local_fname = match header {
            Some(_header) => {
                unimplemented!();
                // Need to build header and compute checksum
                // Need to inject header
            }
            None => {
                // Header is already included within the file
                // TODO check that the header is correct
                local_fname
            }
        };

        // TODO manage more cases in order to allow to provide a destination folder or a destination filename or a different name
        let destination = Path::new(m4_path).join(
            Path::new(local_fname)
                .file_name()
                .expect("Unable to retreive the filename of the file to upload"),
        );
        let destination = destination.to_str().unwrap().to_owned();

        println!("Destination : {:?}", destination);

        let mut form = Form::new();
        form.part("upfile")
            .file(local_fname)
            .filename(&destination)
            .add()
            .unwrap();
        let mut easy = Easy::new();
        easy.url(&self.uri("files.shtml"))?;
        easy.httppost(form)?;
        easy.perform()?;

        Ok(())
    }

    /// Directly sends the SNA to the M4. SNA is first saved as a V2 version as M4 is unable to read other ones
    pub fn upload_and_run_sna(&self, sna: &crate::sna::Snapshot) -> Result<(), XferError> {
        let file = tempfile::NamedTempFile::new().expect("Unable to build a temporary file");
        let path = file.into_temp_path();
        let path = path.to_str().unwrap();
        sna.save(path, crate::sna::SnapshotVersion::V2)
            .expect("Unable to save the snapshot");
        self.upload_and_run(path, None)?;

        // sleep a bit to be sure the file is not deleted
        std::thread::sleep(std::time::Duration::from_secs(5));
        Ok(())
    }

    pub fn upload_and_run<P: AsRef<Path>>(
        &self,
        path: P,
        header: Option<(AmsdosFileType, u16, u16)>,
    ) -> Result<(), XferError> {
        self.upload_and_run_impl(path.as_ref(), header)
    }

    fn upload_and_run_impl(
        &self,
        path: &Path,
        header: Option<(AmsdosFileType, u16, u16)>,
    ) -> Result<(), XferError> {
        // We are sure it is not a snapshot there
        self.upload_impl(path, "/tmp", header)?;
        self.run(&format!(
            "/tmp/{}",
            path.file_name().unwrap().to_str().unwrap()
        ))?;
        Ok(())
    }

    pub fn current_folder_content(&self) -> Result<M4FilesList, XferError> {
        self.download_dir()
    }

    pub fn current_working_directory(&self) -> Result<String, XferError> {
        let data = self.download_dir()?;
        Ok(data.cwd().clone())
    }

    fn download_dir(&self) -> Result<M4FilesList, XferError> {
        let mut dst = Vec::new();
        {
            {
                let mut easy = Easy::new();
                easy.url(&self.uri("sd/m4/dir.txt"))?;
                let mut easy = easy.transfer();
                easy.write_function(|data| {
                    dst.extend_from_slice(data);
                    Ok(data.len())
                })?;
                easy.perform()?;
            }
        }

        let content =
            std::str::from_utf8(&dst).expect("Unable to create an UTF8 string for M4 content");

        Ok(M4FilesList::from(content))
    }

    /// Change the current directory
    pub fn cd(&self, directory: &str) -> Result<(), XferError> {
        // Get the absolute directory
        let mut directory = if let Some('/') = directory.chars().next() {
            directory.to_owned()
        } else {
            self.absolute_path(directory)?
        };

        self.ls_request(&directory);

        // Ensure theire is a / at the end
        if directory.chars().rev().next().unwrap() != '/' {
            directory.push('/');
        }
        let cwd = self.current_working_directory()?;

        if cwd == directory {
            Ok(())
        } else {
            Err(XferError::CdError {
                from: directory,
                to: cwd,
            })
        }
    }

    fn absolute_path(&self, relative: &str) -> Result<String, XferError> {
        match relative.chars().next() {
            None => Err(XferError::InternalError {
                reason: "No path provided".into(),
            }),
            Some('/') => Ok(relative.to_owned()),
            _ => {
                let cwd = self.current_working_directory()?;
                let absolute = Path::new(&cwd).join(relative);

                let absolute = absolute.absolutize().unwrap();
                Ok(absolute.to_str().unwrap().into())
            }
        }
    }

    fn ls_request(&self, folder: &str) -> Result<(), XferError> {
        let mut easy = Easy::new();
        let folder = easy.url_encode(folder.as_bytes());
        easy.get(true)?;
        let url = format!("{}?ls={}", self.uri("config.cgi"), folder);
        easy.url(&url)?;
        easy.perform()?;
        Ok(())
    }
}
