use std::env;

fn build_others() {
    built::write_built_file().expect("Failed to acquire build-time information");

    // apultra crunch
    cc::Build::new()
        .warnings(false)
        .file("extra/apultra.c")
        .opt_level(3)
        .shared_flag(true)
        .cargo_metadata(true)
        .compile("apultra");

    // exomizer crunch
    cc::Build::new()
        .warnings(false)
        .file("extra/exomizer.c")
        .opt_level(3)
        .shared_flag(true)
        .cargo_metadata(true)
        .compile("exomizer");

    // zx0 crunch
    cc::Build::new()
        .warnings(false)
        .file("extra/zx0/compress.c")
        .file("extra/zx0/memory.c")
        .file("extra/zx0/optimize.c")
        .opt_level(3)
        .shared_flag(false)
        .cargo_metadata(true)
        .compile("zx0");

    // lz4 crunch
    cc::Build::new()
        .warnings(false)
        .file("extra/lz4_embedded.c")
        .opt_level(3)
        .shared_flag(true)
        .cargo_metadata(true)
        .compile("lz4");

    // zx7 crunch
    cc::Build::new()
        .warnings(false)
        .file("extra/zx7.c")
        .opt_level(3)
        .shared_flag(false)
        .cargo_metadata(true)
        .compile("zx7");
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

fn build_lzsa() {
    cc::Build::new()
        .include("extra/lzsa/lzsa-master/src/libdivsufsort/include")
        .file("extra/lzsa/lzsa-master/src/dictionary.c")
        .file("extra/lzsa/lzsa-master/src/expand_block_v1.c")
        .file("extra/lzsa/lzsa-master/src/expand_block_v2.c")
        .file("extra/lzsa/lzsa-master/src/expand_context.c")
        .file("extra/lzsa/lzsa-master/src/expand_inmem.c")
        .file("extra/lzsa/lzsa-master/src/frame.c")
        .file("extra/lzsa/lzsa-master/src/matchfinder.c")
        .file("extra/lzsa/lzsa-master/src/shrink_block_v1.c")
        .file("extra/lzsa/lzsa-master/src/shrink_block_v2.c")
        .file("extra/lzsa/lzsa-master/src/shrink_context.c")
        .file("extra/lzsa/lzsa-master/src/shrink_inmem.c")
        .file("extra/lzsa/lzsa-master/src/shrink_streaming.c")
        .flag("-fomit-frame-pointer")
        .opt_level(3)
        .shared_flag(true)
        .cargo_metadata(true)
        .compile("lzsa");
}
fn main() {
    if !env::var("CARGO_CFG_TARGET_ARCH")
        .unwrap()
        .contains("wasm32")
    {
        build_others();
        build_shrinkler();
        build_lzsa();
    }
}
