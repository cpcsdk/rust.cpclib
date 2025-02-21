use clap::{Parser, ValueEnum};
use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::clap;
use cpclib_common::clap::CommandFactory;
use cpclib_crunchers::lzsa::LzsaVersion;
use cpclib_crunchers::CompressMethod;
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
    Apultra,
    Exomizer,
    Lz4,
    Lz48,
    Lz49,
    Lzsa1,
    Lzsa2,
    Shrinkler,
    Zx0
}

impl Cruncher {
    pub fn z80(&self) -> &Utf8Path {
        let fname = match self {
            Cruncher::Apultra => "inner://unaplib.asm",
            Cruncher::Exomizer => "inner://deexo.asm",
            Cruncher::Lz4 => "inner://lz4_docent.asm",
            Cruncher::Lz48 => "inner://lz48decrunch.asm",
            Cruncher::Lz49 => "inner://lz49decrunch.asm",
            Cruncher::Lzsa1 => "inner://unlzsa1_fast.asm",
            Cruncher::Lzsa2 => "inner://unlzsa1_fast.asm",
            Cruncher::Shrinkler => "inner://deshrink.asm",
            Cruncher::Zx0 => "inner://dzx0_fast.asm"
        };
        fname.into()
    }
}

pub fn command() -> clap::Command {
    CrunchArgs::command()
}

pub fn process(args: CrunchArgs) -> Result<(), String> {
    if args.z80 {
        let fname = args.cruncher.z80();
        let content = cpclib_asm::file::load_file(fname, &Default::default())
            .unwrap()
            .0;
        let content = Vec::from(content);
        let content = String::from_utf8(content).unwrap();
        println!("; Import \"{fname}\" in basm or include the following content:\n{content}");
        return Ok(());
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
            eprintln!("There is no header in the input file");
            data.into()
        }
    }
    else {
        data.into()
    };

    // TODO eventually get additional options to properly parametrize them
    let cruncher = match args.cruncher {
        Cruncher::Apultra => CompressMethod::Apultra,
        Cruncher::Exomizer => CompressMethod::Exomizer,
        Cruncher::Lz4 => CompressMethod::Lz4,
        Cruncher::Lz48 => CompressMethod::Lz48,
        Cruncher::Lz49 => CompressMethod::Lz49,
        Cruncher::Lzsa1 => CompressMethod::Lzsa(LzsaVersion::V1, None),
        Cruncher::Lzsa2 => CompressMethod::Lzsa(LzsaVersion::V1, None),
        Cruncher::Shrinkler => CompressMethod::Shrinkler(Default::default()),
        Cruncher::Zx0 => CompressMethod::Zx0
    };

    let crunched = cruncher
        .compress(&data)
        .map_err(|e| "Error when crunching file.".to_string())?;

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
