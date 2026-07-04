use std::{env, fs, io::Write, path::Path};

fn main() {
    println!("cargo:rerun-if-changed=data/timings.txt");
    println!("cargo:rerun-if-changed=../docs/basm/directives.md");
    generate_directive_docs();
    generate_timings();
}

fn generate_directive_docs() {
    let md_src = match fs::read_to_string("../docs/basm/directives.md") {
        Ok(s) => s,
        Err(_) => {
            // Docs not present (e.g. when building outside the full workspace).
            // Emit an empty table so the code still compiles.
            let out_dir = env::var("OUT_DIR").unwrap();
            let dest = Path::new(&out_dir).join("directive_docs_generated.rs");
            fs::write(dest, "pub static DIRECTIVE_DOCS: &[(&[&str], &str)] = &[];\n").unwrap();
            return;
        }
    };

    let entries = parse_directive_md(&md_src);

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("directive_docs_generated.rs");
    let mut out = fs::File::create(dest).unwrap();

    writeln!(out, "// Auto-generated from docs/basm/directives.md — do not edit").unwrap();
    writeln!(out, "pub static DIRECTIVE_DOCS: &[(&[&str], &str)] = &[").unwrap();

    for (names, doc) in &entries {
        let names_lit: Vec<String> = names.iter()
            .map(|n| format!("\"{}\"", esc_str(n)))
            .collect();
        writeln!(out, "    (&[{}], \"{}\"),", names_lit.join(", "), esc_str(doc)).unwrap();
    }

    writeln!(out, "];").unwrap();
}

/// Parse the directives.md and return `(names, hover_markdown)` pairs.
fn parse_directive_md(md: &str) -> Vec<(Vec<String>, String)> {
    #[derive(PartialEq)]
    enum Sec { None, Synopsis, Desc, Example }

    let mut result: Vec<(Vec<String>, String)> = Vec::new();
    let mut names:  Vec<String>  = Vec::new();
    let mut syn:    String       = String::new();
    let mut desc:   String       = String::new();
    let mut sec:    Sec          = Sec::None;
    let mut in_code = false;

    let flush = |names: &mut Vec<String>, syn: &mut String, desc: &mut String,
                 result: &mut Vec<(Vec<String>, String)>| {
        if names.is_empty() { return; }
        let header = names.join(" / ");
        let mut doc = format!("**{header}**");
        let s = syn.trim();
        if !s.is_empty() {
            doc.push_str(&format!("\n\n**Synopsis:**\n```\n{s}\n```"));
        }
        let d = desc.trim();
        if !d.is_empty() {
            doc.push_str(&format!("\n\n{d}"));
        }
        result.push((std::mem::take(names), doc));
        syn.clear();
        desc.clear();
    };

    for line in md.lines() {
        if line.starts_with("### ") {
            flush(&mut names, &mut syn, &mut desc, &mut result);
            in_code = false;
            sec = Sec::None;
            names = line[4..].split(',')
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty())
                .collect();
        } else if line.starts_with("## ") || line.starts_with("# ") || line.starts_with("#### ") {
            // Category or sub-header — don't reset the current entry; ignore.
        } else if names.is_empty() {
            // Before the first ### entry
        } else if line.trim() == "Synopsis:" {
            sec = Sec::Synopsis; in_code = false;
        } else if line.trim() == "Description:" {
            sec = Sec::Desc; in_code = false;
        } else if line.trim() == "Example:" {
            sec = Sec::Example;
        } else {
            match sec {
                Sec::Synopsis => {
                    if line.trim_start().starts_with("```") {
                        in_code = !in_code;
                    } else if in_code {
                        if !syn.is_empty() { syn.push('\n'); }
                        syn.push_str(line);
                    }
                }
                Sec::Desc => {
                    // Skip mkdocs-style file includes (appear in Example blocks,
                    // but guard here just in case).
                    if line.trim().starts_with("--8<--") { continue; }
                    if line.trim_start().starts_with("```") {
                        in_code = !in_code;
                        if !desc.is_empty() { desc.push('\n'); }
                        desc.push_str(line);
                    } else if in_code && line.trim().starts_with("--8<--") {
                        // skip include lines inside code blocks too
                    } else {
                        if !desc.is_empty() { desc.push('\n'); }
                        desc.push_str(line);
                    }
                }
                Sec::Example | Sec::None => {}
            }
        }
    }
    flush(&mut names, &mut syn, &mut desc, &mut result);
    result
}

fn esc_str(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('"',  "\\\"")
     .replace('\n', "\\n")
     .replace('\r', "")
}

fn generate_timings() {

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

    emit_injected_alu_variants(&mut out);

    writeln!(out, "];").unwrap();
}

/// The source table only lists the register form `r` for adc/sub/sbc/and/or/xor/cp.
/// The (hl), (ix+n), and n addressing modes follow the same flag rules and timing
/// as the corresponding `add` variants. Opcodes are verified against the assembler.
///
/// NOPs:  r → 1,  (hl) → 2,  (ix+n) → 5,  n → 2   (identical to add).
/// Note:  the source table has `or r` and `xor r` bit-patterns swapped; the values
///        used here are the correct Z80 encodings from the assembler source.
fn emit_injected_alu_variants(out: &mut std::fs::File) {
    // (pattern, opcodes, flags_8char, nops, notes)
    const EXTRA: &[(&str, &str, &str, u8, &str)] = &[
        // ADC — flags identical to ADD
        ("adc (hl)",   "10001110",      "SZ5H3V0C", 2, "A := A + [(HL) + Carry]"),
        ("adc (ix+n)", "DD 10001110 n", "SZ5H3V0C", 5, "A := A + [(IX+n) + Carry]"),
        ("adc n",      "11001110 n",    "SZ5H3V0C", 2, "A := A + [n + Carry]"),
        // SUB
        ("sub (hl)",   "10010110",      "SZ5H3V1C", 2, ""),
        ("sub (ix+n)", "DD 10010110 n", "SZ5H3V1C", 5, ""),
        ("sub n",      "11010110 n",    "SZ5H3V1C", 2, ""),
        // SBC — flags identical to SUB
        ("sbc (hl)",   "10011110",      "SZ5H3V1C", 2, "A := A - [(HL) + Carry]"),
        ("sbc (ix+n)", "DD 10011110 n", "SZ5H3V1C", 5, "A := A - [(IX+n) + Carry]"),
        ("sbc n",      "11011110 n",    "SZ5H3V1C", 2, "A := A - [n + Carry]"),
        // AND (H always set)
        ("and (hl)",   "10100110",      "SZ513P00",  2, ""),
        ("and (ix+n)", "DD 10100110 n", "SZ513P00",  5, ""),
        ("and n",      "11100110 n",    "SZ513P00",  2, ""),
        // OR (H always reset) — opcode 0xB6/0xF6; table's `or r` bit-pattern is transposed
        ("or (hl)",    "10110110",      "SZ503P00",  2, ""),
        ("or (ix+n)",  "DD 10110110 n", "SZ503P00",  5, ""),
        ("or n",       "11110110 n",    "SZ503P00",  2, ""),
        // XOR (H always reset) — opcode 0xAE/0xEE
        ("xor (hl)",   "10101110",      "SZ503P00",  2, ""),
        ("xor (ix+n)", "DD 10101110 n", "SZ503P00",  5, ""),
        ("xor n",      "11101110 n",    "SZ503P00",  2, ""),
        // CP — flags like sub
        ("cp (hl)",    "10111110",      "SZ5H3V1C", 2, "Flags like sub"),
        ("cp (ix+n)",  "DD 10111110 n", "SZ5H3V1C", 5, "Flags like sub"),
        ("cp n",       "11111110 n",    "SZ5H3V1C", 2, "Flags like sub"),
    ];

    writeln!(out, "    // --- Injected: (hl)/(ix+n)/n variants for 8-bit ALU (adc/sub/sbc/and/or/xor/cp) ---").unwrap();
    for (pattern, opcodes, flags, nops, notes) in EXTRA {
        let mnemonic = pattern.split_whitespace().next().unwrap_or("").to_uppercase();
        writeln!(
            out,
            "    TimingEntry {{ mnemonic: \"{}\", pattern: \"{}\", opcodes: \"{}\", flags: \"{}\", nops: {}u8, nops_alt: None, notes: \"{}\" }},",
            esc(&mnemonic), esc(pattern), esc(opcodes), esc(flags), nops, esc(notes)
        ).unwrap();
    }
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
