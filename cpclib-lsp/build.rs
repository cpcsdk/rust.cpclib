use std::{env, fs, io::Write, path::Path};

fn main() {
    println!("cargo:rerun-if-changed=data/timings.txt");

    let src = fs::read_to_string("data/timings.txt")
        .expect("cannot read data/timings.txt");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("timings_generated.rs");
    let mut out = fs::File::create(dest).unwrap();

    writeln!(out, "// Auto-generated from data/timings.txt — do not edit").unwrap();
    writeln!(out, "#[derive(Debug, Clone)]").unwrap();
    writeln!(out, "pub struct TimingEntry {{").unwrap();
    writeln!(out, "    pub mnemonic: &'static str,").unwrap();
    writeln!(out, "    pub pattern:  &'static str,").unwrap();
    writeln!(out, "    pub opcodes:  &'static str,").unwrap();
    writeln!(out, "    pub flags:    &'static str,").unwrap();
    writeln!(out, "    pub nops:     u8,").unwrap();
    writeln!(out, "    pub nops_alt: Option<u8>,").unwrap();
    writeln!(out, "    pub notes:    &'static str,").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "pub static TIMINGS: &[TimingEntry] = &[").unwrap();

    for raw_line in src.lines() {
        let line = raw_line.trim();
        // Skip blank lines and pure comment lines
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        // Data rows have at least 3 `|` separators
        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() < 4 {
            continue;
        }
        let pattern   = parts[0].trim();
        let opcodes   = parts[1].trim();
        let flags     = parts[2].trim();
        let nops_tail = parts[3];

        // Pattern must start with a letter (mnemonic)
        if !pattern.starts_with(|c: char| c.is_ascii_alphabetic()) {
            continue;
        }
        // flags field must look like an 8-char mask
        if flags.len() != 8 {
            continue;
        }

        // Split notes from nops at first `;`
        let (nops_str, notes) = match nops_tail.find(';') {
            Some(i) => (nops_tail[..i].trim(), nops_tail[i + 1..].trim()),
            None    => (nops_tail.trim(), ""),
        };

        let (nops, nops_alt) = parse_nops(nops_str);
        if nops == 0 {
            continue; // unparseable entry
        }

        let mnemonic = pattern
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_uppercase();
        if mnemonic.is_empty() {
            continue;
        }

        let nops_alt_lit = match nops_alt {
            Some(n) => format!("Some({}u8)", n),
            None    => "None".to_string(),
        };

        writeln!(
            out,
            "    TimingEntry {{ mnemonic: \"{}\", pattern: \"{}\", opcodes: \"{}\", flags: \"{}\", nops: {}u8, nops_alt: {}, notes: \"{}\" }},",
            esc(&mnemonic), esc(pattern), esc(opcodes), esc(flags), nops, nops_alt_lit, esc(notes)
        ).unwrap();
    }

    writeln!(out, "];").unwrap();
}

fn parse_nops(s: &str) -> (u8, Option<u8>) {
    let s = s.trim();
    let first: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if first.is_empty() {
        return (0, None);
    }
    let nops: u8 = first.parse().unwrap_or(0);
    let rest = s[first.len()..].trim_start();
    let alt_start = if rest.starts_with('/') {
        rest[1..].trim_start()
    } else if rest.to_ascii_lowercase().starts_with("or ") {
        rest[3..].trim_start()
    } else {
        return (nops, None);
    };
    let alt: String = alt_start.chars().take_while(|c| c.is_ascii_digit()).collect();
    if alt.is_empty() {
        (nops, None)
    } else {
        (nops, Some(alt.parse().unwrap_or(0)))
    }
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
