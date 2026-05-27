use std::process::Command;
use std::sync::{Arc, LazyLock};

use cpclib_asm::assembler::Env;
use cpclib_asm::error::AssemblerError;
use cpclib_basm::{BasmError, build_args_parser, process};
use cpclib_common::itertools::Itertools;
use pretty_assertions::assert_eq;
use regex::Regex;
use test_generator::test_resources;

static LOCK: LazyLock<parking_lot::Mutex<()>> = LazyLock::new(parking_lot::Mutex::default);

fn manual_cleanup() {
    for fname in &[
        "BANK_C0.TXT",
        "BANK_C4.TXT",
        "BANK_C5.TXT",
        "BANK_C6.TXT",
        "BANK_C7.TXT",
        "good_bankset_0_0.o",
        "good_bankset_0_1.o",
        "good_bankset_0_2.o",
        "good_bankset_0_3.o",
        "good_bankset_1_0.o",
        "good_bankset_1_1.o",
        "good_bankset_1_2.o",
        "good_bankset_1_3.o",
        "good_save_txt.bin",
        "good_save_whole_inner.bin",
        "hello.bin",
        "hello.dsk",
        "hello.hfe",
        "hello1.bin",
        "hello2.bin",
        "hello3.bin",
        "lst.tmp",
        "TESTASCII.DSK"
    ] {
        let p = std::path::Path::new(fname);
        if p.exists() {
            fs_err::remove_file(p).unwrap()
        }
    }
}

fn command_for_generated_test(
    fname: &str,
    output: &str
) -> Result<(Env, Vec<Box<AssemblerError>>), BasmError> {
    let args_parser = build_args_parser();
    let args =
        args_parser.get_matches_from(["basm", "-I", "tests/asm/", "-i", "-o", output, fname]);

    process(&args, Arc::new(()))
}

#[derive(Debug)]
struct ListingBytesRow {
    logical_address: u32,
    has_remapped_physical_address: bool,
    bytes: Vec<u8>
}

#[derive(Copy, Clone)]
enum ListingKind {
    Text,
    Html
}

fn extract_byte_rows_from_text_listing(listing: &str) -> Vec<ListingBytesRow> {
    const LOGICAL_WIDTH: usize = 4;
    const PHYSICAL_WIDTH: usize = 6;
    const BYTES_WIDTH: usize = 24;

    listing
        .lines()
        .filter_map(|line| {
            let line = line.strip_prefix('>').unwrap_or(line);
            if line.len() < LOGICAL_WIDTH + 1 + PHYSICAL_WIDTH + 1 {
                return None;
            }

            let logical_field = &line[..LOGICAL_WIDTH];
            if !logical_field.chars().all(|c| c.is_ascii_hexdigit()) {
                return None;
            }

            let logical_address = u32::from_str_radix(logical_field, 16).ok()?;
            let physical_start = LOGICAL_WIDTH + 1;
            let bytes_start = physical_start + PHYSICAL_WIDTH + 1;
            let physical_field = &line[physical_start..physical_start + PHYSICAL_WIDTH];
            let bytes_end = (bytes_start + BYTES_WIDTH).min(line.len());
            let bytes_field = &line[bytes_start..bytes_end];

            let bytes = bytes_field
                .split_whitespace()
                .filter(|part| part.len() == 2 && part.chars().all(|c| c.is_ascii_hexdigit()))
                .map(|part| u8::from_str_radix(part, 16).unwrap())
                .collect::<Vec<_>>();

            if bytes.is_empty() {
                return None;
            }

            Some(ListingBytesRow {
                logical_address,
                has_remapped_physical_address: !physical_field.trim().is_empty(),
                bytes
            })
        })
        .collect()
}

fn strip_html_spaces(input: &str) -> String {
    input
        .replace("&nbsp;", " ")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

fn extract_byte_rows_from_html_listing(listing: &str) -> Vec<ListingBytesRow> {
    static ROW_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?s)<div[^>]*class=\"row[^\"]*\"[^>]*>.*?</div>"#).unwrap()
    });
    static ADDR_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<span class=\"cell addr\">\s*([0-9A-Fa-f]{4})\s*</span>"#).unwrap()
    });
    static PHYS_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<span class=\"cell phys\">(.*?)</span>"#).unwrap()
    });
    static BYTES_CELL_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?s)<span class=\"cell bytes[^\"]*\"[^>]*>(.*?)</span>"#).unwrap()
    });
    static BYTE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<span class=\"token byte\"[^>]*>\s*([0-9A-Fa-f]{2})\s*</span>"#)
            .unwrap()
    });

    ROW_RE
        .captures_iter(listing)
        .filter_map(|row_cap| {
            let row = row_cap.get(0)?.as_str();
            let logical_address = u32::from_str_radix(ADDR_RE.captures(row)?.get(1)?.as_str(), 16).ok()?;

            let physical_field = PHYS_RE
                .captures(row)
                .and_then(|cap| cap.get(1))
                .map(|m| strip_html_spaces(m.as_str()))
                .unwrap_or_default();

            let bytes_cell = BYTES_CELL_RE
                .captures(row)
                .and_then(|cap| cap.get(1))
                .map(|m| m.as_str())
                .unwrap_or("");

            let bytes = BYTE_RE
                .captures_iter(bytes_cell)
                .filter_map(|cap| cap.get(1))
                .map(|m| u8::from_str_radix(m.as_str(), 16).unwrap())
                .collect::<Vec<_>>();

            if bytes.is_empty() {
                return None;
            }

            Some(ListingBytesRow {
                logical_address,
                has_remapped_physical_address: !physical_field.is_empty(),
                bytes
            })
        })
        .collect()
}

fn reconstruct_linear_output_from_listing(listing: &str, kind: ListingKind) -> Option<Vec<u8>> {
    let rows = match kind {
        ListingKind::Text => extract_byte_rows_from_text_listing(listing),
        ListingKind::Html => extract_byte_rows_from_html_listing(listing)
    };
    if rows.is_empty() || rows.iter().any(|row| row.has_remapped_physical_address) {
        return None;
    }

    let start = rows.iter().map(|row| row.logical_address).min()?;
    let end = rows
        .iter()
        .map(|row| row.logical_address + row.bytes.len() as u32)
        .max()?;

    let mut output = vec![0; (end - start) as usize];
    for row in rows {
        let offset = (row.logical_address - start) as usize;
        output[offset..offset + row.bytes.len()].copy_from_slice(&row.bytes);
    }

    Some(output)
}

fn listing_byte_equivalence_is_meaningful(fname: &str) -> bool {
    !matches!(
        fname,
        "good_basic.asm"
            | "good_document_buildcpr.asm"
            | "good_document_list.asm"
            | "good_document_protect.asm"
            | "good_list.asm"
            | "good_str.asm"
    )
}

fn reconstructed_binary_equivalence_is_meaningful(fname: &str) -> bool {
    !matches!(
        fname,
        "good_aplib_decrunch.asm"
            | "good_aplib_fast_decrunch.asm"
            | "good_assembler_control_with_org.asm"
            | "good_bank.asm"
            | "good_bankset.asm"
            | "good_basic.asm"
            | "good_bzpack_bx0_backward_crunched_section.asm"
            | "good_bzpack_bx0_crunched_section.asm"
            | "good_bzpack_bx2_backward_crunched_section.asm"
            | "good_bzpack_bx2_crunched_section.asm"
            | "good_bzpack_ef8_backward_crunched_section.asm"
            | "good_bzpack_ef8_crunched_section.asm"
            | "good_bzpack_lzm_backward_crunched_section.asm"
            | "good_bzpack_lzm_crunched_section.asm"
            | "good_charset.asm"
            | "good_crunched_section2.asm"
            | "good_crunched_section3.asm"
            | "good_crunched_section4.asm"
            | "good_crunched_section5.asm"
            | "good_crunched_section6.asm"
            | "good_crunched_section.asm"
            | "good_crunched_section lzsa1.asm"
            | "good_document_bank.asm"
            | "good_document_buildcpr.asm"
            | "good_document_defsection.asm"
            | "good_document_even.asm"
            | "good_document_list.asm"
            | "good_document_org.asm"
            | "good_document_phase.asm"
            | "good_document_protect.asm"
            | "good_document_range.asm"
            | "good_document_rorg.asm"
            | "good_dollar.asm"
            | "good_exo_decrunch.asm"
            | "good_include5.asm"
            | "good_include.asm"
            | "good_list.asm"
            | "good_lz48_decrunch.asm"
            | "good_lz49_decrunch.asm"
            | "good_lz4_decrunch.asm"
            | "good_phase.asm"
            | "good_save_bank.asm"
            | "good_section.asm"
            | "good_shrinkler_decrunch.asm"
            | "good_write_direct.asm"
            | "good_zx0_backward_decrunch.asm"
            | "good_zx0_decrunch.asm"
    )
}

#[derive(Debug)]
struct CodeOnlyListingRow {
    logical_address: u32,
    bytes: Vec<u8>
}

fn run_basm_once_with_listing(
    fname: &str,
    output_fname: &str,
    listing_fname: &str
) -> std::process::Output {
    Command::new("../target/debug/basm")
        .args([
            "-I",
            "tests/asm/",
            "-i",
            fname,
            "-o",
            output_fname,
            "--lst",
            listing_fname
        ])
        .output()
        .expect("Unable to launch basm")
}

fn run_basm_once_with_code_only_listing(
    fname: &str,
    output_fname: &str,
    listing_fname: &str
) -> std::process::Output {
    Command::new("../target/debug/basm")
        .args([
            "-I",
            "tests/asm/",
            "-i",
            fname,
            "-o",
            output_fname,
            "--lst",
            listing_fname,
            "--lst-template",
            "{A} {C}",
            "--lst-source-mode",
            "none",
            "--lst-no-line-numbers",
            "--lst-no-physical-address"
        ])
        .output()
        .expect("Unable to launch basm")
}

fn extract_rows_from_code_only_listing(listing: &str) -> Vec<CodeOnlyListingRow> {
    listing
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let addr = parts.next()?;
            if addr.len() != 4 || !addr.chars().all(|c| c.is_ascii_hexdigit()) {
                return None;
            }

            let bytes = parts
                .take_while(|part| part.len() == 2 && part.chars().all(|c| c.is_ascii_hexdigit()))
                .map(|part| u8::from_str_radix(part, 16).unwrap())
                .collect::<Vec<_>>();

            if bytes.is_empty() {
                return None;
            }

            Some(CodeOnlyListingRow {
                logical_address: u32::from_str_radix(addr, 16).unwrap(),
                bytes
            })
        })
        .collect()
}

fn build_source_from_code_output(rows: &[CodeOnlyListingRow]) -> String {
    if rows.is_empty() {
        return String::new();
    }

    let mut lines = Vec::new();
    let mut current_addr = 0u32;

    for row in rows {
        if lines.is_empty() || current_addr != row.logical_address {
            lines.push(format!("org 0x{:X}", row.logical_address));
        }

        let db_values = row.bytes.iter().map(|b| format!("0x{b:02X}")).join(", ");
        lines.push(format!("db {db_values}"));

        current_addr = row.logical_address + row.bytes.len() as u32;
    }

    lines.push(String::new());
    lines.join("\n")
}

fn format_basm_failure(res: &std::process::Output) -> String {
    #[cfg(unix)]
    let signal = std::os::unix::process::ExitStatusExt::signal(&res.status);
    #[cfg(not(unix))]
    let signal: Option<i32> = None;

    format!(
        "status_code: {:?}\nsignal: {:?}\nstdout:\n{}\nstderr:\n{}",
        res.status.code(),
        signal,
        String::from_utf8_lossy(&res.stdout),
        String::from_utf8_lossy(&res.stderr)
    )
}

fn assemble_and_compare_listing_bytes(fname: &str, listing_kind: ListingKind) {
    let output_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let listing_stem_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let listing_stem = listing_stem_file.path().as_os_str().to_str().unwrap();
    let listing_fname = match listing_kind {
        ListingKind::Text => format!("{listing_stem}.lst"),
        ListingKind::Html => format!("{listing_stem}.html")
    };

    let res = run_basm_once_with_listing(fname, output_fname, &listing_fname);
    if !res.status.success() {
        panic!("Failure to assemble {}.\n{}", fname, format_basm_failure(&res));
    }

    let output = fs_err::read(output_fname).expect("Generated output is missing");
    let listing = fs_err::read_to_string(&listing_fname).expect("Listing is missing");
    let listed_bytes = match reconstruct_linear_output_from_listing(&listing, listing_kind) {
        Some(bytes) => bytes,
        None => return
    };

    let listing_kind_name = match listing_kind {
        ListingKind::Text => "text",
        ListingKind::Html => "html"
    };

    assert_eq!(
        output,
        listed_bytes,
        "Generated output differs from bytes shown in {listing_kind_name} listing for {fname}."
    );
}

fn assemble_and_check_reconstructed_source_from_code_listing(fname: &str) {
    let output_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let listing_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let listing_fname = listing_file.path().as_os_str().to_str().unwrap();

    let first = run_basm_once_with_code_only_listing(fname, output_fname, listing_fname);
    if !first.status.success() {
        panic!("Failure to assemble {}.\n{}", fname, format_basm_failure(&first));
    }

    let listing = fs_err::read_to_string(listing_fname).expect("Listing is missing");
    let listed_rows = extract_rows_from_code_only_listing(&listing);
    if listed_rows.is_empty() {
        return;
    }

    let reconstructed_source = build_source_from_code_output(&listed_rows);

    let reconstructed_input_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let reconstructed_input_fname = reconstructed_input_file.path().as_os_str().to_str().unwrap();
    fs_err::write(reconstructed_input_fname, reconstructed_source)
        .expect("Unable to write reconstructed source");

    let reconstructed_output_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let reconstructed_output_fname = reconstructed_output_file.path().as_os_str().to_str().unwrap();

    let second = Command::new("../target/debug/basm")
        .args([
            "-I",
            "tests/asm/",
            "-i",
            reconstructed_input_fname,
            "-o",
            reconstructed_output_fname
        ])
        .output()
        .expect("Unable to launch basm");

    if !second.status.success() {
        panic!(
            "Failure to assemble reconstructed source for {}.\n{}",
            fname,
            format_basm_failure(&second)
        );
    }

    let first_output = fs_err::read(output_fname).expect("Generated output is missing");
    let reconstructed_output =
        fs_err::read(reconstructed_output_fname).expect("Reconstructed output is missing");

    assert_eq!(
        first_output,
        reconstructed_output,
        "Reconstructed source output differs from original assembled output for {}.",
        fname
    );
}

#[test]
fn listing_contains_good_str_directives_and_escaped_quotes() {
    let _lock = LOCK.lock();
    manual_cleanup();

    let output_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let listing_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let listing_fname = listing_file.path().as_os_str().to_str().unwrap();

    let res = run_basm_once_with_listing("good_str.asm", output_fname, listing_fname);
    if !res.status.success() {
        panic!(
            "Failure to assemble good_str.asm.\n{}",
            format_basm_failure(&res)
        );
    }

    let listing = fs_err::read_to_string(listing_fname).expect("Listing is missing");

    assert!(listing.contains("ORG 0X1000"));
    assert!(listing.contains("DEFB \"HELL\""));
    assert!(listing.contains("STR \"HELLO\""));
    assert!(listing.contains("DB \"   \\\" ET VOILA\""));
    assert!(listing.contains("DB \" \\\" ET VOILA\""));
    assert!(listing.contains("DB \"\\\" ET VOILA\""));

    let code_only_listing_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let code_only_listing_fname = code_only_listing_file.path().as_os_str().to_str().unwrap();
    let res_code_only = run_basm_once_with_code_only_listing(
        "good_str.asm",
        output_fname,
        code_only_listing_fname,
    );
    if !res_code_only.status.success() {
        panic!(
            "Failure to assemble good_str.asm with code-only listing.\n{}",
            format_basm_failure(&res_code_only)
        );
    }

    let code_only_listing =
        fs_err::read_to_string(code_only_listing_fname).expect("Code-only listing is missing");
    let rows = extract_rows_from_code_only_listing(&code_only_listing);
    let str_row = rows
        .iter()
        .find(|row| row.logical_address == 0x2000)
        .expect("Missing row for STR at 0x2000 in code-only listing");
    assert_eq!(
        str_row.bytes,
        vec![0x68, 0x65, 0x6C, 0x6C, 0xEF],
        "STR bytes in code-only listing should match emitted bytes (last char with bit 7 set)."
    );
}

fn specific_test(folder: &str, fname: &str) {
    let output_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let res = Command::new("../target/debug/basm")
        .args(["-I", folder, "-i", fname, "-o", output_fname])
        .output()
        .expect("Unable to launch basm");

    if !res.status.success() {
        panic!("Failure to assemble {}.\n{}", fname, format_basm_failure(&res));
    }
}

#[test]
#[ignore]
fn test_roudoudou_generated_code() {
    let _ = fs_err::create_dir("generated_sprites");
    specific_test("tests/asm/roudoudou", "rasm_sprites.asm");
    let _ = fs_err::remove_dir("generated_sprites");
}

#[test_resources("cpclib-basm/tests/asm/warning_*.asm")]
fn expect_warning_but_success(real_fname: &str) {
    let _lock = LOCK.lock();
    manual_cleanup();

    let fname = &real_fname["cpclib-basm/tests/asm/".len()..];

    let output_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let listing_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let listing_fname = listing_file.path().as_os_str().to_str().unwrap();

    let content =
        fs_err::read_to_string(&real_fname["cpclib-basm/".len()..]).expect("Unable to read_source");

    static RE1: LazyLock<Regex> = LazyLock::new(|| Regex::new(r";.*$").unwrap());
    static RE2: LazyLock<Regex> = LazyLock::new(|| Regex::new(r":\s*:").unwrap());

    let mut content = content
        .split("\n")
        .map(|l| RE1.replace(l, "").replace('\r', ""))
        .join(":");
    while RE2.is_match(&content) {
        content = RE2.replace_all(&content, ":").to_string();
    }

    let content = if let Some(stripped) = content.strip_prefix(':') {
        stripped
    }
    else {
        &content[..]
    };

    let content = if let Some(':') = content.chars().last() {
        &content[..content.len() - 1]
    }
    else {
        content
    };

    let content = content.replace("\\:", "");

    if !content.is_empty() {
        let input_file =
            camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
        let input_fname = input_file.path().as_os_str().to_str().unwrap();
        fs_err::write(input_fname, content).unwrap();

        let res = Command::new("../target/debug/basm")
            .args([
                "-I",
                "tests/asm/",
                "-i",
                input_fname,
                "-o",
                output_fname,
                "--lst",
                listing_fname
            ])
            .output()
            .expect("Unable to launch basm");

        if !res.status.success() {
            panic!(
                "Failure to assemble {}.\n{}",
                fname,
                String::from_utf8_lossy(&res.stderr)
            );
        }

        let stderr = std::str::from_utf8(&res.stderr).unwrap();
        if !strip_ansi_escapes::strip_str(stderr).contains("warning: ") {
            panic!("No warning have been generated");
        }
    }
}

#[test_resources("cpclib-basm/tests/asm/good_*.asm")]
fn expect_one_line_success(real_fname: &str) {
    if real_fname.contains("basic") // basic cannot be inlined 
    || real_fname.contains("good_module.asm")
    // there are labels with ::
    || real_fname.contains("good_opcode.asm")
    // opcode() with multiline cannot be represented as single line with :
    {
        return;
    }
    let _lock = LOCK.lock();
    manual_cleanup();

    manual_cleanup();

    let fname = &real_fname["cpclib-basm/tests/asm/".len()..];

    let output_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let listing_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let listing_fname = listing_file.path().as_os_str().to_str().unwrap();

    let content =
        fs_err::read_to_string(&real_fname["cpclib-basm/".len()..]).expect("Unable to read_source");

    static RE1: LazyLock<Regex> = LazyLock::new(|| Regex::new(r";.*$").unwrap());
    static RE2: LazyLock<Regex> = LazyLock::new(|| Regex::new(r":\s*:").unwrap());

    let mut content = content
        .split("\n")
        .map(|l| RE1.replace(l, "").replace('\r', ""))
        .join(":");
    while RE2.is_match(&content) {
        content = RE2.replace_all(&content, ":").to_string();
    }

    let content = if let Some(stripped) = content.strip_prefix(':') {
        stripped
    }
    else {
        &content[..]
    };

    let content = if let Some(':') = content.chars().last() {
        &content[..content.len() - 1]
    }
    else {
        content
    };

    let content = content.replace("\\:", "");

    if !content.is_empty() {
        let input_file =
            camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
        let input_fname = input_file.path().as_os_str().to_str().unwrap();
        fs_err::write(input_fname, content).unwrap();

        let res = Command::new("../target/debug/basm")
            .args([
                "-I",
                "tests/asm/",
                "-i",
                input_fname,
                "-o",
                output_fname,
                "--lst",
                listing_fname
            ])
            .output()
            .expect("Unable to launch basm");

        if !res.status.success() {
            panic!(
                "Failure to assemble {}.\n{}",
                fname,
                String::from_utf8_lossy(&res.stderr)
            );
        }
    }
}

#[test_resources("cpclib-basm/tests/asm/good_*.asm")]
fn expect_several_empty_lines_success(real_fname: &str) {
    if real_fname.contains("basic") {
        return;
    }
    let _lock = LOCK.lock();

    manual_cleanup();

    let fname = &real_fname["cpclib-basm/tests/asm/".len()..];

    let output_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let listing_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let listing_fname = listing_file.path().as_os_str().to_str().unwrap();

    let content =
        fs_err::read_to_string(&real_fname["cpclib-basm/".len()..]).expect("Unable to read_source");

    static RE1: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)([^\\])\n").unwrap());
    static RE2: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)\\\n").unwrap());

    let content = content.replace("\r", "");
    let content = RE1.replace_all(&content, "$1\n\n\n");
    let content = RE2.replace_all(&content, "\\\n\\\n\\\n");
    let content = content.as_ref();

    // Optional verbose dump: set BASM_TEST_DEBUG=1 to enable
    if std::env::var("BASM_TEST_DEBUG").is_ok() {
        eprintln!("{}", &content);
    }

    let input_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let input_fname = input_file.path().as_os_str().to_str().unwrap();
    fs_err::write(input_fname, content).unwrap();

    let res = Command::new("../target/debug/basm")
        .args([
            "-I",
            "tests/asm/",
            "-i",
            input_fname,
            "-o",
            output_fname,
            "--lst",
            listing_fname
        ])
        .output()
        .expect("Unable to launch basm");

    if !res.status.success() {
        panic!(
            "Failure to assemble {}.\n{}",
            fname,
            String::from_utf8_lossy(&res.stderr)
        );
    }
}

#[test_resources("cpclib-basm/tests/asm/good_*.asm")]
/// TODO write tests specifics for this purpose
fn expect_listing_success(fname: &str) {
    let fname = &fname["cpclib-basm/tests/asm/".len()..];
    let _lock = LOCK.lock();

    manual_cleanup();

    let output_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let listing_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let listing_fname = listing_file.path().as_os_str().to_str().unwrap();

    let res = run_basm_once_with_listing(fname, output_fname, listing_fname);
    if !res.status.success() {
        panic!("Failure to assemble {}.\n{}", fname, format_basm_failure(&res));
    }
}

#[test_resources("cpclib-basm/tests/asm/good_*.asm")]
fn expect_listing_bytes_match_generated_output(fname: &str) {
    let _lock = LOCK.lock();

    manual_cleanup();

    let fname = &fname["cpclib-basm/tests/asm/".len()..];
    if !listing_byte_equivalence_is_meaningful(fname) {
        return;
    }

    assemble_and_compare_listing_bytes(fname, ListingKind::Text);
}

#[test_resources("cpclib-basm/tests/asm/good_*.asm")]
fn expect_html_listing_bytes_match_generated_output(fname: &str) {
    let _lock = LOCK.lock();

    manual_cleanup();

    let fname = &fname["cpclib-basm/tests/asm/".len()..];
    if !listing_byte_equivalence_is_meaningful(fname) {
        return;
    }

    assemble_and_compare_listing_bytes(fname, ListingKind::Html);
}

#[test_resources("cpclib-basm/tests/asm/good_*.asm")]
fn expect_reconstructed_source_from_code_listing_assembles(fname: &str) {
    let _lock = LOCK.lock();

    manual_cleanup();

    let fname = &fname["cpclib-basm/tests/asm/".len()..];
    if !reconstructed_binary_equivalence_is_meaningful(fname) {
        return;
    }

    assemble_and_check_reconstructed_source_from_code_listing(fname);
}

//#[test_resources("basm/tests/asm/good_*.sym")]
/// TODO write tests specifics for this purpose
fn expect_symbols_success(fname: &str) {
    let _lock = LOCK.lock();

    manual_cleanup();

    let sym_gt = &fname["cpclib-basm/tests/asm/".len()..];
    let fname = sym_gt.replace(".sym", ".asm");

    let output_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let symbol_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let symbol_fname = symbol_file.path().as_os_str().to_str().unwrap();

    let res = Command::new("../target/debug/basm")
        .args([
            "-I",
            "tests/asm/",
            "-i",
            fname.as_str(),
            "-o",
            output_fname,
            "--sym",
            symbol_fname
        ])
        .output()
        .expect("Unable to launch basm");

    if !res.status.success() {
        panic!(
            "Failure to assemble {}.\n{}",
            fname,
            String::from_utf8_lossy(&res.stderr)
        );
    }

    let sym_gt = fs_err::read_to_string(fname).unwrap();
    let sym = fs_err::read_to_string(symbol_fname).expect("Symbols not generated");

    assert_eq!(sym_gt, sym, "Symbols differ.");
}

#[test_resources("cpclib-basm/tests/asm/good_*.asm")]
fn expect_success(fname: &str) {
    let _lock = LOCK.lock();

    manual_cleanup();

    // Optional verbose: set BASM_TEST_DEBUG=1 to print file under test
    if std::env::var("BASM_TEST_DEBUG").is_ok() {
        eprintln!("{}", fname);
    }

    let fname = &fname["cpclib-basm/tests/asm/".len()..];

    let output_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let res = command_for_generated_test(fname, output_fname);
    if res.is_ok() {
        // TODO - add additional checks
        let equiv_fname = fname.replace(".asm", ".equiv");
        if std::path::Path::new("tests/asm/")
            .join(std::path::Path::new(&equiv_fname))
            .exists()
        {
            // control with an equivalent file
            let equiv_output_file =
                camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
            let equiv_output_fname = equiv_output_file.path().as_os_str().to_str().unwrap();

            let res_equiv = command_for_generated_test(&equiv_fname, equiv_output_fname);
            if res_equiv.is_err() {
                eprintln!(
                    "Error while assembling the equivalent file.\n{}",
                    res.err().unwrap()
                );
                panic!()
            }

            let output_content = fs_err::read(output_fname).unwrap();
            let equiv_output_content = fs_err::read(equiv_output_fname).unwrap();
            assert_eq!(
                output_content, equiv_output_content,
                "Content differ between {} and {}.",
                fname, equiv_fname
            );
        }
    }
    else {
        eprintln!("Error when assembling {}:\n{}", fname, res.err().unwrap());
        panic!()
    }
}

#[test_resources("cpclib-basm/tests/asm/bad_*.asm")]
fn expect_failure(fname: &str) {
    let _lock = LOCK.lock();

    manual_cleanup();

    let fname = &fname["cpclib-basm/tests/asm/".len()..];

    let output_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let res = command_for_generated_test(fname, output_fname);
    if res.is_err() {
        let msg = res.err().unwrap().to_string();

        if msg.contains("[Invalid file name]") {
            panic!("There is a memory issue there...{}", msg)
        }
    }
    else {
        eprintln!("Error when assembling {}. Wrong success:\n", fname);
        panic!();
    }
}

#[test]
fn test_at2_akm() {
    let args_parser = build_args_parser();
    let args = args_parser.get_matches_from(["basm", "--db", "tests/asm/at2/test_akm.asm"]);

    process(&args, Arc::new(())).expect("Error while assembling AT2/AKM");
}

#[test]
fn test_output_directive() {
    let _lock = LOCK.lock();
    manual_cleanup();

    // Clean up any pre-existing testoutput.bin
    let output_path = std::path::Path::new("testoutput.bin");
    if output_path.exists() {
        fs_err::remove_file(output_path).unwrap();
    }

    let fname = "tests/asm/good_document_output.asm";

    // Use basm command directly to assemble the file
    let res = Command::new("../target/debug/basm")
        .args(["-I", "tests/asm/", "-i", fname])
        .output()
        .expect("Unable to launch basm");

    assert!(
        res.status.success(),
        "Assembly failed: {}",
        String::from_utf8_lossy(&res.stderr)
    );

    // Check that the OUTPUT directive created the file
    assert!(
        output_path.exists(),
        "OUTPUT directive did not create testoutput.bin file"
    );

    // Verify the file has content (should have assembled code)
    let file_content = fs_err::read(output_path).expect("Unable to read testoutput.bin");
    assert!(
        !file_content.is_empty(),
        "testoutput.bin should not be empty"
    );

    // Clean up
    fs_err::remove_file(output_path).unwrap();
}

#[test]
fn test_output_directive_with_command_line() {
    let _lock = LOCK.lock();
    manual_cleanup();

    // Clean up any pre-existing files
    let directive_path = std::path::Path::new("testoutput.bin");
    let cmdline_path = std::path::Path::new("cmdline_output.bin");

    if directive_path.exists() {
        fs_err::remove_file(directive_path).unwrap();
    }
    if cmdline_path.exists() {
        fs_err::remove_file(cmdline_path).unwrap();
    }

    let fname = "tests/asm/good_document_output.asm";

    // Use basm with both OUTPUT directive and -o command-line argument
    let res = Command::new("../target/debug/basm")
        .args(["-I", "tests/asm/", "-i", fname, "-o", "cmdline_output.bin"])
        .output()
        .expect("Unable to launch basm");

    assert!(
        res.status.success(),
        "Assembly failed: {}",
        String::from_utf8_lossy(&res.stderr)
    );

    // Check that BOTH files were created
    assert!(
        directive_path.exists(),
        "OUTPUT directive file (testoutput.bin) was not created"
    );
    assert!(
        cmdline_path.exists(),
        "Command-line output file (cmdline_output.bin) was not created"
    );

    // Verify both files have the same content
    let directive_content = fs_err::read(directive_path).expect("Unable to read testoutput.bin");
    let cmdline_content = fs_err::read(cmdline_path).expect("Unable to read cmdline_output.bin");

    assert_eq!(
        directive_content, cmdline_content,
        "Both output files should have the same content"
    );
    assert!(
        !directive_content.is_empty(),
        "Output files should not be empty"
    );

    // Clean up
    fs_err::remove_file(directive_path).unwrap();
    fs_err::remove_file(cmdline_path).unwrap();
}
