use std::fs;
use std::path::{Path, PathBuf};
use camino_tempfile::NamedUtf8TempFile;
use std::io::Read;

use cpclib_bndbuild::{build_args_parser, process_matches};

fn find_pairs(asm_dir: &Path, rasm_dir: &Path) -> Vec<(PathBuf, PathBuf)> {
    // For each .rasm in `rasm_dir`, pair it with the .asm of same stem in `asm_dir`.
    let mut pairs = Vec::new();
    for entry in walkdir::WalkDir::new(rasm_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let p = entry.path();
        if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
            if ext == "rasm" {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    let asm = asm_dir.join(format!("{}.asm", stem));
                    if asm.exists() {
                        pairs.push((asm, p.to_path_buf()));
                    }
                }
            }
        }
    }
    pairs
}

fn assemble_with_bndbuild_basm(_bndbuild: &Path, asm_path: &Path) -> Vec<u8> {
    let out_file = NamedUtf8TempFile::new().expect("unable to create temp file for basm output");
    let out_path = out_file.path().to_string();

    // Build argument vector and parse with bndbuild's clap parser
    let args = [
        "bndbuilder",
        "--direct",
        "--",
        "basm",
        asm_path.to_str().unwrap(),
        "-o",
        out_path.as_str(),
    ];

    let cmd = build_args_parser();
    let matches = cmd
        .try_get_matches_from(args)
        .expect("failed to parse arguments for basm via bndbuild");

    process_matches(&matches).expect("bndbuild process_matches failed for basm");

    let mut buf = Vec::new();
    let mut f = std::fs::File::open(out_path.as_str()).expect("unable to open basm output");
    f.read_to_end(&mut buf).expect("unable to read basm output");
    buf
}

fn assemble_with_bndbuild_rasm(_bndbuild: &Path, rasm_path: &Path) -> Vec<u8> {
    let out_file = NamedUtf8TempFile::new().expect("unable to create temp file for rasm output");
    let out_path = out_file.path().to_string();

    // rasm wants the include as a single token "-I<path>" and binary output as "-ob <file>"
    let include_token = format!("-I{}", "../cpclib-asm/assets/");

    let args = [
        "bndbuilder",
        "--direct",
        "--",
        "rasm",
        rasm_path.to_str().unwrap(),
        include_token.as_str(),
        "-ob",
        out_path.as_str(),
    ];

    let cmd = build_args_parser();
    let matches = cmd
        .try_get_matches_from(args)
        .expect("failed to parse arguments for rasm via bndbuild");

    process_matches(&matches).expect("bndbuild process_matches failed for rasm");

    let mut buf = Vec::new();
    let mut f = std::fs::File::open(out_path.as_str()).expect("unable to open rasm output");
    f.read_to_end(&mut buf).expect("unable to read rasm output");
    buf
}

#[test]
fn compare_basm_and_rasm_outputs() {
    // working dir is crate root (cpclib-rasm-basm-tests)
    let asm_dir = Path::new("../cpclib-basm/tests/asm");
    assert!(asm_dir.exists(), "cpclib-basm/tests/asm directory not found");

    let rasm_dir = Path::new("tests/asm");
    assert!(rasm_dir.exists(), "cpclib-rasm-basm-tests/tests/asm directory not found");

    let pairs = find_pairs(asm_dir, rasm_dir);
    assert!(!pairs.is_empty(), "no .asm/.rasm pairs found (asm: ../cpclib-basm/tests/asm, rasm: tests/asm)");

    // No external binary required any more; we call cpclib-bndbuild functions directly.
    let bndbuild = Path::new("");

    for (asm_path, rasm_path) in pairs {
        let a = assemble_with_bndbuild_basm(bndbuild, &asm_path);
        let b = assemble_with_bndbuild_rasm(bndbuild, &rasm_path);
        assert_eq!(a, b, "outputs differ for pair: {} / {}", asm_path.display(), rasm_path.display());
    }
}
