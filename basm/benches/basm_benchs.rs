#![feature(path_file_prefix)]

use std::fs;

use cpclib_asm::basm_utils::{build_args_parser, process};
use cpclib_common::itertools::Itertools;
use criterion::{criterion_group, criterion_main, Criterion};
use globset::Glob;
use tempfile::NamedTempFile;

#[inline]
fn command_for_generated_test(fname: &str, output_file: &NamedTempFile) {
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let args_parser = build_args_parser();
    let args = args_parser.get_matches_from(&[
        "basm",
        "-I",
        "tests/asm/",
        "-i",
        "-o",
        output_fname,
        fname
    ]);
    process(&args).expect("basm failed");
}

fn criterion_benchmark(c: &mut Criterion) {
    let glob = Glob::new("**/good_*.asm").unwrap().compile_matcher();
    let fnames = fs::read_dir("tests/asm")
        .expect("Unable to browse asm dir")
        .filter_map(|name| name.ok())
        .map(|name| name.path().to_str().unwrap().to_owned())
        .filter(|name| glob.is_match(name))
        .map(|name| {
            (
                name,
                tempfile::NamedTempFile::new().expect("Unable to build temporary file")
            )
        })
        .collect_vec();

    for (fname, output) in &fnames {
        c.bench_function(
            std::path::Path::new(fname)
                .file_prefix()
                .unwrap()
                .to_str()
                .unwrap(),
            |b| b.iter(|| command_for_generated_test(fname, output))
        );
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
