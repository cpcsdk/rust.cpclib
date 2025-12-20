use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use camino_tempfile::NamedUtf8TempFile;
use cpclib_bndbuild::{build_args_parser, process_matches};
// use the macro with a workspace-root relative pattern (avoids compile-time missing-resource panic)

fn assemble_with_bndbuild_basm(_bndbuild: &Path, asm_path: &Path) -> Result<Vec<u8>> {
    let out_file = NamedUtf8TempFile::new().context("creating temp file for basm output")?;
    let out_path = out_file.path().to_string();

    // Build argument vector and parse with bndbuild's clap parser
    let args = [
        "bndbuilder",
        "--direct",
        "--",
        "basm",
        asm_path
            .to_str()
            .ok_or_else(|| anyhow!("asm path is not valid unicode"))?,
        "-o",
        out_path.as_str()
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

    let args = dbg!([
        "bndbuilder",
        "--direct",
        "--",
        "rasm",
        rasm_path
            .to_str()
            .ok_or_else(|| anyhow!("rasm path is not valid unicode"))?,
        include_token.as_str(),
        "-ob",
        out_path.as_str(),
    ]);

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

// Use a workspace-root relative pattern so the proc-macro can find resources here.
// Strip the workspace prefix inside the test so the test code itself doesn't contain
// the workspace path literal.
#[test_generator::test_resources("cpclib-rasm-basm-tests/tests/asm/*.rasm")]
#[cfg(not(windows))]
fn compare_basm_and_rasm_outputs_per_file(rasm: &str) {
    // working dir is crate root (cpclib-rasm-basm-tests)
    let asm_dir = Path::new("../cpclib-basm/tests/asm");
    assert!(
        asm_dir.exists(),
        "cpclib-basm/tests/asm directory not found"
    );

    // `rasm` is provided by the macro as a workspace-prefixed path like
    // "cpclib-rasm-basm-tests/tests/asm/foo.rasm". Strip that prefix to get
    // the crate-relative path used at runtime.
    let prefix = "cpclib-rasm-basm-tests/";
    let rasm_rel = &rasm[prefix.len()..];

    let rasm_path = Path::new(rasm_rel);

    if !rasm_path.exists() {
        panic!("rasm path should exist at runtime: {}", rasm_path.display());
    }

    let stem = rasm_path
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("invalid rasm file stem");
    let asm_path = asm_dir.join(format!("{}.asm", stem));
    assert!(asm_path.exists(), "asm file not found for {}", rasm_rel);

    let bndbuild = Path::new("");

    let a = assemble_with_bndbuild_basm(bndbuild, &asm_path).expect("basm assembly failed");
    let b = assemble_with_bndbuild_rasm(bndbuild, rasm_path).expect("rasm assembly failed");

    assert_eq!(
        a,
        b,
        "outputs differ for pair: {} / {}",
        asm_path.display(),
        rasm_path.display()
    );
}
