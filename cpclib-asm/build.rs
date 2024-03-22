use std::env;
use std::path::Path;

fn build() {
    built::write_built_file(
    )
    .expect("Failed to acquire build-time information");
}

fn main() {
    build_deps::rerun_if_changed_paths("assets").unwrap();
    build_deps::rerun_if_changed_paths("assets/**").unwrap();
    build_deps::rerun_if_changed_paths("assets/*.*").unwrap();
    build_deps::rerun_if_changed_paths("assets/**/*.*").unwrap();

    if !env::var("CARGO_CFG_TARGET_ARCH")
        .unwrap()
        .contains("wasm32")
    {
        build();
    }
}
