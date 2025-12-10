use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use camino_tempfile::NamedUtf8TempFile;
use std::io::Read;

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

fn assemble_with_bndbuild_basm(bndbuild: &Path, asm_path: &Path) -> Vec<u8> {
    let out_file = NamedUtf8TempFile::new().expect("unable to create temp file for basm output");
    let out_path = out_file.path().as_os_str().to_str().unwrap().to_string();

    let status = Command::new(bndbuild)
        .arg("--direct")
        .arg("--")
        .arg("basm")
        .arg(asm_path.to_str().unwrap())
        .arg("-o")
        .arg(out_path.as_str())
        .status()
        .expect("failed to run bndbuild basm");

    assert!(status.success(), "bndbuild basm failed for {}", asm_path.display());

    let mut buf = Vec::new();
    let mut f = std::fs::File::open(out_path.as_str()).expect("unable to open basm output");
    f.read_to_end(&mut buf).expect("unable to read basm output");
    buf
}

fn assemble_with_bndbuild_rasm(bndbuild: &Path, rasm_path: &Path) -> Vec<u8> {
    let out_file = NamedUtf8TempFile::new().expect("unable to create temp file for rasm output");
    let out_path = out_file.path().as_os_str().to_str().unwrap().to_string();

    let status = Command::new(bndbuild)
        .arg("--direct")
        .arg("--")
        .arg("rasm")
        .arg(rasm_path.to_str().unwrap())
        .arg(format!("-I{}", "../cpclib-asm/assets/"))
        .arg("-ob")
        .arg(out_path.as_str())
        .status()
        .expect("failed to run bndbuild rasm");

    assert!(status.success(), "bndbuild rasm failed for {}", rasm_path.display());

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

    // ensure bndbuild binary is built
    let build_status = Command::new("cargo")
        .args(["build", "-p", "cpclib-bndbuild"]) 
        .status()
        .expect("failed to spawn cargo build for cpclib-bndbuild");
    assert!(build_status.success(), "cargo build -p cpclib-bndbuild failed");

    let bndbuild = Path::new("../target/debug/bndbuild");
    assert!(bndbuild.exists(), "bndbuild binary not found at ../target/debug/bndbuild");

    for (asm_path, rasm_path) in pairs {
        let a = assemble_with_bndbuild_basm(bndbuild, &asm_path);
        let b = assemble_with_bndbuild_rasm(bndbuild, &rasm_path);
        assert_eq!(a, b, "outputs differ for pair: {} / {}", asm_path.display(), rasm_path.display());
    }
}
