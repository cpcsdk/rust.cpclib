use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use cpclib_asm::parser::instructions::INSTRUCTIONS;
use cpclib_asm::parser::parser::{END_DIRECTIVE, STAND_ALONE_DIRECTIVE, START_DIRECTIVE};

/// Generate syntax highlighting keyword strings for the template
fn generate_syntax_highlighting() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("syntax_keywords.rs");
    let file = File::create(&dest_path).unwrap();
    let mut file = BufWriter::new(file);

    // Generate SYNTAX_INSTRUCTIONS from the INSTRUCTIONS array
    let instructions_str = INSTRUCTIONS
        .iter()
        .map(|&instr| std::str::from_utf8(instr).unwrap())
        .collect::<Vec<_>>()
        .join(" ");

    writeln!(
        file,
        "pub const SYNTAX_INSTRUCTIONS: &str = \"{}\";",
        instructions_str
    )
    .unwrap();

    // Generate SYNTAX_DIRECTIVES from all directive arrays
    let mut all_directives = Vec::new();
    all_directives.extend(STAND_ALONE_DIRECTIVE.iter());
    all_directives.extend(START_DIRECTIVE.iter());
    all_directives.extend(END_DIRECTIVE.iter());

    let directives_str = all_directives
        .iter()
        .map(|&directive| std::str::from_utf8(directive).unwrap())
        .collect::<Vec<_>>()
        .join(" ");

    writeln!(
        file,
        "pub const SYNTAX_DIRECTIVES: &str = \"{}\";",
        directives_str
    )
    .unwrap();

    file.flush().unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}

fn main() {
    generate_syntax_highlighting();
}
