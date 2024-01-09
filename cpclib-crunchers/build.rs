use std::env;
use std::path::Path;

fn build() {
    let src = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dst = Path::new(&env::var("OUT_DIR").unwrap()).join("built.rs");

    built::write_built_file_with_opts(
        built::Options::default().set_time(true),
        Path::new(&src),
        &dst
    )
    .expect("Failed to acquire build-time information");

    cc::Build::new()
        .warnings(false)
        .file("extra/apultra.c")
        .opt_level(0)
        .shared_flag(true)
        .cargo_metadata(true)
        .compile("apultra");

    cc::Build::new()
        .warnings(false)
        .file("extra/exomizer.c")
        .opt_level(0)
        .shared_flag(true)
        .cargo_metadata(true)
        .compile("exomizer");

    cc::Build::new()
        .warnings(false)
        .file("extra/zx0/compress.c")
        .file("extra/zx0/memory.c")
        .file("extra/zx0/optimize.c")
        .opt_level(0)
        .shared_flag(false)
        .cargo_metadata(true)
        .compile("zx0");

    cc::Build::new()
        .warnings(false)
        .file("extra/lz4_embedded.c")
        .opt_level(0)
        .shared_flag(true)
        .cargo_metadata(true)
                .compile("lz4");
}

fn build_shrinkler() {
    cxx_build::bridge("src/shrinkler.rs")
    .file("extra/Shrinkler4.6NoParityContext//basm_bridge.cpp")
    .flag_if_supported("-std=c++14")
    .compile("cpclib-crunchers");


    println!("cargo:rerun-if-changed=src/shrinkler.rs");
    println!("cargo:rerun-if-changed=extra/Shrinkler4.6NoParityContext//basm_bridge.cpp");
    println!("cargo:rerun-if-changed=extra/Shrinkler4.6NoParityContext//basm_bridge.h");
}

fn main() {
    if !env::var("CARGO_CFG_TARGET_ARCH")
        .unwrap()
        .contains("wasm32")
    {
        build();
        build_shrinkler();
    }
}
