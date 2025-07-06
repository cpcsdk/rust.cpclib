use std::fmt::Display;
#[cfg(feature = "cmdline")]
use std::fs::File;
#[cfg(feature = "cmdline")]
use std::io::{Read, Write};

#[cfg(feature = "cmdline")]
use cpclib_common::camino::Utf8Path;
#[cfg(feature = "cmdline")]
use cpclib_common::clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
#[cfg(feature = "cmdline")]
use cpclib_common::parse_value;
#[cfg(feature = "cmdline")]
use cpclib_common::winnow::Parser;
#[cfg(feature = "cmdline")]
use cpclib_common::winnow::error::ContextError;

#[cfg(feature = "cmdline")]
use crate::amsdos::{AmsdosFile, AmsdosFileName, AmsdosFileType, AmsdosHeader};

#[derive(Debug)]
pub enum HideurError {
    IoError(std::io::Error)
}

impl From<std::io::Error> for HideurError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl Display for HideurError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HideurError::IoError(e) => write!(f, "IO error: {e}")
        }
    }
}

#[cfg(feature = "cmdline")]
pub fn hideur_build_arg_parser() -> Command {
    Command::new("hideur")
        .arg(
            Arg::new("INPUT")
                .required(true)
                .help("Input file to manipulate")
        )
        .arg(Arg::new("INFO").long("info").action(ArgAction::SetTrue))
        .arg(
            Arg::new("OUTPUT")
                .short('o')
                .long("output")
                .required_unless_present("INFO")
                .help("Output file to generate")
        )
        .arg(
            Arg::new("USER")
                .short('u')
                .long("user")
                .conflicts_with("INFO")
                .help("User where to put the file")
                .value_parser(value_parser!(u8))
        )
        .arg(
            Arg::new("TYPE")
                .short('t')
                .long("type")
                .conflicts_with("INFO")
                .required_unless_present("INFO")
                .help("File type")
                .ignore_case(true)
                .value_parser([
                    "0",
                    "1",
                    "2",
                    "Basic",
                    "Protected",
                    "Binary",
                    "basic",
                    "protected",
                    "binary",
                    "BASIC",
                    "PROTECTED",
                    "BINARY"
                ])
        )
        .arg(
            Arg::new("EXEC")
                .short('x')
                .long("execution")
                .conflicts_with("INFO")
                .help("Execution address. Default to the load address if not specified.")
        )
        .arg(
            Arg::new("LOAD")
                .short('l')
                .long("load")
                .conflicts_with("INFO")
                .help("Loading address.")
                .required_if_eq_any([
                    ("TYPE", "2"),
                    ("TYPE", "Binary"),
                    ("TYPE", "binary"),
                    ("TYPE", "BINARY")
                ])
        )
}

#[cfg(feature = "cmdline")]
pub fn hideur_handle(matches: &ArgMatches) -> Result<(), HideurError> {
    // Read the input file
    let complete_filename = Utf8Path::new(matches.get_one::<String>("INPUT").unwrap());

    let content = {
        let input = Utf8Path::new(matches.get_one::<String>("INPUT").unwrap());
        let mut f = File::open(input)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        buf
    };

    // Get filename and extension
    let filename = {
        let user = matches.get_one::<u8>("USER").copied().unwrap_or(0);
        let (filename, extension) = {
            let parts = complete_filename
                .file_name()
                .unwrap()
                .split('.')
                .collect::<Vec<_>>();
            let (filename, extension) = match parts.len() {
                1 => (parts[0].to_owned(), String::new()),
                2 => (parts[0].to_owned(), parts[1].to_owned()),
                _n => {
                    eprintln!(
                        "[Warning] Filename contains several `.`. They have been all removed."
                    );
                    (
                        parts[..parts.len() - 1].join("_"),
                        parts[parts.len() - 1].to_owned()
                    )
                }
            };

            let filename = if filename.len() > 8 {
                eprintln!(
                    "[Warning] Filename is too large and has been cropped. If it is not the expected behavior provide a file with the right filename"
                );
                filename[..8].to_owned()
            }
            else {
                filename
            };

            let extension = if extension.len() > 3 {
                eprintln!(
                    "[Warning] Extension is too large and has been cropped. If it is not the expected behavior provide a file with the right extension"
                );
                extension[..3].to_owned()
            }
            else {
                extension
            };

            (filename, extension)
        };

        AmsdosFileName::new_correct_case(user, filename, extension)
            .expect("Invalid file definition")
    };

    if matches.get_flag("INFO") {
        // In this branch we display information about the header
        let amsfile = AmsdosFile::from_buffer(&content);
        match amsfile.header() {
            Some(header) => {
                println!("{header:?}");
            },
            None => {
                eprintln!("This is an ASCII file");
            }
        }
    }
    else {
        // In this branch, we build the file with its header

        // Get the type of file
        let ftype = {
            match matches
                .get_one::<String>("TYPE")
                .unwrap()
                .to_ascii_lowercase()
                .as_ref()
            {
                "0" | "basic" => AmsdosFileType::Basic,
                "1" | "protected" => AmsdosFileType::Protected,
                "2" | "binary" => AmsdosFileType::Binary,
                _ => unreachable!()
            }
        };

        // Build the header according to the given options
        let header = match ftype {
            AmsdosFileType::Binary => {
                let load = matches
                    .get_one::<String>("LOAD")
                    .expect("The load address is expected for a binary target")
                    .as_bytes();
                let load = parse_value::<_, ContextError>
                    .parse(load)
                    .expect("Wrong LOAD format") as u16;

                let exec = if let Some(exec) = matches.get_one::<String>("EXEC") {
                    let exec = exec.as_bytes();
                    parse_value::<_, ContextError>
                        .parse(exec)
                        .expect("Wrong EXEC format") as u16
                }
                else {
                    load
                };

                AmsdosHeader::compute_binary_header(&filename, load, exec, &content)
            },
            AmsdosFileType::Basic => AmsdosHeader::compute_basic_header(&filename, &content),
            _ => unimplemented!()
        };

        // Write the final file
        let mut f = File::create(matches.get_one::<String>("OUTPUT").unwrap())?;
        f.write_all(header.as_bytes())?;
        f.write_all(&content)?;
    }
    Ok(())
}
