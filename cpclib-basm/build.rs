use std::env;
use std::path::Path;

fn main() {
    //   static_vcruntime::metabuild();
    build_deps::rerun_if_changed_paths( "tests/asm/*.asm" ).unwrap();
    build_deps::rerun_if_changed_paths( "tests/asm/*" ).unwrap();

    let src = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dst = Path::new(&env::var("OUT_DIR").unwrap()).join("built.rs");

    built::write_built_file_with_opts(
        built::Options::default().set_time(true),
        Path::new(&src),
        &dst
    )
    .expect("Failed to acquire build-time information");
}
