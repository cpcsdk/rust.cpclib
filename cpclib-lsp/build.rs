use std::io::Write;
use std::path::Path;
use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=data/timings.txt");
    println!("cargo:rerun-if-changed=../docs/basm/directives.md");
    println!("cargo:rerun-if-changed=../cpclib-lsp-zed/snippets/basm.json");
    generate_directive_docs();
    generate_timings();
    generate_instr_forms();
    generate_snippets();
}

/// Generate the table of valid instruction forms (mnemonic + operand
/// patterns) from `data/timings.txt`. Used by completion to filter out
/// impossible operand combinations (e.g. the second `LD` argument depends on
/// the first).
fn generate_instr_forms() {
    let src = fs::read_to_string("data/timings.txt").expect("cannot read data/timings.txt");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("instr_forms_generated.rs");
    let mut out = fs::File::create(dest).unwrap();

    writeln!(out, "// Auto-generated from data/timings.txt — do not edit").unwrap();
    writeln!(
        out,
        "/// Valid instruction forms: (MNEMONIC, operand patterns)."
    )
    .unwrap();
    writeln!(
        out,
        "pub static INSTR_FORMS: &[(&'static str, &'static [&'static str])] = &["
    )
    .unwrap();

    let mut emit_form = |pattern: &str| {
        let pattern = pattern.trim();
        let (mnemonic, rest) = match pattern.split_once(char::is_whitespace) {
            Some((m, r)) => (m, r.trim()),
            None => (pattern, "")
        };
        if mnemonic.is_empty() || !mnemonic.starts_with(|c: char| c.is_ascii_alphabetic()) {
            return;
        }
        let operands: Vec<String> = if rest.is_empty() {
            Vec::new()
        }
        else {
            split_operands(rest)
                .into_iter()
                .map(|o| o.trim().to_lowercase())
                .collect()
        };
        let ops_lit: Vec<String> = operands.iter().map(|o| format!("\"{}\"", esc(o))).collect();
        writeln!(
            out,
            "    (\"{}\", &[{}]),",
            esc(&mnemonic.to_uppercase()),
            ops_lit.join(", ")
        )
        .unwrap();
    };

    for raw_line in src.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() < 4 {
            continue;
        }
        let pattern = parts[0].trim();
        let flags = parts[2].trim();
        if !pattern.starts_with(|c: char| c.is_ascii_alphabetic()) || flags.len() != 8 {
            continue;
        }
        emit_form(pattern);
    }

    // Same injected ALU variants as the timing table.
    for pattern in [
        "adc (hl)",
        "adc (ix+n)",
        "adc n",
        "sub (hl)",
        "sub (ix+n)",
        "sub n",
        "sbc (hl)",
        "sbc (ix+n)",
        "sbc n",
        "and (hl)",
        "and (ix+n)",
        "and n",
        "or (hl)",
        "or (ix+n)",
        "or n",
        "xor (hl)",
        "xor (ix+n)",
        "xor n",
        "cp (hl)",
        "cp (ix+n)",
        "cp n"
    ] {
        emit_form(pattern);
    }

    writeln!(out, "];").unwrap();
}

/// Split an operand list at top-level commas (commas inside parentheses do
/// not split — e.g. there are none in practice, but stay safe).
fn split_operands(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, b) in s.bytes().enumerate() {
        match b {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b',' if depth <= 0 => {
                parts.push(&s[start..i]);
                start = i + 1;
            },
            _ => {}
        }
    }
    parts.push(&s[start..]);
    parts
}

/// Generate completion snippets from the Zed extension's snippet file, so the
/// same file can be reused in other contexts.
fn generate_snippets() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("snippets_generated.rs");

    let empty = "pub static SNIPPETS: &[(&str, &str, &str)] = &[];\n";
    let src = match fs::read_to_string("../cpclib-lsp-zed/snippets/basm.json") {
        Ok(s) => s,
        Err(_) => {
            fs::write(dest, empty).unwrap();
            return;
        }
    };
    let json: serde_json::Value = match serde_json::from_str(&src) {
        Ok(v) => v,
        Err(e) => {
            println!("cargo:warning=cannot parse basm.json snippets: {e}");
            fs::write(dest, empty).unwrap();
            return;
        }
    };

    let mut out = fs::File::create(dest).unwrap();
    writeln!(
        out,
        "// Auto-generated from cpclib-lsp-zed/snippets/basm.json — do not edit"
    )
    .unwrap();
    writeln!(
        out,
        "/// (prefix, description, snippet body in LSP snippet syntax)"
    )
    .unwrap();
    writeln!(out, "pub static SNIPPETS: &[(&str, &str, &str)] = &[").unwrap();

    if let Some(map) = json.as_object() {
        for (name, snip) in map {
            let prefix = snip.get("prefix").and_then(|v| v.as_str()).unwrap_or("");
            if prefix.is_empty() {
                continue;
            }
            let description = snip
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or(name);
            let body = match snip.get("body") {
                Some(serde_json::Value::String(s)) => s.clone(),
                Some(serde_json::Value::Array(lines)) => {
                    lines
                        .iter()
                        .filter_map(|l| l.as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                },
                _ => continue
            };
            writeln!(
                out,
                "    (\"{}\", \"{}\", \"{}\"),",
                esc_str(prefix),
                esc_str(description),
                esc_str(&body)
            )
            .unwrap();
        }
    }

    writeln!(out, "];").unwrap();
}

fn generate_directive_docs() {
    let md_src = match fs::read_to_string("../docs/basm/directives.md") {
        Ok(s) => s,
        Err(_) => {
            // Docs not present (e.g. when building outside the full workspace).
            // Emit an empty table so the code still compiles.
            let out_dir = env::var("OUT_DIR").unwrap();
            let dest = Path::new(&out_dir).join("directive_docs_generated.rs");
            fs::write(
                dest,
                "pub static DIRECTIVE_DOCS: &[(&[&str], &str)] = &[];\n\
                 pub static DIRECTIVE_FILE_ARGS: &[&str] = &[];\n"
            )
            .unwrap();
            return;
        }
    };

    let entries = parse_directive_md(&md_src);

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("directive_docs_generated.rs");
    let mut out = fs::File::create(dest).unwrap();

    writeln!(
        out,
        "// Auto-generated from docs/basm/directives.md — do not edit"
    )
    .unwrap();
    writeln!(out, "pub static DIRECTIVE_DOCS: &[(&[&str], &str)] = &[").unwrap();

    for (names, doc, _) in &entries {
        let names_lit: Vec<String> = names
            .iter()
            .map(|n| format!("\"{}\"", esc_str(n)))
            .collect();
        writeln!(
            out,
            "    (&[{}], \"{}\"),",
            names_lit.join(", "),
            esc_str(doc)
        )
        .unwrap();
    }

    writeln!(out, "];").unwrap();

    // Directives whose synopsis takes a quoted filename argument (INCLUDE,
    // INCBIN, SAVE, ...): completion offers filenames-in-strings for these.
    writeln!(out).unwrap();
    writeln!(
        out,
        "/// Directive names (uppercase) whose argument is a quoted filename."
    )
    .unwrap();
    writeln!(out, "pub static DIRECTIVE_FILE_ARGS: &[&str] = &[").unwrap();
    for (names, _, has_file_arg) in &entries {
        if !has_file_arg {
            continue;
        }
        for n in names {
            writeln!(out, "    \"{}\",", esc_str(&n.to_uppercase())).unwrap();
        }
    }
    writeln!(out, "];").unwrap();
}

/// Returns true when a directive synopsis takes a quoted filename argument.
fn synopsis_has_file_arg(syn: &str) -> bool {
    let lower = syn.to_lowercase();
    [
        "\"fname",
        "\"<fname",
        "\"filename",
        "\"template",
        ".sna\"",
        ".cpr\"",
        ".dsk\""
    ]
    .iter()
    .any(|m| lower.contains(m))
}

/// Parse the directives.md and return `(names, hover_markdown, has_file_arg)` tuples.
fn parse_directive_md(md: &str) -> Vec<(Vec<String>, String, bool)> {
    #[derive(PartialEq)]
    enum Sec {
        None,
        Synopsis,
        Desc,
        Example
    }

    let mut result: Vec<(Vec<String>, String, bool)> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let mut syn: String = String::new();
    let mut desc: String = String::new();
    let mut sec: Sec = Sec::None;
    let mut in_code = false;

    let flush = |names: &mut Vec<String>,
                 syn: &mut String,
                 desc: &mut String,
                 result: &mut Vec<(Vec<String>, String, bool)>| {
        if names.is_empty() {
            return;
        }
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
        let has_file_arg = synopsis_has_file_arg(syn);
        result.push((std::mem::take(names), doc, has_file_arg));
        syn.clear();
        desc.clear();
    };

    for line in md.lines() {
        if line.starts_with("### ") {
            flush(&mut names, &mut syn, &mut desc, &mut result);
            in_code = false;
            sec = Sec::None;
            names = line[4..]
                .split(',')
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty())
                .collect();
        }
        else if line.starts_with("## ") || line.starts_with("# ") || line.starts_with("#### ") {
            // Category or sub-header — don't reset the current entry; ignore.
        }
        else if names.is_empty() {
            // Before the first ### entry
        }
        else if line.trim() == "Synopsis:" {
            sec = Sec::Synopsis;
            in_code = false;
        }
        else if line.trim() == "Description:" {
            sec = Sec::Desc;
            in_code = false;
        }
        else if line.trim() == "Example:" {
            sec = Sec::Example;
        }
        else {
            match sec {
                Sec::Synopsis => {
                    if line.trim_start().starts_with("```") {
                        in_code = !in_code;
                    }
                    else if in_code {
                        if !syn.is_empty() {
                            syn.push('\n');
                        }
                        syn.push_str(line);
                    }
                },
                Sec::Desc => {
                    // Skip mkdocs-style file includes (appear in Example blocks,
                    // but guard here just in case).
                    if line.trim().starts_with("--8<--") {
                        continue;
                    }
                    if line.trim_start().starts_with("```") {
                        in_code = !in_code;
                        if !desc.is_empty() {
                            desc.push('\n');
                        }
                        desc.push_str(line);
                    }
                    else if in_code && line.trim().starts_with("--8<--") {
                        // skip include lines inside code blocks too
                    }
                    else {
                        if !desc.is_empty() {
                            desc.push('\n');
                        }
                        desc.push_str(line);
                    }
                },
                Sec::Example | Sec::None => {}
            }
        }
    }
    flush(&mut names, &mut syn, &mut desc, &mut result);
    result
}

fn esc_str(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "")
}

fn generate_timings() {
    let src = fs::read_to_string("data/timings.txt").expect("cannot read data/timings.txt");

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
        let pattern = parts[0].trim();
        let opcodes = parts[1].trim();
        let flags = parts[2].trim();
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
            None => (nops_tail.trim(), "")
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
            None => "None".to_string()
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
        (
            "adc (hl)",
            "10001110",
            "SZ5H3V0C",
            2,
            "A := A + [(HL) + Carry]"
        ),
        (
            "adc (ix+n)",
            "DD 10001110 n",
            "SZ5H3V0C",
            5,
            "A := A + [(IX+n) + Carry]"
        ),
        ("adc n", "11001110 n", "SZ5H3V0C", 2, "A := A + [n + Carry]"),
        // SUB
        ("sub (hl)", "10010110", "SZ5H3V1C", 2, ""),
        ("sub (ix+n)", "DD 10010110 n", "SZ5H3V1C", 5, ""),
        ("sub n", "11010110 n", "SZ5H3V1C", 2, ""),
        // SBC — flags identical to SUB
        (
            "sbc (hl)",
            "10011110",
            "SZ5H3V1C",
            2,
            "A := A - [(HL) + Carry]"
        ),
        (
            "sbc (ix+n)",
            "DD 10011110 n",
            "SZ5H3V1C",
            5,
            "A := A - [(IX+n) + Carry]"
        ),
        ("sbc n", "11011110 n", "SZ5H3V1C", 2, "A := A - [n + Carry]"),
        // AND (H always set)
        ("and (hl)", "10100110", "SZ513P00", 2, ""),
        ("and (ix+n)", "DD 10100110 n", "SZ513P00", 5, ""),
        ("and n", "11100110 n", "SZ513P00", 2, ""),
        // OR (H always reset) — opcode 0xB6/0xF6; table's `or r` bit-pattern is transposed
        ("or (hl)", "10110110", "SZ503P00", 2, ""),
        ("or (ix+n)", "DD 10110110 n", "SZ503P00", 5, ""),
        ("or n", "11110110 n", "SZ503P00", 2, ""),
        // XOR (H always reset) — opcode 0xAE/0xEE
        ("xor (hl)", "10101110", "SZ503P00", 2, ""),
        ("xor (ix+n)", "DD 10101110 n", "SZ503P00", 5, ""),
        ("xor n", "11101110 n", "SZ503P00", 2, ""),
        // CP — flags like sub
        ("cp (hl)", "10111110", "SZ5H3V1C", 2, "Flags like sub"),
        (
            "cp (ix+n)",
            "DD 10111110 n",
            "SZ5H3V1C",
            5,
            "Flags like sub"
        ),
        ("cp n", "11111110 n", "SZ5H3V1C", 2, "Flags like sub")
    ];

    writeln!(
        out,
        "    // --- Injected: (hl)/(ix+n)/n variants for 8-bit ALU (adc/sub/sbc/and/or/xor/cp) ---"
    )
    .unwrap();
    for (pattern, opcodes, flags, nops, notes) in EXTRA {
        let mnemonic = pattern
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_uppercase();
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
    }
    else if rest.to_ascii_lowercase().starts_with("or ") {
        rest[3..].trim_start()
    }
    else {
        return (nops, None);
    };
    let alt: String = alt_start
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if alt.is_empty() {
        (nops, None)
    }
    else {
        (nops, Some(alt.parse().unwrap_or(0)))
    }
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
