

extern crate path_absolutize;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::fs;
use std::path::Path;
use curl::easy::{Easy, Form};
use std::sync::Arc;
use std::borrow::BorrowMut;
use std::borrow::Borrow;

use path_absolutize::*;

#[derive(Debug)]
pub struct M4File {
    fname: String,
    unknown: String,
    size: String
}

impl From<&str> for M4File {
    fn from(line: &str) -> M4File {
        let mut splitted = line.split(',');
        M4File {
            fname: splitted.next().unwrap().into(),
            unknown: splitted.next().unwrap().into(),
            size: splitted.next().unwrap().into()
        }
    }
}

pub struct M4FilesList {
    cwd: String,
    files: Vec<M4File>
}

impl From<&str> for M4FilesList {
    fn from(buffer: &str) -> M4FilesList {
        let mut iter = buffer.lines();
        let mut path = iter.next().unwrap();
        if path == "//" {
            path  = "/";
        }
        let files = iter.map(|s|{s.into()}).collect::<Vec<M4File>>();
        M4FilesList {
            cwd: path.into(),
            files: files
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




pub struct CpcXfer {
    hostname: String,
}


pub enum AmsdosFileType {
    Basic,
    Protected,
    Binary
}

impl AmsdosFileType {
    pub fn code(&self) -> u8 {
        match self {
            &AmsdosFileType::Basic => 0,
            &AmsdosFileType::Protected=> 1,
            &AmsdosFileType::Binary => 2,
        }
    }
}



impl CpcXfer {
    pub fn new(hostname: &str) -> CpcXfer {
        CpcXfer {
            hostname: String::from(hostname)
        }

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
    pub fn reset_m4(&self) {
        self.simple_query(&[("mres","")])
            .expect("Unable to reset the M4");
    }

    /// Reset the Cpc
    pub fn reset_cpc(&self) {
        self.simple_query(&[("cres","")])
            .expect("Unable to reset the M4");
    }

    /// Run the file from the current selected path
    /// TODO debug this
   pub fn run_rom_current_path(&self, fname: &str) {
       self.simple_query(&[("run", fname)])
        .expect("Unable to run the given file");
   }

    /// Run the file whose complete path is provided
    pub fn run(&self, path: &str)  {
       let absolute = self.absolute_path(path).unwrap();
       self.simple_query(&[("run2", &absolute)])
        .expect("Unable to run the given file");
    }


    /// Remove the file whose complete path is provided
    pub fn rm(&self, path: &str) {
       self.simple_query(&[("rm", path)])
        .expect("Unable to delete the given file");
    }

    /// upload a file on the M4
    pub fn upload<P>(&self, path: P, m4_path: &str, header: Option<(AmsdosFileType, u16, u16)>)
	where P: AsRef<Path> {
		self.upload_impl(path.as_ref(), m4_path, header);
	}

    pub fn upload_impl(&self, path: &Path, m4_path: &str, header: Option< (AmsdosFileType, u16, u16)>) {

		let local_fname = path.to_str().unwrap();

        if m4_path.len() >255 {
            panic!("{} path is too long (should be limited to 255 chars)", m4_path);
        }
        let file_contents = fs::read(local_fname).expect("Unable to read PC file");


        let local_fname = match header {
            Some(header) => {
                unimplemented!();
                // Need to build header and compute checksum
                // Need to inject header
            },
            None => {
                // Header is already included within the file
                // TODO check that the header is correct
                local_fname
            }
        };

        // TODO manage more cases in order to allow to provide a destination folder or a destination filename or a different name
        let destination = Path::new(m4_path).join(
            Path::new(local_fname).file_name().expect("Unable to retreive the filename of the file to upload")
        );
        let destination = destination.to_str().unwrap().to_owned();

        println!("Destination : {:?}", destination);

        let mut form = Form::new();
        form.part("upfile")
            .file(local_fname)
            .filename(&destination)
            .add().unwrap();
        let mut easy = Easy::new();
        easy.url(&self.uri("files.shtml")).unwrap();
        easy.httppost(form).unwrap();
        easy.perform().unwrap();
    }


	pub fn upload_and_run<P>(&self, path: P, header: Option<(AmsdosFileType, u16, u16)>)
		where P:AsRef<Path> {
			self.upload_and_run_impl(path.as_ref(), header);
	}

	fn upload_and_run_impl(&self, path: &Path, header: Option<(AmsdosFileType, u16, u16)>) {

			self.upload_impl(path, "/tmp", header);
			self.run(&format!("/tmp/{}", path.file_name().unwrap().to_str().unwrap()));
	}


    pub fn current_folder_content(&self) -> M4FilesList {
        self.download_dir()
    }

    pub fn current_working_directory(&self) -> String {
        let data = self.download_dir();
        data.cwd().clone()
    }



    fn download_dir(&self) -> M4FilesList{
        let mut dst = Vec::new();
        {
            {
                let mut easy = Easy::new();
                easy.url(&self.uri("sd/m4/dir.txt")).unwrap();
                let mut easy = easy.transfer();
                easy.write_function(|data| {
                    dst.extend_from_slice(data);
                    Ok(data.len())
                }).unwrap();
                easy.perform().unwrap();
            }
        }

        let content = std::str::from_utf8(&dst).expect("Unable to create an UTF8 string for M4 content");

        M4FilesList::from(content)

    }


    /// Change the current directory
    pub fn cd(&self, directory: &str) -> Result<(), String> {

        // Get the absolute directory
        let mut directory = if let Some('/') = directory.chars().next() {
            directory.to_owned()
        }
        else {
            self.absolute_path(directory)?
        };



        self.ls_request(&directory);

        // Ensure theire is a / at the end
        if  directory.chars().rev().next().unwrap() != '/' {
            directory.push('/');
        }
        let cwd = self.current_working_directory();

        if cwd == directory {
            Ok(())
        }
        else {
            Err(format!("[ERROR] Unable to move in {}. Current working directory is {}", directory, cwd))
        }



    }



    fn absolute_path(&self, relative: &str) -> Result<String, String> {
        match relative.chars().next() {
            None => Err("No path provided".into()),
            Some('/') => Ok(relative.to_owned()),
            _ => {
                let cwd = self.current_working_directory();
                let absolute = Path::new(&cwd).join(relative);

                let absolute = absolute.absolutize().unwrap();
                Ok(absolute.to_str().unwrap().into())
            }
        }

    }


    fn ls_request(&self, folder: &str) {
        let mut easy = Easy::new();
        let folder = easy.url_encode(folder.as_bytes());
        easy.get(true).unwrap();
        let url = format!(
            "{}?ls={}",
            self.uri("config.cgi"),
            folder
            );
        easy.url(&url).unwrap();
        easy.perform().unwrap();
    }

}


