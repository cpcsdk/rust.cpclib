mod common;

use std::process::Command;

use pretty_assertions::assert_eq;
use test_generator::test_resources;
use common::{GLOBAL_TEST_LOCK, manual_cleanup};

fn acquire_lock() -> parking_lot::lock_api::MutexGuard<'static, parking_lot::RawMutex, ()> {
    GLOBAL_TEST_LOCK.lock()
}

#[derive(Debug)]
struct ListingBytesRow {
    logical_address: u32,
    has_remapped_physical_address: bool,
    bytes: Vec<u8>
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
            "{A} {P} {C} | {S}",
            "--lst-no-line-numbers"
        ])
        .output()
        .expect("Unable to launch basm")
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

fn extract_rows_from_code_only_listing(listing: &str) -> Vec<CodeOnlyListingRow> {
    listing
        .lines()
        .filter_map(|line| {
            let line = line.strip_prefix('>').unwrap_or(line);
            let left = line.split_once(" | ").map(|(left, _)| left).unwrap_or(line);

            let mut parts = left.split_whitespace();
            let addr = parts.next()?;
            if addr.len() != 4 || !addr.chars().all(|c| c.is_ascii_hexdigit()) {
                return None;
            }

            let mut remaining_parts = Vec::new();
            if let Some(next) = parts.next() {
                if !(next.len() != 2 && next.chars().all(|c| c.is_ascii_hexdigit())) {
                    remaining_parts.push(next);
                }
            }
            remaining_parts.extend(parts);

            let bytes = remaining_parts
                .iter()
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

fn listing_byte_equivalence_is_meaningful(fname: &str) -> bool {
    let _ = fname;
    true
}

fn assemble_and_compare_listing_bytes_from_lst(fname: &str) {
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

    let listing = fs_err::read_to_string(listing_fname).expect("Listing is missing");
    let rows = extract_byte_rows_from_text_listing(&listing);

    if rows.is_empty() || rows.iter().any(|row| row.has_remapped_physical_address) {
        return;
    }

    let code_only_listing_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let code_only_listing_fname = code_only_listing_file.path().as_os_str().to_str().unwrap();

    let code_only_output_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let code_only_output_fname = code_only_output_file.path().as_os_str().to_str().unwrap();

    let res_code_only =
        run_basm_once_with_code_only_listing(fname, code_only_output_fname, code_only_listing_fname);

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
                "Generated output differs from bytes shown in text listing for {fname} at 0x{:04X}.",
                row.logical_address
            );
        }

        return;
    }

    let code_only_listing =
        fs_err::read_to_string(code_only_listing_fname).expect("Code-only listing is missing");
    let code_only_rows = extract_rows_from_code_only_listing(&code_only_listing);

    for row in rows {
        let exists_in_code_only = code_only_rows
            .iter()
            .any(|candidate| candidate.logical_address == row.logical_address && candidate.bytes == row.bytes);

        assert!(
            exists_in_code_only,
            "Bytes shown in text listing for {fname} at 0x{:04X} are missing or different in code-only listing.",
            row.logical_address
        );
    }
}

#[test]
fn listing_contains_good_str_directives_and_escaped_quotes() {
    let _lock = acquire_lock();
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
    let res_code_only =
        run_basm_once_with_code_only_listing("good_str.asm", output_fname, code_only_listing_fname);
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

#[test]
fn listing_contains_full_good_basic_locomotive_source_block() {
    let _lock = acquire_lock();
    manual_cleanup();

    let output_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let listing_file =
        camino_tempfile::NamedUtf8TempFile::new().expect("Unable to build temporary file");
    let listing_fname = listing_file.path().as_os_str().to_str().unwrap();

    let res = run_basm_once_with_listing("good_basic.asm", output_fname, listing_fname);
    if !res.status.success() {
        panic!(
            "Failure to assemble good_basic.asm.\n{}",
            format_basm_failure(&res)
        );
    }

    let listing = fs_err::read_to_string(listing_fname).expect("Listing is missing");

    let listing_lower = listing.to_ascii_lowercase();
    assert!(listing_lower.contains("locomotive"));
    assert!(listing_lower.contains("10 rem basic loader of binary exec"));
    assert!(listing_lower.contains("20 rem yeah !!"));
    assert!(listing_lower.contains("30 call"));
    assert!(listing_lower.contains("endlocomotive"));
    assert!(!listing.contains(">0178"));
}

#[test_resources("cpclib-basm/tests/asm/good_*.asm")]
fn expect_listing_bytes_match_generated_output(fname: &str) {
    let _lock = acquire_lock();

    manual_cleanup();

    let fname = &fname["cpclib-basm/tests/asm/".len()..];
    if !listing_byte_equivalence_is_meaningful(fname) {
        return;
    }

    assemble_and_compare_listing_bytes_from_lst(fname);
}
