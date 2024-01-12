use std::process::Command;

use cpclib_asm::assembler::Env;
use cpclib_asm::error::AssemblerError;
use cpclib_basm::{build_args_parser, process, BasmError};
use cpclib_common::itertools::Itertools;
use cpclib_common::lazy_static;
use pretty_assertions::assert_eq;
use regex::Regex;
use test_generator::test_resources;

lazy_static::lazy_static! {
    static ref  LOCK: parking_lot::Mutex<()> = parking_lot::Mutex::default();
}

fn manual_cleanup() {
    for fname in &[
        "hello.dsk",
        "hello.hfe",
        "good_save_whole_inner.bin",
        "hello1.bin",
        "hello2.bin",
        "hello3.bin",
        "good_save_txt.bin"
    ] {
        let p = std::path::Path::new(fname);
        if p.exists() {
            std::fs::remove_file(p).unwrap()
        }
    }
}

fn command_for_generated_test(
    fname: &str,
    output: &str
) -> Result<(Env, Vec<AssemblerError>), BasmError> {
    let args_parser = build_args_parser();
    let args =
        args_parser.get_matches_from(&["basm", "-I", "tests/asm/", "-i", "-o", output, fname]);

    process(&args)
}

fn specific_test(folder: &str, fname: &str) {
    let output_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let res = Command::new("../target/debug/basm")
        .args(["-I", folder, "-i", fname, "-o", output_fname])
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

#[test]
#[ignore]
fn test_roudoudou_generated_code() {
    std::fs::create_dir("generated_sprites");
    specific_test("tests/asm/roudoudou", "rasm_sprites.asm");
    std::fs::remove_dir("generated_sprites");
}

#[test_resources("cpclib-basm/tests/asm/warning_*.asm")]
fn expect_warning_but_success(real_fname: &str) {
    let _lock = LOCK.lock();
    manual_cleanup();

    let fname = &real_fname["cpclib-basm/tests/asm/".len()..];

    let output_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let listing_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let listing_fname = listing_file.path().as_os_str().to_str().unwrap();

    let content = std::fs::read_to_string(dbg!(&real_fname["cpclib-basm/".len()..]))
        .expect("Unable to read_source");

    lazy_static::lazy_static! {
        static ref RE1: Regex = Regex::new(r";.*$").unwrap();
        static ref RE2: Regex = Regex::new(r":\s*:").unwrap();
    }

    let mut content = content
        .split("\n")
        .map(|l| RE1.replace(&l, "").replace('\r', ""))
        .join(":");
    dbg!(&content);
    while RE2.is_match(&content) {
        content = RE2.replace_all(&content, ":").to_string();
    }
    dbg!(&content);

    let content = if content.chars().next().unwrap() == ':' {
        &content[1..]
    }
    else {
        &content[..]
    };
    dbg!(&content);

    let content = if let Some(':') = content.chars().last() {
        &content[..content.len() - 1]
    }
    else {
        content
    };

    dbg!(&content);

    let content = content.replace("\\:", "");

    dbg!(&content);

    if !content.is_empty() {
        let input_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
        let input_fname = input_file.path().as_os_str().to_str().unwrap();
        std::fs::write(input_fname, content).unwrap();

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
        if !stderr.contains("warning: ") {
            panic!("No warning have been generated");
        }
    }
}

#[test_resources("cpclib-basm/tests/asm/good_*.asm")]
fn expect_one_line_success(real_fname: &str) {

    if real_fname.contains("basic") // basic cannot be inlined 
    || real_fname.contains("good_module.asm")
    // there are labels with ::
    {
        return;
    }
    let _lock = LOCK.lock();
    manual_cleanup();

    manual_cleanup();

    let fname = &real_fname["cpclib-basm/tests/asm/".len()..];

    let output_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let listing_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let listing_fname = listing_file.path().as_os_str().to_str().unwrap();

    let content = std::fs::read_to_string(dbg!(&real_fname["cpclib-basm/".len()..]))
        .expect("Unable to read_source");

    lazy_static::lazy_static! {
        static ref RE1: Regex = Regex::new(r";.*$").unwrap();
        static ref RE2: Regex = Regex::new(r":\s*:").unwrap();
    }

    let mut content = content
        .split("\n")
        .map(|l| RE1.replace(&l, "").replace('\r', ""))
        .join(":");
    dbg!(&content);
    while RE2.is_match(&content) {
        content = RE2.replace_all(&content, ":").to_string();
    }
    dbg!(&content);

    let content = if content.chars().next().unwrap() == ':' {
        &content[1..]
    }
    else {
        &content[..]
    };
    dbg!(&content);

    let content = if let Some(':') = content.chars().last() {
        &content[..content.len() - 1]
    }
    else {
        content
    };

    dbg!(&content);

    let content = content.replace("\\:", "");

    dbg!(&content);

    if !content.is_empty() {
        let input_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
        let input_fname = input_file.path().as_os_str().to_str().unwrap();
        std::fs::write(input_fname, content).unwrap();

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

    let output_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let listing_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let listing_fname = listing_file.path().as_os_str().to_str().unwrap();

    let content = std::fs::read_to_string(dbg!(&real_fname["cpclib-basm/".len()..]))
        .expect("Unable to read_source");

    lazy_static::lazy_static! {
        static ref RE1: Regex = Regex::new(r"(?m)([^\\])\n").unwrap();
        static ref RE2: Regex = Regex::new(r"(?m)\\\n").unwrap();
    }

    let content = content.replace("\r", "");
    let content = RE1.replace_all(&content, "$1\n\n\n");
    let content = RE2.replace_all(&content, "\\\n\\\n\\\n");
    let content = content.as_ref();

    eprintln!("{}", &content);

    let input_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let input_fname = input_file.path().as_os_str().to_str().unwrap();
    std::fs::write(input_fname, content).unwrap();

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

    let output_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let listing_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let listing_fname = listing_file.path().as_os_str().to_str().unwrap();

    let res = Command::new("../target/debug/basm")
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
        .expect("Unable to launch basm");

    if !res.status.success() {
        panic!(
            "Failure to assemble {}.\n{}",
            fname,
            String::from_utf8_lossy(&res.stderr)
        );
    }
}

//#[test_resources("basm/tests/asm/good_*.sym")]
/// TODO write tests specifics for this purpose
fn expect_symbols_success(fname: &str) {
    let _lock = LOCK.lock();

    manual_cleanup();

    let sym_gt = &fname["cpclib-basm/tests/asm/".len()..];
    let fname = sym_gt.replace(".sym", ".asm");

    let output_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let symbol_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
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

    let sym_gt = std::fs::read_to_string(fname).unwrap();
    let sym = std::fs::read_to_string(symbol_fname).expect("Symbols not generated");

    assert_eq!(sym_gt, sym, "Symbols differ.");
}

#[test_resources("cpclib-basm/tests/asm/good_*.asm")]
fn expect_success(fname: &str) {
    let _lock = LOCK.lock();

    manual_cleanup();

    eprintln!("{}", fname);

    let fname = &fname["cpclib-basm/tests/asm/".len()..];

    let output_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
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
                tempfile::NamedTempFile::new().expect("Unable to build temporary file");
            let equiv_output_fname = equiv_output_file.path().as_os_str().to_str().unwrap();

            let res_equiv = command_for_generated_test(&equiv_fname, equiv_output_fname);
            if !res_equiv.is_ok() {
                eprintln!(
                    "Error while assembling the equivalent file.\n{}",
                    res.err().unwrap().to_string()
                );
                panic!()
            }

            let output_content = std::fs::read(output_fname).unwrap();
            let equiv_output_content = std::fs::read(equiv_output_fname).unwrap();
            assert_eq!(
                output_content, equiv_output_content,
                "Content differ between {} and {}.",
                fname, equiv_fname
            );
        }
    }
    else {
        eprintln!(
            "Error when assembling {}:\n{}",
            fname,
            res.err().unwrap().to_string()
        );
        panic!()
    }
}

#[test_resources("cpclib-basm/tests/asm/bad_*.asm")]
fn expect_failure(fname: &str) {
    let _lock = LOCK.lock();

    manual_cleanup();

    let fname = &fname["cpclib-basm/tests/asm/".len()..];

    let output_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
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
