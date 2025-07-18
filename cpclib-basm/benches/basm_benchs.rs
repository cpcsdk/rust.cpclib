#![feature(path_file_prefix)]

use std::fs;
use std::sync::Arc;

use camino_tempfile::NamedUtf8TempFile;
use cpclib_basm::{build_args_parser, process};
use cpclib_common::itertools::Itertools;
use criterion::{Criterion, criterion_group, criterion_main};
use globset::Glob;

#[inline]
fn command_for_generated_test(fname: &str, output_file: &NamedUtf8TempFile) {
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let args_parser = build_args_parser();
    let args =
        args_parser.get_matches_from(["basm", "-I", "tests/asm/", "-i", "-o", output_fname, fname]);
    process(&args, Arc::from(())).expect("basm failed");
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
                NamedUtf8TempFile::new().expect("Unable to build temporary file")
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
