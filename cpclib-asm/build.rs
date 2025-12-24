use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

const UNARY_FUNCTIONS: &[&[u8]] = &[
    b"ABS", b"ACOS", b"ASIN", b"CEIL", b"CHAR", b"COS", b"EXP", b"FLOOR", b"FRAC", b"HI", b"HIGH",
    b"INT", b"LN", b"LO", b"LOG10", b"LOW", b"MEMORY", b"PEEK", b"SIN", b"SQRT"
];

const NO_ARGS_FUNCTIONS: &[&[u8]] = &[b"RND"];

const BINARY_FUNCTIONS: &[&[u8]] = &[b"MAX", b"MIN", b"POW"];

// Directive and instruction definitions (must match parser.rs and instructions.rs)
const STAND_ALONE_DIRECTIVE: &[&[u8]] = &[
    b"#",
    b"ABYTE",
    b"ALIGN",
    b"ASMCONTROL",
    b"ASSERT",
    b"BANK",
    b"BANKSET",
    b"BINCLUDE",
    b"BREAK",
    b"BREAKPOINT",
    b"BUILDCPR",
    b"BUILDSNA",
    b"BYTE",
    b"CASE",
    b"CHARSET",
    b"DB",
    b"DEFAULT",
    b"DEFB",
    b"DEFM",
    b"DEFS",
    b"DEFW",
    b"DEFSECTION",
    b"DM",
    b"DS",
    b"FOR",
    b"DW",
    b"ELSE",
    b"ELSEIF",
    b"ELSEIFDEF",
    b"ELSEIFEXIST",
    b"ELSEIFNDEF",
    b"ELSEIFNOT",
    b"ELSEIFUSED",
    //  b"END",
    b"ENT",
    b"EQU",
    b"EXPORT",
    b"FAIL",
    b"INCBIN",
    b"INCLUDE",
    b"INCLZ4",
    b"INCEXO",
    b"INCL48",
    b"INCL49",
    b"INCLZSA1",
    b"INCLZSA2",
    b"INCAPU",
    b"INCZX0",
    b"INCZX0_BACKWARD",
    b"INCSHRINKLER",
    b"INCUPKR",
    b"LET",
    b"LIMIT",
    b"LIST",
    b"LZEXO",
    b"LZSA1",
    b"LZSA2",
    b"LZUPKR",
    b"MAP",
    b"MODULE",
    b"NOEXPORT",
    b"NOLIST",
    b"NOP",
    b"ORG",
    b"PAUSE",
    b"PRINT",
    b"PROTECT",
    b"RANGE",
    b"READ",
    b"REND",
    b"REPEAT",
    b"REP",
    b"REPT",
    b"RORG",
    b"RETURN",
    b"RUN",
    b"SAVE",
    b"SECTION",
    b"SNAINIT",
    b"SNAPINIT",
    b"SNASET",
    b"STARTINGINDEX",
    b"STR",
    b"TEXT",
    b"TICKER",
    b"UNDEF",
    b"UNTIL",
    b"WAITNOPS",
    b"WORD",
    b"WRITE DIRECT",
    b"WRITE"
];

const START_DIRECTIVE: &[&[u8]] = &[
    b"ASMCONTROLENV",
    b"CONFINED",
    b"FUNCTION",
    b"FOR",
    b"IF",
    b"IFDEF",
    b"IFEXIST",
    b"IFNDEF",
    b"IFNOT",
    b"IFUSED",
    b"IFNUSED",
    b"ITER",
    b"ITERATE",
    b"LZ4",
    b"LZ48",
    b"LZ49",
    b"LZ48",
    b"LZAPU",
    b"LZX0_BACKWARD",
    b"LZX0",
    b"LZEXO",
    b"LZ4",
    b"LZX7",
    b"LZSHRINKLER",
    b"LOCOMOTIVE",
    b"MACRO",
    b"MODULE",
    b"PHASE",
    b"REPEAT",
    b"REPT",
    b"STRUCT",
    b"SWITCH",
    b"WHILE"
];

const END_DIRECTIVE: &[&[u8]] = &[
    b"END", // for orgams
    b"ENDASMCONTROLENV",
    b"ENDA",
    b"BREAK",
    b"CASE",
    b"CEND",
    b"DEFAULT",
    b"DEPHASE",
    b"ELSE",
    b"ELSEIF",
    b"ELSEIFDEF",
    b"ELSEIFEXIST",
    b"ELSEIFNDEF",
    b"ELSEIFNOT",
    b"ELSEIFUSED",
    b"ENDC",
    b"ENDCONFINED",
    b"ENDF",
    b"ENDFOR",
    b"ENDFUNCTION",
    b"ENDI",
    b"ENDIF", // if directive
    b"ENDITER",
    b"ENDITERATE",
    b"ENDM",
    b"ENDMACRO",
    b"ENDMODULE",
    b"ENDR",
    b"ENDREP", // repeat directive
    b"ENDREPEAT",
    b"ENDS",
    b"ENDSWITCH",
    b"ENDW",
    b"ENDWHILE",
    b"FEND",
    b"IEND",
    b"LZCLOSE",
    b"REND", // rorg directive
    b"UNTIL",
    b"WEND"
];

const REGISTERS: &[&[u8]] = &[b"AF", b"HL", b"DE", b"BC", b"IX", b"IY", b"IXL", b"IXH"];

const INSTRUCTIONS: &[&[u8]] = &[
    b"ADC", b"ADD", b"AND", b"BIT", b"CALL", b"CCF", b"CP", b"CPD", b"CPDR", b"CPI", b"CPIR",
    b"CPL", b"DAA", b"DEC", b"DI", b"DJNZ", b"EI", b"EX", b"EXA", b"EXX", b"HALT", b"IM", b"IN",
    b"INC", b"IND", b"INDR", b"INI", b"INIR", b"JP", b"JR", b"LD", b"LDD", b"LDDR", b"LDI",
    b"LDIR", b"NEG", b"NOP", b"OR", b"OTDR", b"OTIR", b"OUT", b"OUTD", b"OUTI", b"POP", b"PUSH",
    b"RES", b"RET", b"RETI", b"RETN", b"RL", b"RLA", b"RLC", b"RLCA", b"RLD", b"RR", b"RRA",
    b"RRC", b"RRCA", b"RRD", b"RST", b"SBC", b"SCF", b"SET", b"SLA", b"SLL", b"SRA", b"SRL",
    b"SUB", b"XOR"
];

// Orgams-specific forbidden names (keep this list in sync with expression.rs)
const STAND_ALONE_DIRECTIVE_ORGAMS: &[&[u8]] = &[
    b"BANK",
    b"BRK",
    b"BUILDSNA",
    b"BY",
    b"BYTE",
    b"DB",
    b"DEFB",
    b"DEFS",
    b"ELSE", //  b"END",
    b"ENT",
    b"IMPORT",
    b"ORG",
    b"PRINT",
    b"SKIP",
    b"WORD"
];
const START_DIRECTIVE_ORGAMS: &[&[u8]] = &[b"IF", b"MACRO"];
const END_DIRECTIVE_ORGAMS: &[&[u8]] = &[b"END", b"ENDM", b"]"];

fn generate_forbidden_names() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("forbidden_names_generated.rs");
    let mut f = File::create(&dest_path).unwrap();

    // Helper: convert byte slice to string (case-insensitive ASCII)
    fn to_str(b: &[u8]) -> &str {
        std::str::from_utf8(b).unwrap()
    }

    // Build standard impossible names (directives + registers + instructions)
    let mut impossible_names: Vec<String> = STAND_ALONE_DIRECTIVE
        .iter()
        .chain(START_DIRECTIVE.iter())
        .chain(END_DIRECTIVE.iter())
        .chain(REGISTERS.iter())
        .chain(INSTRUCTIONS.iter())
        .map(|b| to_str(b).to_string())
        .collect();
    impossible_names.sort();
    impossible_names.dedup();

    // Build dotted versions (keep same semantics as previous LazyLock data):
    // registers + instructions (not dotted) + dotted directives
    let mut dotted_impossible_names: Vec<String> = REGISTERS
        .iter()
        .chain(INSTRUCTIONS.iter())
        .map(|b| to_str(b).to_string())
        .chain(
            STAND_ALONE_DIRECTIVE
                .iter()
                .chain(START_DIRECTIVE.iter())
                .chain(END_DIRECTIVE.iter())
                .map(|b| format!(".{}", to_str(b)))
        )
        .collect();
    dotted_impossible_names.sort();
    dotted_impossible_names.dedup();

    // Build orgams versions (orgams-only directive set + registers/instructions)
    let mut impossible_names_orgams: Vec<String> = REGISTERS
        .iter()
        .chain(INSTRUCTIONS.iter())
        .chain(STAND_ALONE_DIRECTIVE_ORGAMS.iter())
        .chain(START_DIRECTIVE_ORGAMS.iter())
        .chain(END_DIRECTIVE_ORGAMS.iter())
        .map(|b| to_str(b).to_string())
        .collect();
    impossible_names_orgams.sort();
    impossible_names_orgams.dedup();

    // Compute length-based buckets
    fn bucket_by_length(names: &[String]) -> (HashMap<usize, Vec<String>>, usize, usize) {
        let mut buckets: HashMap<usize, Vec<String>> = HashMap::new();
        let mut min_len = usize::MAX;
        let mut max_len = 0;

        for name in names {
            let len = name.len();
            buckets.entry(len).or_default().push(name.clone());
            min_len = min_len.min(len);
            max_len = max_len.max(len);
        }

        (buckets, min_len, max_len)
    }

    let (buckets, min_len, max_len) = bucket_by_length(&impossible_names);
    let (dotted_buckets, dotted_min_len, dotted_max_len) =
        bucket_by_length(&dotted_impossible_names);
    let (orgams_buckets, orgams_min_len, orgams_max_len) =
        bucket_by_length(&impossible_names_orgams);

    // Write generated code
    writeln!(f, "// Auto-generated by build.rs - DO NOT EDIT").unwrap();
    writeln!(f).unwrap();

    // Write flat arrays
    writeln!(f, "pub const IMPOSSIBLE_NAMES: &[&str] = &[").unwrap();
    for name in &impossible_names {
        writeln!(f, "    {:?},", name).unwrap();
    }
    writeln!(f, "];").unwrap();
    writeln!(f).unwrap();

    writeln!(f, "pub const DOTTED_IMPOSSIBLE_NAMES: &[&str] = &[").unwrap();
    for name in &dotted_impossible_names {
        writeln!(f, "    {:?},", name).unwrap();
    }
    writeln!(f, "];").unwrap();
    writeln!(f).unwrap();

    writeln!(f, "pub const IMPOSSIBLE_NAMES_ORGAMS: &[&str] = &[").unwrap();
    for name in &impossible_names_orgams {
        writeln!(f, "    {:?},", name).unwrap();
    }
    writeln!(f, "];").unwrap();
    writeln!(f).unwrap();

    // Write min/max ranges
    writeln!(
        f,
        "pub const MIN_MAX_LABEL_SIZE: (usize, usize) = ({}, {});",
        min_len, max_len
    )
    .unwrap();
    writeln!(
        f,
        "pub const DOTTED_MIN_MAX_LABEL_SIZE: (usize, usize) = ({}, {});",
        dotted_min_len, dotted_max_len
    )
    .unwrap();
    writeln!(
        f,
        "pub const ORGAMS_MIN_MAX_LABEL_SIZE: (usize, usize) = ({}, {});",
        orgams_min_len, orgams_max_len
    )
    .unwrap();
    writeln!(f).unwrap();

    // Write bucket functions (using match for O(1) lookup)
    fn write_bucket_match(f: &mut File, fn_name: &str, buckets: &HashMap<usize, Vec<String>>) {
        writeln!(
            f,
            "pub fn {}(len: usize) -> &'static [&'static str] {{",
            fn_name
        )
        .unwrap();
        writeln!(f, "    match len {{").unwrap();
        let mut sorted_keys: Vec<_> = buckets.keys().collect();
        sorted_keys.sort();
        for key in sorted_keys {
            let names = &buckets[key];
            writeln!(f, "        {} => &[", key).unwrap();
            for name in names {
                writeln!(f, "            {:?},", name).unwrap();
            }
            writeln!(f, "        ],").unwrap();
        }
        writeln!(f, "        _ => &[],").unwrap();
        writeln!(f, "    }}").unwrap();
        writeln!(f, "}}").unwrap();
        writeln!(f).unwrap();
    }

    write_bucket_match(&mut f, "impossible_by_length", &buckets);
    write_bucket_match(&mut f, "dotted_impossible_by_length", &dotted_buckets);
    write_bucket_match(&mut f, "orgams_impossible_by_length", &orgams_buckets);

    println!("cargo:rerun-if-changed=build.rs");
}

pub fn generate_orgams_directive_names() {
    generate_directive_names_file(
        STAND_ALONE_DIRECTIVE_ORGAMS,
        START_DIRECTIVE_ORGAMS,
        END_DIRECTIVE_ORGAMS,
        "orgams_directives_name_generated.rs",
        "STAND_ALONE_DIRECTIVE_ORGAMS",
        "START_DIRECTIVE_ORGAMS",
        "END_DIRECTIVE_ORGAMS"
    );
}

pub fn generate_basm_directive_names() {
    generate_directive_names_file(
        STAND_ALONE_DIRECTIVE,
        START_DIRECTIVE,
        END_DIRECTIVE,
        "basm_directives_name_generated.rs",
        "STAND_ALONE_DIRECTIVE",
        "START_DIRECTIVE",
        "END_DIRECTIVE"
    );
}

/// Helper to generate directive names file for both BASM and ORGAMS
pub fn generate_directive_names_file(
    stand_alone: &[&[u8]],
    start: &[&[u8]],
    end: &[&[u8]],
    filename: &str,
    stand_alone_array_name: &str,
    start_array_name: &str,
    end_array_name: &str
) {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join(filename);
    let file = File::create(&dest_path).unwrap();
    let mut file = BufWriter::new(file);

    let arrays = [
        (stand_alone, stand_alone_array_name),
        (start, start_array_name),
        (end, end_array_name)
    ];

    for (array, array_name) in arrays.iter() {
        writeln!(file, "pub const {}: &[&[u8]] = &[", array_name).unwrap();
        for directive in *array {
            writeln!(
                file,
                "    b\"{}\",",
                std::str::from_utf8(directive).unwrap()
            )
            .unwrap();
        }
        writeln!(file, "];").unwrap();
    }

    // Generate DOTTED_* arrays by prepending '.'
    for (array, array_name) in arrays.iter() {
        let dotted_name = format!("DOTTED_{}", array_name);
        writeln!(file, "pub const {}: &[&[u8]] = &[", dotted_name).unwrap();
        for directive in *array {
            let s = std::str::from_utf8(directive).unwrap();
            writeln!(file, "    b\".{}\",", s).unwrap();
        }
        writeln!(file, "];").unwrap();
    }
}

fn build() {
    built::write_built_file().expect("Failed to acquire build-time information");
}

fn main() {
    build_deps::rerun_if_changed_paths("assets").unwrap();
    build_deps::rerun_if_changed_paths("assets/**").unwrap();
    build_deps::rerun_if_changed_paths("assets/*.*").unwrap();
    build_deps::rerun_if_changed_paths("assets/**/*.*").unwrap();

    if !env::var("CARGO_CFG_TARGET_ARCH")
        .unwrap()
        .contains("wasm32")
    {
        build();
    }

    // Generate forbidden names at build time
    generate_forbidden_names();
    generate_basm_directive_names();
    generate_orgams_directive_names();
}
