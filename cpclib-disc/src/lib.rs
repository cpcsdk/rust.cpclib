#![feature(impl_trait_in_bindings)]

use std::collections::VecDeque;
#[cfg(feature = "cmdline")]
use std::fs::File;
#[cfg(feature = "cmdline")]
use std::io::{Read, Write};
#[cfg(feature = "cmdline")]
use std::str::FromStr;

use amsdos::AmsdosError;
use cpclib_common::camino::Utf8Path;
#[cfg(feature = "cmdline")]
use cpclib_common::camino::Utf8PathBuf;
#[cfg(feature = "cmdline")]
use cpclib_common::clap;
#[cfg(feature = "cmdline")]
use cpclib_common::clap::*;
use disc::Disc;

#[cfg(feature = "cmdline")]
use crate::amsdos::*;
use crate::edsk::Head;

/// Concerns all stuff related to Amsdos disc format
pub mod amsdos;
/// Utility function to build a DSK thanks to a format description
pub mod builder;
/// Parser of the format description
pub mod cfg;
pub mod disc;
/// EDSK File format
pub mod edsk;
pub mod hideur;

/// HFE File format
#[cfg(feature = "hfe")]
pub mod hfe;

use custom_error::custom_error;

use crate::amsdos::AmsdosHeader;
use crate::edsk::ExtendedDsk;
#[cfg(feature = "hfe")]
use crate::hfe::Hfe;

pub enum AnyDisc {
    Edsk(ExtendedDsk),
    #[cfg(feature = "hfe")]
    Hfe(Hfe)
}

impl From<ExtendedDsk> for AnyDisc {
    fn from(value: ExtendedDsk) -> Self {
        Self::Edsk(value)
    }
}

#[cfg(feature = "hfe")]
impl From<Hfe> for AnyDisc {
    fn from(value: Hfe) -> Self {
        Self::Hfe(value)
    }
}

impl Default for AnyDisc {
    #[cfg(feature = "hfe")]
    fn default() -> Self {
        Hfe::default().into()
    }

    #[cfg(not(feature = "hfe"))]
    fn default() -> Self {
        ExtendedDsk::default().into()
    }
}

impl Disc for AnyDisc {
    delegate::delegate! {
        to match self {
            AnyDisc::Edsk(disc) => disc,
            #[cfg(feature="hfe")]
            AnyDisc::Hfe(disc) => disc,
        } {
            fn next_position(&self, head: u8, track: u8, sector: u8) -> Option<(u8, u8, u8)>;
            fn save<P>(&self, path: P) -> Result<(), String> where P: AsRef<Utf8Path> ;
            fn global_min_sector<S: Into<Head>>(&self, side: S) -> u8;
            fn track_min_sector<S: Into<Head>>(&self, side: S, track: u8) -> u8;
            fn nb_tracks_per_head(&self) -> u8;
            fn sector_read_bytes<S: Into<Head>>(
                &self,
                head: S,
                track: u8,
                sector_id: u8
            ) -> Option<Vec<u8>>;
            fn sector_write_bytes<S: Into<Head>>(
                &mut self,
                head: S,
                track: u8,
                sector_id: u8,
                bytes: &[u8]
            ) -> Result<(), String>;
        }
    }

    #[cfg(feature = "hfe")]
    fn open<P>(path: P) -> Result<Self, String>
    where
        Self: Sized,
        P: AsRef<Utf8Path>
    {
        Hfe::open(path).map(|d| d.into())
    }

    #[cfg(not(feature = "hfe"))]
    fn open<P>(path: P) -> Result<Self, String>
    where
        Self: Sized,
        P: AsRef<Utf8Path>
    {
        ExtendedDsk::open(path).map(|d| d.into())
    }
}

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

custom_error! {pub DskManagerError
    IOError{source: std::io::Error} = "IO error: {source}.",
    AnyError{msg: String} = "{msg}",
    DiscConfigError{source: crate::cfg::DiscConfigError} = "Disc configuration: {source}",
}

impl From<AmsdosError> for DskManagerError {
    fn from(value: AmsdosError) -> Self {
        DskManagerError::AnyError {
            msg: format!("Amsdos error: {value}")
        }
    }
}

#[inline]
pub fn new_disc<P: AsRef<Utf8Path>>(path: Option<P>) -> AnyDisc {
    AnyDisc::default()
}

#[inline]
pub fn open_disc<P: AsRef<Utf8Path>>(path: P, fail_if_missing: bool) -> Result<AnyDisc, String> {
    let path = path.as_ref();
    if !path.exists() {
        if fail_if_missing {
            return Err(format!("{} does not exists", path));
        }
        else {
            return Ok(new_disc(Some(path)));
        }
    }

    AnyDisc::open(path).map_err(|e| format!("Error while loading {e}"))
}

#[cfg(feature = "cmdline")]
pub fn dsk_manager_handle(matches: &ArgMatches) -> Result<(), DskManagerError> {
    use cpclib_common::camino::Utf8Path;

    let dsk_fname = matches.get_one::<String>("DSK_FILE").unwrap();
    let behavior = amsdos::AmsdosAddBehavior::ReplaceIfPresent;

    // Manipulate the catalog of a disc
    if let Some(sub) = matches.subcommand_matches("catalog") {
        let mut dsk = open_disc(dsk_fname, true)
            .unwrap_or_else(|_| panic!("Unable to open the file {dsk_fname}"));
        eprintln!("WIP - We assume head 0 is chosen");

        // Import the catalog from one file in one existing disc
        if let Some(fname) = sub.get_one::<String>("IMPORT") {
            let mut f = File::open(fname)?;
            let mut bytes = Vec::new();
            let size = f.read_to_end(&mut bytes)?;

            if size != 64 * 32 {
                eprintln!(
                    "Catalog size uses {} bytes whereas it should be {}",
                    size,
                    64 * 32
                );
            }

            for idx in 0..4 {
                let mut sector = dsk.sector_mut(0, 0, idx + 0xc1).expect("Wrong format");
                let idx = idx as usize;
                sector
                    .set_values(&bytes[idx * 512..(idx + 1) * 512])
                    .unwrap();
            }

            dsk.save(dsk_fname)
            .map_err(|e| DskManagerError::AnyError { msg: e })?;

        /*
            // TODO find why this method DOES NOT WORK
                // Generate the entry for this catart
               let  entries = AmsdosEntries::from_slice(&bytes);

               // And inject it in the disc
               let mut manager = AmsdosManager::new_from_disc(dsk, 0);
               manager.set_catalog(&entries);

               let copy = manager.catalog();
               assert_eq!(
                   copy,
                   entries
               );
               manager.dsk_mut().save(dsk_fname)?;
        */
        // override the disc
        }
        // Export the catalog of an existing disc in a file
        else if let Some(fname) = sub.get_one::<String>("EXPORT") {
            eprintln!("WIP - We assume the format of the Track 0 is similar to Amsdos one");

            let manager = AmsdosManagerNonMut::new_from_disc(&mut dsk, 0);
            let bytes = manager.catalog().as_bytes();
            let mut f = File::create(fname)?;
            f.write_all(&bytes)?;
        } else if sub.contains_id("LIST") {
            let manager = AmsdosManagerNonMut::new_from_disc(&mut dsk, 0);
            let catalog = manager.catalog();
            let entries = catalog.visible_entries().collect::<Vec<_>>();
            // TODO manage files instead of entries
            println!("Dsk {} -- {} files", dsk_fname, entries.len());
            for entry in &entries {
                println!("{entry}");
            }
        } else {
            panic!("Error - missing argument");
        }
    }
    else if let Some(sub) = matches.subcommand_matches("put") {
        use cpclib_tokens::{Listing, builder};

        // Add files in a sectorial way
        let mut track =
            u8::from_str(sub.get_one::<String>("TRACK").unwrap()).expect("Wrong track format");
        let mut sector =
            u8::from_str(sub.get_one::<String>("SECTOR").unwrap()).expect("Wrong track format");
        let mut head =
            u8::from_str(sub.get_one::<String>("SIDE").unwrap()).expect("Wrong track format");
        let _export = sub.get_one::<String>("Z80_EXPORT").unwrap();

        let mut dsk = open_disc(dsk_fname, true)
            .unwrap_or_else(|_| panic!("Unable to open the file {dsk_fname}"));

        let mut listing = Listing::new();
        for file in sub.get_many::<String>("FILES").unwrap() {
            // get the file
            let mut f = File::open(file)?;
            let mut content = Vec::new();
            f.read_to_end(&mut content)?;

            let next_position = dsk
                .add_file_sequentially(head, track, sector, &content)
                .unwrap_or_else(|_| panic!("Unable to add {file}"));

            let base_label = Utf8Path::new(file).file_name().unwrap().replace('.', "_");
            listing.add(builder::equ(format!("{}_head", &base_label), head));
            listing.add(builder::equ(format!("{}_track", &base_label), track));
            listing.add(builder::equ(format!("{}_sector", &base_label), sector));

            head = next_position.0;
            track = next_position.1;
            sector = next_position.2;
        }
    }
    else if let Some(sub) = matches.subcommand_matches("get") {
        let disc = open_disc(dsk_fname, true)
            .unwrap_or_else(|_| panic!("Unable to open the file {dsk_fname}"));
        let head = Head::A;

        for filename in sub.get_many::<String>("OUTPUT_FILES").unwrap() {
            let ams_filename = AmsdosFileName::try_from(filename)?;
            let file = disc.get_amsdos_file(head, ams_filename)?;

            if file.is_none() {
                return Err(DskManagerError::AnyError {
                    msg: format!("missing {filename}")
                });
            }
            else {
                let file = file.unwrap();
                if sub.get_flag("noheader") {
                    std::fs::write(filename, file.content())?;
                }
                else {
                    std::fs::write(filename, file.header_and_content())?;
                }
            }
        }
    }
    else if let Some(sub) = matches.subcommand_matches("add") {
        // Add files in an Amsdos compatible disc

        // Get the input dsk
        let mut disc = open_disc(dsk_fname, true)
            .unwrap_or_else(|_| panic!("Unable to open the file {dsk_fname}"));

        // Get the common parameters
        let is_system = sub.get_flag("SYSTEM");
        let is_read_only = sub.get_flag("READ_ONLY");
        let head = Head::A;

        // loop over all the files to add them
        for fname in sub.get_many::<Utf8PathBuf>("INPUT_FILES").unwrap() {
            if sub.get_flag("ASCII") {
                let (ams_file, ams_filename) =
                    AmsdosFile::open_valid_ascii(fname).expect("Error when reading the ascii file");
                disc.add_ascii_file(
                    &ams_file,
                    dbg!(&ams_filename),
                    head,
                    is_system,
                    is_read_only,
                    behavior
                )
                .unwrap();
            }
            else {
                let ams_file = match AmsdosFile::open_valid(fname) {
                    Ok(mut ams_file) => {
                        let amsdos_fname = ams_file.amsdos_filename().expect("There is a bug here");

                        if amsdos_fname.is_err() || !amsdos_fname.unwrap().is_valid() {
                            // the amsdos header is crappy and does not handle properly the name. Probably because it comes from orgams ;)
                            // then we try to replace it by the file name
                            eprintln!("AMSDOS filename is invalid. We try to use the PC filename");

                            let pc_fname = fname.file_name().unwrap().to_ascii_uppercase();
                            let mut pc_fname = pc_fname.split(".");
                            let mut header = ams_file.header().expect("Need to handle ASCII files");
                            let new_amsdos_fname = AmsdosFileName::new_correct_case(
                                0,
                                pc_fname.next().unwrap(),
                                pc_fname.next().unwrap_or_default()
                            )?;
                            assert!(pc_fname.next().is_none());

                            header.set_amsdos_filename(&new_amsdos_fname);
                            header.update_checksum();

                            // replace the header with the modified filename
                            let content = ams_file.content();
                            ams_file = AmsdosFile::from_header_and_buffer(header, content)?;
                        }

                        assert!(
                            ams_file.amsdos_filename().unwrap()?.is_valid(),
                            "Invalid amsdos filename ! {:?}",
                            ams_file.amsdos_filename().unwrap()
                        );
                        println!("{:?} added", ams_file.amsdos_filename());
                        ams_file
                    },
                    Err(e) => {
                        panic!("Unable to load {fname}: {e:?}");
                    }
                };

                disc.add_amsdos_file(&ams_file, head, is_system, is_read_only, behavior)
                    .unwrap();
            }
        }

        // Save the dsk on disc
        disc.save(dsk_fname)
            .map_err(|e| DskManagerError::AnyError { msg: e })?;
    }
    else if let Some(sub) = matches.subcommand_matches("format") {
        // Manage the formating of a disc
        use crate::cfg::DiscConfig;

        // Retrieve the format description
        // fallback to standard data
        let cfg = if let Some(desc_fname) = sub.get_one::<String>("FORMAT_FILE") {
            crate::cfg::DiscConfig::new(desc_fname)?
        }
        else if let Some(desc) = sub.get_one::<String>("FORMAT_NAME") {
            match desc.as_str() {
                "data42" => DiscConfig::single_head_data42_format(),
                "data" => DiscConfig::single_head_data_format(),
                _ => unreachable!()
            }
        }
        else {
            DiscConfig::single_head_data_format()
        };

        // Make the dsk based on the format
        let dsk = crate::builder::build_edsk_from_cfg(&cfg);
        dsk.save(dsk_fname)
            .map_err(|e| DskManagerError::AnyError { msg: e })?;
    }
    else {
        eprintln!("Missing command\n");
    }

    Ok(())
}

#[cfg(feature = "cmdline")]
pub fn dsk_manager_build_arg_parser() -> Command {
    #[cfg(feature = "hfe")]
    let about = "Manipulate DSK or HFE files";
    #[cfg(not(feature = "hfe"))]
    let about = "Manipulate DSK files";

    #[cfg(feature = "hfe")]
    let f_help = "DSK or HFE file to manipulate";
    #[cfg(not(feature = "hfe"))]
    let f_help = "DSK file to manipulate";

    Command::new("disc_manager")
                       .about(about)
                       .author("Krusty/Benediction")
                       .after_help("Pale buggy copy of an old Ramlaid's tool")
                       .arg(
                           Arg::new("DSK_FILE")
                            .help(f_help)
                            .required(true)
                            .index(1)
                       )
                       .subcommand(
                           Command::new("format")
                            .about("Format a dsk")
                            .arg(
                                Arg::new("FORMAT_FILE")
                                    .help("Provide a file that describes the format of the disc")
                                    .long("description")
                                    .short('d')
                            )
                            .arg(
                                Arg::new("FORMAT_NAME")
                                    .help("Provide the name of a format that can be used")
                                    .short('f')
                                    .long("format")
                                    .value_parser(["data", "data42"])
                            )
                            .group(
                                ArgGroup::new("command")
                                    .arg("FORMAT_FILE")
                                    .arg("FORMAT_NAME")
                            )
                       )
                       .subcommand(
                           Command::new("catalog")
                           .about("Manipulate the catalog. Can only works for DSK having a Track 0 compatible with Amsdos")
                           .arg(
                               Arg::new("IMPORT")
                                .help("Import an existing catalog in the dsk. All entries are thus erased")
                                .long("import")
                                .short('i')
                           )
                           .arg(
                               Arg::new("EXPORT")
                                .help("Export the catalog in a specific file")
                                .long("export")
                                .short('e')
                           )
                           .arg(
                               Arg::new("LIST")
                               .help("Display the catalog on screen")
                               .long("list")
                               .short('l')
                        .action(ArgAction::SetTrue)

                           )
                           .arg(
                               Arg::new("CATART")
                               .help("[unimplemented] Display the catart version")
                               .long("catart")
                        .action(ArgAction::SetTrue)

                           )
                           .group(
                               ArgGroup::new("command")
                                .arg("IMPORT")
                                .arg("EXPORT")
                                .arg("LIST")
                                .arg("CATART")
                                .required(true)
                           )
                       )
                       .subcommand(
                            Command::new("get")
                                .about("Retrieve files for the disc in the Amsdos way")
                                .arg(Arg::new("OUTPUT_FILES")
                                    .help("The files to retrieve")
                                    .action(ArgAction::Append)
                                    .required(true)
                                )
                                .arg(
                                    Arg::new("noheader")
                                    .long("no-header")
                                    .help("Do not store the header of the file")
                                    .action(ArgAction::SetTrue)
                                )
                            )
                       .subcommand(
                           Command::new("add")
                           .about("Add files in the disc in an Amsdos way")
                           .arg(
                               Arg::new("INPUT_FILES")
                                .help("The files to add. They MUST have a header if they are BINARY or BASIC files. Otherwise, they are considered to be ASCII files.")
                                .action(ArgAction::Append)
                                .required(true)
                                .value_parser(clap::value_parser!(Utf8PathBuf))
                            )
                            .arg(
                                Arg::new("ASCII")
                                    .help("Completly ignore header aspect. If it is present it will be an amsdos file. If it is absent, it will be an ascii file.")
                                    .action(ArgAction::SetTrue)
                                    .long("ascii")
                                    .short('a')
                            )
                            .arg(
                                Arg::new("SYSTEM")
                                .help("Indicates if the files are system files")
                                .long("system")
                                .short('s')
                        .action(ArgAction::SetTrue)

                            )
                            .arg(
                                Arg::new("READ_ONLY")
                                .help("Indicates if the files are read only")
                                .long("read_only")
                                .short('r')
                        .action(ArgAction::SetTrue)

                            )
                            .arg(
                                Arg::new("AS_AMSDOS")
                                .help("[unimplemented] Uses the same strategy as amsdos when adding a file: add .???, delete .BAK, rename other as .BAK, rename .??? with real extension")
                                .long("secure")
                        .action(ArgAction::SetTrue)

                            )
                       )
                       .subcommand(
                           Command::new("put")
                           .about("Add files in the disc in a sectorial way")
                           .arg(
                               Arg::new("TRACK")
                                .help("The track of interest")
                                .short('a')
                                .required(true)
                           )
                           .arg(
                               Arg::new("SECTOR")
                                .help("The sector of interest")
                                .short('o')
                                .required(true)
                           )
                           .arg(
                               Arg::new("SIDE")
                                .help("The head of interest")
                                .short('p')
                                .required(true)
                           )
                           .arg(
                               Arg::new("Z80_EXPORT")
                               .help("The path to the z80 files that will contains all the import information")
                                .short('z')
                                .required(false)
                           )
                           .arg(
                               Arg::new("FILES")
                               .help("The ordered list of files to import in the dsk")
                               .action(ArgAction::Append)
                                .required(true)
                                .last(true)
                           )
                       )
}

/// Open the file and remove the header if any
pub fn read<P: AsRef<Utf8Path>>(p: P) -> Result<(VecDeque<u8>, Option<AmsdosHeader>), AmsdosError> {
    let data = std::fs::read(p.as_ref()).map_err(|e| AmsdosError::IO(e.to_string()))?;

    Ok(split_header(data))
}

/// Extract the header from binary data
pub fn split_header<D: Into<VecDeque<u8>>>(data: D) -> (VecDeque<u8>, Option<AmsdosHeader>) {
    let mut data = data.into();

    // get a slice on the data to ease its cut
    let header = if data.len() >= 128 {
        // by construction there is only one slice
        let header = AmsdosHeader::from_buffer(data.as_slices().0);

        if header.represent_a_valid_file() {
            data.drain(..128);
            Some(header)
        }
        else {
            None
        }
    }
    else {
        None
    };

    (data, header)
}
