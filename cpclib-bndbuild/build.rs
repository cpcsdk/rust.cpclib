use std::env;
use std::path::Path;

fn main() {

    build_deps::rerun_if_changed_paths("tests/valid/*.yml").unwrap();
    build_deps::rerun_if_changed_paths("tests/invalid/*.yml").unwrap();

    built::write_built_file(
    )
    .expect("Failed to acquire build-time information");
}
