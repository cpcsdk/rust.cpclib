use std::path::{Path, PathBuf};
use camino_tempfile::NamedUtf8TempFile;
use std::io::Read;

use anyhow::{anyhow, Context, Result};
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

fn assemble_with_bndbuild_basm(_bndbuild: &Path, asm_path: &Path) -> Result<Vec<u8>> {
    let out_file = NamedUtf8TempFile::new().context("creating temp file for basm output")?;
    let out_path = out_file.path().to_string();

    // Build argument vector and parse with bndbuild's clap parser
    let args = [
        "bndbuilder",
        "--direct",
        "--",
        "basm",
        asm_path.to_str().ok_or_else(|| anyhow!("asm path is not valid unicode"))?,
        "-o",
        out_path.as_str(),
    ];

    let cmd = build_args_parser();
    let matches = cmd
        .try_get_matches_from(args)
        .map_err(|e| anyhow!(e.to_string()))?;

    process_matches(&matches)?;

    let mut buf = Vec::new();
    let mut f = std::fs::File::open(out_path.as_str()).context("opening basm output")?;
    f.read_to_end(&mut buf).context("reading basm output")?;
    Ok(buf)
}

fn assemble_with_bndbuild_rasm(_bndbuild: &Path, rasm_path: &Path) -> Result<Vec<u8>> {
    let out_file = NamedUtf8TempFile::new().context("creating temp file for rasm output")?;
    let out_path = out_file.path().to_string();

    // rasm wants the include as a single token "-I<path>" and binary output as "-ob <file>"
    let include_token = format!("-I{}", "../cpclib-asm/assets/");

    let args = [
        "bndbuilder",
        "--direct",
        "--",
        "rasm",
        rasm_path.to_str().ok_or_else(|| anyhow!("rasm path is not valid unicode"))?,
        include_token.as_str(),
        "-ob",
        out_path.as_str(),
    ];

    let cmd = build_args_parser();
    let matches = cmd
        .try_get_matches_from(args)
        .map_err(|e| anyhow!(e.to_string()))?;

    process_matches(&matches)?;

    let mut buf = Vec::new();
    let mut f = std::fs::File::open(out_path.as_str()).context("opening rasm output")?;
    f.read_to_end(&mut buf).context("reading rasm output")?;
    Ok(buf)
}

#[test]
fn compare_basm_and_rasm_outputs() -> Result<()> {
    // working dir is crate root (cpclib-rasm-basm-tests)
    let asm_dir = Path::new("../cpclib-basm/tests/asm");
    if !asm_dir.exists() {
        return Err(anyhow!("cpclib-basm/tests/asm directory not found"));
    }

    let rasm_dir = Path::new("tests/asm");
    if !rasm_dir.exists() {
        return Err(anyhow!("cpclib-rasm-basm-tests/tests/asm directory not found"));
    }

    let pairs = find_pairs(asm_dir, rasm_dir);
    if pairs.is_empty() {
        return Err(anyhow!("no .asm/.rasm pairs found (asm: ../cpclib-basm/tests/asm, rasm: tests/asm)"));
    }

    // No external binary required any more; we call cpclib-bndbuild functions directly.
    let bndbuild = Path::new("");

    for (asm_path, rasm_path) in pairs {
        let a = assemble_with_bndbuild_basm(bndbuild, &asm_path)?;
        let b = assemble_with_bndbuild_rasm(bndbuild, &rasm_path)?;
        if a != b {
            return Err(anyhow!("outputs differ for pair: {} / {}", asm_path.display(), rasm_path.display()));
        }
    }

    Ok(())
}
