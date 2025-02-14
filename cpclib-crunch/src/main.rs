use cpclib_common::camino::Utf8PathBuf;
use cpclib_common::clap;
use clap::{Parser, ValueEnum};
use cpclib_crunchers::{lzsa::LzsaVersion, CompressMethod};
use cpclib_disc::amsdos::{AmsdosAddBehavior, AmsdosFile, AmsdosFileName};
use cpclib_files::FileAndSupport;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CrunchArgs {

    #[arg(short, long, help="Cruncher of interest")]
    cruncher: Cruncher,

    #[arg(short, long, help="Input file to compress. Can be a binary (my_file.o), an Amsdos file (FILE.BIN), a file in a disc (my_disc.dsk#FILE.BIN")]
    input: Utf8PathBuf,

    #[arg(short, long, help="Also crunch the header. This is useful for binary files where the first bytes still correspond to a valid amsdos header", default_value_t=false)]
    keep_header: bool,

    #[arg(short, long, help="Compressed output file. Can be a binary, an Amsdos file, a file in a disc")]
    output: Utf8PathBuf,

    #[arg(short='H', long, help="Add a header when storing the file on the host", default_value_t=false)]
    header: bool
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



fn main() {
    let args = CrunchArgs::parse();

    // TODO move loading code in the disc crate
    let (data, header) = cpclib_asm::file::load_file(args.input.as_path(), &Default::default())
        .map_err(|e| format!("Unable to load the input file {}.\n{}", args.input, e))
        .unwrap();

    // keep header if needed
    let data = if args.keep_header {
        if let Some(header) = header {
            let mut header = header.as_bytes().to_vec();
            let data: Vec<u8> = data.into();
            header.extend_from_slice(&data);
            header
        } else {
            eprintln!("There is no header in the input file");
            data.into()
        }

    } else {
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
        Cruncher::Zx0 => CompressMethod::Zx0,
    };

    let crunched = cruncher.compress(&data)
        .expect("Error when crunching file");


    let file_and_support = FileAndSupport::new_auto(args.output, args.header);

    dbg!(&file_and_support);
    
    file_and_support.save(
        &crunched, 
        Some(0xc000),
        None,
        Some(AmsdosAddBehavior::ReplaceAndEraseIfPresent)
    ).expect("Error when saving the file");

}
