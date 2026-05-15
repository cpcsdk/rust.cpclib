use clap::{Parser, ValueEnum};
use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::clap;
use cpclib_common::clap::CommandFactory;
use cpclib_common::event::EventObserver;
use cpclib_crunchers::CompressMethod;

#[cfg(feature = "lzsa")]
use cpclib_crunchers::lzsa::LzsaVersion;
use cpclib_disc::amsdos::AmsdosAddBehavior;
use cpclib_files::FileAndSupport;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CrunchArgs {
    #[arg(short, long, help = "Cruncher of interest")]
    cruncher: Cruncher,

    #[arg(
        short,
        long,
        help = "Input file to compress. Can be a binary (my_file.o), an Amsdos file (FILE.BIN), a file in a disc (my_disc.dsk#FILE.BIN"
    )]
    input: Option<Utf8PathBuf>,

    #[arg(
        short,
        long,
        help = "Also crunch the header. This is useful for binary files where the first bytes still correspond to a valid amsdos header",
        default_value_t = false
    )]
    keep_header: bool,

    #[arg(
        short,
        long,
        help = "Compressed output file. Can be a binary, an Amsdos file, a file in a disc",
        requires = "input"
    )]
    output: Option<Utf8PathBuf>,

    #[arg(
        short = 'H',
        long,
        help = "Add a header when storing the file on the host",
        default_value_t = false
    )]
    header: bool,

    #[arg(
        short,
        long,
        help = "Show the z80 decompression source",
        default_value_t = false,
        conflicts_with = "input"
    )]
    z80: bool
}

#[derive(Debug, ValueEnum, Clone)]
pub enum Cruncher {
    #[cfg(feature = "apultra")]
    Apultra,
    #[cfg(feature = "zx0")]
    BackwardZx0,
    #[cfg(feature = "exomizer")]
    Exomizer,
    #[cfg(feature = "lz4")]
    Lz4,
    #[cfg(feature = "lz48")]
    Lz48,
    #[cfg(feature = "lz49")]
    Lz49,
    #[cfg(feature = "lzsa")]
    Lzsa1,
    #[cfg(feature = "lzsa")]
    Lzsa2,
    #[cfg(feature = "pucrunch")]
    Pucrunch,
    #[cfg(feature = "shrinkler")]
    Shrinkler,
    #[cfg(feature = "upkr")]
    Upkr,
    #[cfg(feature = "zx0")]
    Zx0
}

impl Cruncher {
    #[cfg(any(feature = "apultra", feature = "exomizer", feature = "lz4", feature = "lz48", feature = "lz49", feature = "lzsa", feature = "pucrunch", feature = "shrinkler", feature = "zx7", feature = "upkr", feature = "zx0"))]
    pub fn z80(&self) -> &Utf8Path {
        let fname: &str = match self {
            #[cfg(feature = "apultra")]
            Cruncher::Apultra => "inner://unaplib.asm",
            #[cfg(feature = "zx0")]
            Cruncher::BackwardZx0 => "inner://uncrunch/dzx0_standard_back.asm",
            #[cfg(feature = "exomizer")]
            Cruncher::Exomizer => "inner://deexo.asm",
            #[cfg(feature = "lz4")]
            Cruncher::Lz4 => "inner://lz4_docent.asm",
            #[cfg(feature = "lz48")]
            Cruncher::Lz48 => "inner://lz48decrunch.asm",
            #[cfg(feature = "lz49")]
            Cruncher::Lz49 => "inner://lz49decrunch.asm",
            #[cfg(feature = "lzsa")]
            Cruncher::Lzsa1 => "inner://unlzsa1_fast.asm",
            #[cfg(feature = "lzsa")]
            Cruncher::Lzsa2 => "inner://unlzsa1_fast.asm",
            #[cfg(feature = "pucrunch")]
            Cruncher::Pucrunch => "inner://uncrunch/pucrunch_z80.asm",
            #[cfg(feature = "shrinkler")]
            Cruncher::Shrinkler => "inner://deshrink.asm",
            #[cfg(feature = "zx0")]
            Cruncher::Zx0 => "inner://dzx0_fast.asm",
            #[cfg(feature = "upkr")]
            Cruncher::Upkr => "inner://uncrunch/upkr.asm"
        };
        fname.into()
    }
}

pub fn command() -> clap::Command {
    CrunchArgs::command()
}

pub fn process(args: CrunchArgs, o: &dyn EventObserver) -> Result<(), String> {
    if args.z80 {
        #[cfg(not(any(feature = "apultra", feature = "exomizer", feature = "lz4", feature = "lz48", feature = "lz49", feature = "lzsa", feature = "pucrunch", feature = "shrinkler", feature = "zx7", feature = "upkr", feature = "zx0")))]
        panic!("This is a bug, please report it. The z80 option should not be available if no cruncher is enabled");

        #[cfg(any(feature = "apultra", feature = "exomizer", feature = "lz4", feature = "lz48", feature = "lz49", feature = "lzsa", feature = "pucrunch", feature = "shrinkler", feature = "zx7", feature = "upkr", feature = "zx0"))]
        {let fname = args.cruncher.z80();
        let content = cpclib_asm::file::load_file(fname, &Default::default())
            .unwrap()
            .0;
        let content = Vec::from(content);
        let content = String::from_utf8(content).unwrap();
        o.emit_stdout(&format!(
            "; Import \"{fname}\" in basm or include the following content:\n{content}"
        ));
        return Ok(());
        }
    }

    // TODO move loading code in the disc crate
    let (data, header) =
        cpclib_asm::file::load_file(args.input.as_ref().unwrap().as_path(), &Default::default())
            .map_err(|e| {
                format!(
                    "Unable to load the input file {}.\n{}",
                    args.input.unwrap(),
                    e
                )
            })
            .map_err(|e| format!("Error while loading the file. {e}"))?;

    // keep header if needed
    let data = if args.keep_header {
        if let Some(header) = header {
            let mut header = header.as_bytes().to_vec();
            let data: Vec<u8> = data.into();
            header.extend_from_slice(&data);
            header
        }
        else {
            o.emit_stderr("There is no header in the input file");
            data.into()
        }
    }
    else {
        data.into()
    };

    // TODO eventually get additional options to properly parametrize them
    let cruncher: CompressMethod = match args.cruncher {
        #[cfg(feature = "apultra")]
        Cruncher::Apultra => CompressMethod::Apultra,
        #[cfg(feature = "exomizer")]
        Cruncher::Exomizer => CompressMethod::Exomizer,
        #[cfg(feature = "lz4")]
        Cruncher::Lz4 => CompressMethod::Lz4,
        #[cfg(feature = "lz48")]
        Cruncher::Lz48 => CompressMethod::Lz48,
        #[cfg(feature = "lz49")]
        Cruncher::Lz49 => CompressMethod::Lz49,
        #[cfg(feature = "lzsa")]
        Cruncher::Lzsa1 => CompressMethod::Lzsa(LzsaVersion::V1, None),
        #[cfg(feature = "lzsa")]
        Cruncher::Lzsa2 => CompressMethod::Lzsa(LzsaVersion::V1, None),
        #[cfg(feature = "pucrunch")]
        Cruncher::Pucrunch => CompressMethod::Pucrunch,
        #[cfg(feature = "shrinkler")]
        Cruncher::Shrinkler => CompressMethod::Shrinkler(Default::default()),
        #[cfg(feature = "zx0")]
        Cruncher::Zx0 => CompressMethod::Zx0,
        #[cfg(feature = "upkr")]
        Cruncher::Upkr => CompressMethod::Upkr,
        #[cfg(feature = "backward_zx0")]
        Cruncher::BackwardZx0 => CompressMethod::BackwardZx0
    };

    let crunched = cruncher
        .compress(&data)
        .map_err(|_e| "Error when crunching file.".to_string())?;

    let file_and_support = FileAndSupport::new_auto(args.output.unwrap(), args.header);

    file_and_support
        .save(
            &crunched,
            Some(0xC000),
            None,
            Some(AmsdosAddBehavior::ReplaceAndEraseIfPresent)
        )
        .map_err(|e| format!("Error when saving the file. {e}"))?;

    Ok(())
}
