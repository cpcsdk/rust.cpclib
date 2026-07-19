mod common;

use std::process::Command;
use std::sync::{Arc, LazyLock};

use common::{GLOBAL_TEST_LOCK, manual_cleanup};
use cpclib_asm::assembler::Env;
use cpclib_asm::error::AssemblerError;
use cpclib_basm::{BasmError, build_args_parser, process};
use cpclib_common::itertools::Itertools;
use pretty_assertions::assert_eq;
use regex::Regex;
use test_generator::test_resources;

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
    static ROW_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?s)<div[^>]*class=\"row[^\"]*\"[^>]*>.*?</div>"#).unwrap());
    static ADDR_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<span class=\"cell addr\">\s*([0-9A-Fa-f]{4})\s*</span>"#).unwrap()
    });
    static PHYS_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"<span class=\"cell phys\">(.*?)</span>"#).unwrap());
    static BYTES_CELL_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?s)<span class=\"cell bytes[^\"]*\"[^>]*>(.*?)</span>"#).unwrap()
    });
    static BYTE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<span class=\"token byte\"[^>]*>\s*([0-9A-Fa-f]{2})\s*</span>"#).unwrap()
    });

    ROW_RE
        .captures_iter(listing)
        .filter_map(|row_cap| {
            let row = row_cap.get(0)?.as_str();
            let logical_address =
                u32::from_str_radix(ADDR_RE.captures(row)?.get(1)?.as_str(), 16).ok()?;

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
    let _ = fname;
    true
}

fn reconstructed_binary_equivalence_is_meaningful(fname: &str) -> bool {
    let _ = fname;
    true
}

fn acquire_lock() -> parking_lot::lock_api::MutexGuard<'static, parking_lot::RawMutex, ()> {
    GLOBAL_TEST_LOCK.lock()
}

#[derive(Debug)]
struct CodeOnlyListingRow {
    logical_address: u32,
    physical_address: Option<u32>,
    bytes: Vec<u8>,
    source: Option<String>
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
            "{A} {P} {C} | {S}",
            "--lst-no-line-numbers"
        ])
        .output()
        .expect("Unable to launch basm")
}

fn extract_rows_from_code_only_listing(listing: &str) -> Vec<CodeOnlyListingRow> {
    listing
        .lines()
        .filter_map(|line| {
            let line = line.strip_prefix('>').unwrap_or(line);
            let (left, right) = line.split_once(" | ").unwrap_or((line, ""));
            let source = {
                let s = right.trim();
                if s.is_empty() {
                    None
                }
                else {
                    Some(s.to_string())
                }
            };

            let mut parts = left.split_whitespace();
            let addr = parts.next()?;
            if addr.len() != 4 || !addr.chars().all(|c| c.is_ascii_hexdigit()) {
                return None;
            }

            let mut physical_address = None;
            let mut remaining_parts = Vec::new();

            if let Some(next) = parts.next() {
                // With `{A} {P} {C}`, `{P}` is either omitted/blank or a non-byte hex address.
                if next.len() != 2 && next.chars().all(|c| c.is_ascii_hexdigit()) {
                    physical_address = u32::from_str_radix(next, 16).ok();
                }
                else {
                    remaining_parts.push(next);
                }
            }
            remaining_parts.extend(parts);

            let bytes = remaining_parts
                .iter()
                .take_while(|part| part.len() == 2 && part.chars().all(|c| c.is_ascii_hexdigit()))
                .map(|part| u8::from_str_radix(part, 16).unwrap())
                .collect::<Vec<_>>();

            let keeps_directive = source
                .as_deref()
                .map(|s| {
                    let upper = s.to_ascii_uppercase();
                    upper.starts_with("BANK ")
                        || upper.starts_with("BANKSET ")
                        || upper.starts_with("BUILDCPR")
                })
                .unwrap_or(false);

            if bytes.is_empty() && !keeps_directive {
                return None;
            }

            Some(CodeOnlyListingRow {
                logical_address: u32::from_str_radix(addr, 16).unwrap(),
                physical_address,
                bytes,
                source
            })
        })
        .collect()
}

fn extract_rows_from_html_code_only_listing(listing: &str) -> Vec<CodeOnlyListingRow> {
    static ROW_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?s)<div[^>]*class=\"row[^\"]*\"[^>]*>.*?</div>"#).unwrap());
    static ADDR_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<span class=\"cell addr\">\s*([0-9A-Fa-f]{4})\s*</span>"#).unwrap()
    });
    static PHYS_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"<span class=\"cell phys\">(.*?)</span>"#).unwrap());
    static BYTES_CELL_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?s)<span class=\"cell bytes[^\"]*\"[^>]*>(.*?)</span>"#).unwrap()
    });
    static BYTE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<span class=\"token byte\"[^>]*>\s*([0-9A-Fa-f]{2})\s*</span>"#).unwrap()
    });
    static SOURCE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?s)<span class=\"cell source\">(.*?)</span>"#).unwrap());
    static TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"<[^>]+>"#).unwrap());

    ROW_RE
        .captures_iter(listing)
        .filter_map(|row_cap| {
            let row = row_cap.get(0)?.as_str();
            let logical_address =
                u32::from_str_radix(ADDR_RE.captures(row)?.get(1)?.as_str(), 16).ok()?;

            let physical_address = PHYS_RE
                .captures(row)
                .and_then(|cap| cap.get(1))
                .map(|m| strip_html_spaces(m.as_str()))
                .filter(|s| !s.is_empty())
                .and_then(|s| u32::from_str_radix(&s, 16).ok());

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

            let source = SOURCE_RE
                .captures(row)
                .and_then(|cap| cap.get(1))
                .map(|m| m.as_str())
                .map(|src| TAG_RE.replace_all(src, "").to_string())
                .map(|src| {
                    src.replace("&nbsp;", " ")
                        .replace("&lt;", "<")
                        .replace("&gt;", ">")
                        .replace("&amp;", "&")
                })
                .map(|src| src.trim().to_string())
                .filter(|src| !src.is_empty());

            let keeps_directive = source
                .as_deref()
                .map(|s| {
                    let upper = s.to_ascii_uppercase();
                    upper.starts_with("BANK ")
                        || upper.starts_with("BANKSET ")
                        || upper.starts_with("BUILDCPR")
                })
                .unwrap_or(false);

            if bytes.is_empty() && !keeps_directive {
                return None;
            }

            Some(CodeOnlyListingRow {
                logical_address,
                physical_address,
                bytes,
                source
            })
        })
        .collect()
}

fn extract_rows_from_code_only_listing_for_kind(
    listing: &str,
    listing_kind: ListingKind
) -> Vec<CodeOnlyListingRow> {
    match listing_kind {
        ListingKind::Text => extract_rows_from_code_only_listing(listing),
        ListingKind::Html => extract_rows_from_html_code_only_listing(listing)
    }
}

fn build_source_from_code_output(rows: &[CodeOnlyListingRow]) -> String {
    if rows.is_empty() {
        return String::new();
    }

    let mut lines = Vec::new();
    let mut current_addr = 0u32;
    let mut current_bank: Option<u32> = None;
    let mut last_banked_index: Option<u32> = None;
    let mut saw_explicit_bank = false;
    let mut saw_explicit_bankset = false;
    let mut saw_buildcpr = false;
    let mut inserted_default_bankset = false;

    for row in rows {
        if let Some(source) = row.source.as_deref() {
            let trimmed = source.trim();
            let upper = trimmed.to_ascii_uppercase();
            if upper.starts_with("BANKSET ") {
                lines.push(trimmed.to_ascii_lowercase());
                saw_explicit_bankset = true;
                current_addr = u32::MAX;
                continue;
            }
            if upper.starts_with("BUILDCPR") {
                lines.push(trimmed.to_ascii_lowercase());
                saw_buildcpr = true;
                current_addr = u32::MAX;
                continue;
            }
            if upper.starts_with("BANK ") {
                let raw_bank_value = trimmed
                    .split_once(char::is_whitespace)
                    .and_then(|(_, rest)| parse_bank_value(rest.trim()));

                if !saw_buildcpr
                    && !saw_explicit_bankset
                    && let Some(value) = raw_bank_value
                    && value < 0xC0
                {
                    // In BUILDCPR mode, BANK values are cartridge bank indices
                    // (0,1,...) rather than RAM bank IDs (0xC0..0xC7).
                    lines.push("buildcpr".to_string());
                    saw_buildcpr = true;
                }

                if !saw_buildcpr && !saw_explicit_bankset && !inserted_default_bankset {
                    lines.push("bankset 0".to_string());
                    inserted_default_bankset = true;
                }

                let normalized = if saw_buildcpr {
                    trimmed.to_ascii_lowercase()
                }
                else {
                    raw_bank_value
                        .map(|value| {
                            if value < 0xC0 {
                                format!("bank 0x{:X}", 0xC0 + value)
                            }
                            else {
                                format!("bank 0x{:X}", value)
                            }
                        })
                        .unwrap_or_else(|| trimmed.to_ascii_lowercase())
                };

                lines.push(normalized);
                saw_explicit_bank = true;
                current_addr = u32::MAX;
                continue;
            }
        }

        if row.bytes.is_empty() {
            continue;
        }

        // For banked outputs, physical addresses above 64k point to bank storage
        // (`bank_index * 0x4000 + offset`). Recreate BANK directives when needed.
        if !saw_explicit_bank
            && !saw_explicit_bankset
            && let Some(physical_address) = row.physical_address
            && physical_address >= 0x1_0000
        {
            let bank_index = physical_address / 0x4000;
            last_banked_index = Some(bank_index);
            if current_bank != Some(bank_index) {
                lines.push(format!("bank 0x{:X}", 0xC0 + bank_index));
                current_bank = Some(bank_index);
                current_addr = u32::MAX;
            }
        }

        if lines.is_empty() || current_addr != row.logical_address {
            lines.push(format!("org 0x{:X}", row.logical_address));
        }

        let db_values = row.bytes.iter().map(|b| format!("0x{b:02X}")).join(", ");
        lines.push(format!("db {db_values}"));

        current_addr = row.logical_address + row.bytes.len() as u32;
    }

    if !saw_explicit_bankset && let Some(bank_index) = last_banked_index {
        lines.push(format!("bankset {}", bank_index / 4));
    }

    lines.push(String::new());
    lines.join("\n")
}

fn parse_bank_value(raw: &str) -> Option<u32> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }

    if let Some(stripped) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u32::from_str_radix(stripped, 16).ok()
    }
    else {
        value.parse::<u32>().ok()
    }
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

fn emitted_rows_signature(rows: &[CodeOnlyListingRow]) -> Vec<(u32, Vec<u8>)> {
    rows.iter()
        .filter(|row| !row.bytes.is_empty())
        .map(|row| (row.logical_address, row.bytes.clone()))
        .collect()
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
        panic!(
            "Failure to assemble {}.\n{}",
            fname,
            format_basm_failure(&res)
        );
    }

    let listing = fs_err::read_to_string(&listing_fname).expect("Listing is missing");
    let rows = match listing_kind {
        ListingKind::Text => extract_byte_rows_from_text_listing(&listing),
        ListingKind::Html => extract_byte_rows_from_html_listing(&listing)
    };

    if rows.is_empty() || rows.iter().any(|row| row.has_remapped_physical_address) {
        return;
    }

    let code_only_listing_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let code_only_listing_fname = code_only_listing_file.path().as_os_str().to_str().unwrap();

    let code_only_output_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let code_only_output_fname = code_only_output_file.path().as_os_str().to_str().unwrap();

    let res_code_only = run_basm_once_with_code_only_listing(
        fname,
        code_only_output_fname,
        code_only_listing_fname
    );
    let listing_kind_name = match listing_kind {
        ListingKind::Text => "text",
        ListingKind::Html => "html"
    };

    if !res_code_only.status.success() {
        let stderr = String::from_utf8_lossy(&res_code_only.stderr);
        let side_effect_conflict = stderr.contains("already present in disc");
        if !side_effect_conflict {
            panic!(
                "Failure to assemble {} with code-only listing.\n{}",
                fname,
                format_basm_failure(&res_code_only)
            );
        }

        // SAVE-related fixtures can fail on a second assembly due to persistent
        // external media state (e.g., files already present in DSK/CPR outputs).
        // In that case, we still verify bytes shown in listing against the first
        // assembled output bytes.
        let output = fs_err::read(output_fname).expect("Generated output is missing");
        let start = match rows.iter().map(|row| row.logical_address).min() {
            Some(addr) => addr,
            None => return
        };

        for row in rows {
            let offset = (row.logical_address - start) as usize;
            if offset + row.bytes.len() > output.len() {
                return;
            }

            assert_eq!(
                &output[offset..offset + row.bytes.len()],
                row.bytes.as_slice(),
                "Generated output differs from bytes shown in {listing_kind_name} listing for {fname} at 0x{:04X}.",
                row.logical_address
            );
        }

        return;
    }

    let code_only_listing =
        fs_err::read_to_string(code_only_listing_fname).expect("Code-only listing is missing");
    let code_only_rows = extract_rows_from_code_only_listing(&code_only_listing);

    // Compare bytes shown in the selected listing format against bytes shown in
    // code-only listing rows for the same assembly output.
    for row in rows {
        let exists_in_code_only = code_only_rows.iter().any(|candidate| {
            candidate.logical_address == row.logical_address && candidate.bytes == row.bytes
        });

        assert!(
            exists_in_code_only,
            "Bytes shown in {listing_kind_name} listing for {fname} at 0x{:04X} are missing or different in code-only listing.",
            row.logical_address
        );
    }
}

fn assemble_and_check_reconstructed_source_from_code_listing(
    fname: &str,
    listing_kind: ListingKind
) {
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

    let first = run_basm_once_with_code_only_listing(fname, output_fname, &listing_fname);
    if !first.status.success() {
        panic!(
            "Failure to assemble {}.\n{}",
            fname,
            format_basm_failure(&first)
        );
    }

    let listing = fs_err::read_to_string(&listing_fname).expect("Listing is missing");
    let listed_rows = extract_rows_from_code_only_listing_for_kind(&listing, listing_kind);
    if listed_rows.is_empty() {
        return;
    }

    let reconstructed_source = build_source_from_code_output(&listed_rows);

    let reconstructed_input_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let reconstructed_input_fname = reconstructed_input_file
        .path()
        .as_os_str()
        .to_str()
        .unwrap();
    fs_err::write(reconstructed_input_fname, reconstructed_source)
        .expect("Unable to write reconstructed source");

    let reconstructed_output_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let reconstructed_output_fname = reconstructed_output_file
        .path()
        .as_os_str()
        .to_str()
        .unwrap();

    let reconstructed_listing_stem_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let reconstructed_listing_stem = reconstructed_listing_stem_file
        .path()
        .as_os_str()
        .to_str()
        .unwrap();
    let reconstructed_listing_fname = match listing_kind {
        ListingKind::Text => format!("{reconstructed_listing_stem}.lst"),
        ListingKind::Html => format!("{reconstructed_listing_stem}.html")
    };

    let second = run_basm_once_with_code_only_listing(
        reconstructed_input_fname,
        reconstructed_output_fname,
        &reconstructed_listing_fname
    );

    if !second.status.success() {
        panic!(
            "Failure to assemble reconstructed source for {}.\n{}",
            fname,
            format_basm_failure(&second)
        );
    }

    let reconstructed_listing = fs_err::read_to_string(&reconstructed_listing_fname)
        .expect("Reconstructed listing is missing");
    let reconstructed_rows =
        extract_rows_from_code_only_listing_for_kind(&reconstructed_listing, listing_kind);

    let first_signature = emitted_rows_signature(&listed_rows);
    let reconstructed_signature = emitted_rows_signature(&reconstructed_rows);

    assert_eq!(
        first_signature, reconstructed_signature,
        "Reconstructed source emitted rows differ from original assembled rows for {}.",
        fname
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
        panic!(
            "Failure to assemble {}.\n{}",
            fname,
            format_basm_failure(&res)
        );
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
    let _lock = acquire_lock();
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
    let _lock = acquire_lock();
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
    let _lock = acquire_lock();

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
    let _lock = acquire_lock();

    manual_cleanup();

    let output_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let listing_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let listing_fname = listing_file.path().as_os_str().to_str().unwrap();

    let res = run_basm_once_with_listing(fname, output_fname, listing_fname);
    if !res.status.success() {
        panic!(
            "Failure to assemble {}.\n{}",
            fname,
            format_basm_failure(&res)
        );
    }
}

#[test_resources("cpclib-basm/tests/asm/good_*.asm")]
fn expect_html_listing_bytes_match_generated_output(fname: &str) {
    let _lock = acquire_lock();

    manual_cleanup();

    let fname = &fname["cpclib-basm/tests/asm/".len()..];
    if !listing_byte_equivalence_is_meaningful(fname) {
        return;
    }

    assemble_and_compare_listing_bytes(fname, ListingKind::Html);
}

#[test_resources("cpclib-basm/tests/asm/good_*.asm")]
fn expect_reconstructed_source_from_code_listing_assembles(fname: &str) {
    let _lock = acquire_lock();

    manual_cleanup();

    let fname = &fname["cpclib-basm/tests/asm/".len()..];
    if !reconstructed_binary_equivalence_is_meaningful(fname) {
        return;
    }

    assemble_and_check_reconstructed_source_from_code_listing(fname, ListingKind::Text);
    manual_cleanup();
    assemble_and_check_reconstructed_source_from_code_listing(fname, ListingKind::Html);
}

//#[test_resources("basm/tests/asm/good_*.sym")]
/// TODO write tests specifics for this purpose
fn expect_symbols_success(fname: &str) {
    let _lock = acquire_lock();

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
    let _lock = acquire_lock();

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
    let _lock = acquire_lock();

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
    let _lock = acquire_lock();
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
fn test_output_directive_dry_run() {
    let _lock = acquire_lock();
    manual_cleanup();

    // Clean up any pre-existing testoutput.bin
    let output_path = std::path::Path::new("testoutput.bin");
    if output_path.exists() {
        fs_err::remove_file(output_path).unwrap();
    }

    let fname = "tests/asm/good_document_output.asm";

    // Use basm command directly to assemble the file, with --dry-run
    let res = Command::new("../target/debug/basm")
        .args(["-I", "tests/asm/", "-i", fname, "--dry-run"])
        .output()
        .expect("Unable to launch basm");

    assert!(
        res.status.success(),
        "Assembly failed: {}",
        String::from_utf8_lossy(&res.stderr)
    );

    // --dry-run must prevent the OUTPUT directive from creating any file
    assert!(
        !output_path.exists(),
        "--dry-run must prevent the OUTPUT directive from creating testoutput.bin"
    );
}

#[test]
fn test_output_directive_with_command_line() {
    let _lock = acquire_lock();
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
