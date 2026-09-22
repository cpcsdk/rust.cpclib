//! `size_map` - where do the bytes go? Assembles a source and reports its
//! memory map, optionally with crunched costs - the CLI over
//! `cpclib_crunch::sizemap`, for use directly from a shell/script, no MCP
//! host required. See that module's own doc comment for what it computes.

use std::io::Write;
use std::process;

use cpclib_common::camino::Utf8PathBuf;
use cpclib_common::clap::{self, Parser};
use cpclib_crunch::sizemap::{SizeMapOptions, size_map};

/// A CPC address as `basm` itself accepts one: decimal, or `0x`/`&`/`#`
/// hex - not just plain decimal, so a value copied out of a `.lst`/source
/// file (almost always hex there) can be pasted straight in.
fn parse_address(s: &str) -> Result<u32, String> {
    let digits = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).or_else(|| s.strip_prefix('&')).or_else(|| s.strip_prefix('#'));
    match digits {
        Some(hex) => u32::from_str_radix(hex, 16).map_err(|e| format!("'{s}' is not a valid hex address: {e}")),
        None => s.parse().map_err(|e| format!("'{s}' is not a valid address: {e}"))
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "size_map",
    about = "Where do the bytes go? Assemble a source and report its memory map.",
    long_about = "Assembles a .asm file (dry run - nothing is written to disk, SAVE directives \
                  are never executed) and reports, per global label, how many bytes it spans and \
                  what share of the image that is. With --cruncher it also measures each \
                  region's *crunched* cost (the whole image crunched, minus the image without \
                  that region), and breaks down every crunched section (LZ48/LZSHRINKLER/...) \
                  in the source by what went into it, since the image only holds its compressed \
                  output. Labels come from the assembler's own symbol table (real addresses \
                  only, never `equ` constants)."
)]
struct Cli {
    /// The .asm source file to assemble and map.
    source: Utf8PathBuf,

    /// Extra directory to search when resolving this file's INCLUDEs - same
    /// role as basm's own -I/--include. Repeatable.
    #[arg(short = 'I', long = "include", value_name = "DIR")]
    include_dirs: Vec<String>,

    /// Define a symbol before assembling, NAME (= 1) or NAME=VALUE, same
    /// role as basm's own -D. Give whatever the real build passes on its
    /// command line (e.g. LINKED_VERSION=1) or the map won't match it.
    /// Repeatable.
    #[arg(short = 'D', long = "define", value_name = "NAME[=VALUE]")]
    defines: Vec<String>,

    /// Only report regions at or above this address (default: where the
    /// produced bytes start). Decimal or 0x/&/# hex.
    #[arg(long, value_name = "ADDR", value_parser = parse_address)]
    min_address: Option<u32>,

    /// Only report regions below this address (default: end of the
    /// produced bytes). Decimal or 0x/&/# hex.
    #[arg(long, value_name = "ADDR", value_parser = parse_address)]
    max_address: Option<u32>,

    /// Also measure each region's crunched cost with this cruncher (e.g.
    /// shrinkler, zx0, upkr - see cpclib-crunch's own --cruncher list for
    /// every name).
    #[arg(short, long)]
    cruncher: Option<String>,

    /// How many of the biggest regions to list.
    #[arg(long, default_value_t = 25)]
    top: usize,

    /// Print the full report as JSON instead of markdown tables - for a
    /// script to consume, rather than a person to read.
    #[arg(long)]
    json: bool
}

fn main() {
    // Parsed (and `--help`/`--version`/a usage error printed and exited on,
    // through the real stdout/stderr) before the stdout redirect below, so
    // that conventional shell behavior for those is unaffected.
    let cli = Cli::parse();

    // Before any cruncher can run: a native one (Shrinkler in particular)
    // writes its own progress straight to the real process stdout, which
    // would otherwise land in the middle of the report below - see
    // `stdio_guard`'s own doc comment.
    let private_stdout = cpclib_crunch::stdio_guard::private_stdout();
    let mut out: Box<dyn Write> = match &private_stdout {
        Some(f) => Box::new(f),
        None => Box::new(std::io::stdout())
    };

    let options = SizeMapOptions {
        include_dirs: cli.include_dirs,
        defines: cli.defines,
        min_address: cli.min_address,
        max_address: cli.max_address,
        cruncher: cli.cruncher,
        top: Some(cli.top)
    };

    let report = match size_map(&cli.source, &options) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    if cli.json {
        writeln!(out, "{}", serde_json::to_string_pretty(&report).expect("SizeMapReport always serializes")).ok();
        return;
    }

    writeln!(
        out,
        "{}: {} bytes (0x{:04x}-0x{:04x}){}\n",
        report.path,
        report.image_bytes,
        report.start,
        report.end,
        report.cruncher.as_deref().map_or(String::new(), |c| {
            format!(", {} bytes crunched with {c}", report.crunched_image.map_or("?".to_string(), |v| v.to_string()))
        })
    )
    .ok();
    writeln!(out, "{}", report.table).ok();

    if let Some(listing) = &report.listing {
        writeln!(out, "By source file:\n\n{}", listing.files_table).ok();
        if listing.macros_table.lines().count() > 2 {
            writeln!(out, "By macro:\n\n{}", listing.macros_table).ok();
        }
        writeln!(out, "By source line:\n\n{}", listing.lines_table).ok();
        if listing.unowned_image_bytes > 0 {
            writeln!(
                out,
                "{} image byte(s) not accounted for by any placeable source-map row.",
                listing.unowned_image_bytes
            )
            .ok();
        }
    }

    for section in &report.crunched_sections {
        writeln!(
            out,
            "\nCrunched section #{} ({}) at 0x{:04x}: {} bytes in, {} bytes out (remeasured: {})",
            section.index,
            section.cruncher,
            section.address,
            section.decrunched_bytes,
            section.crunched_bytes,
            section.remeasured_crunched_bytes.map_or("?".to_string(), |v| v.to_string())
        )
        .ok();
        writeln!(out, "{}", section.regions_table).ok();
        writeln!(out, "{}", section.lines_table).ok();
    }
}
