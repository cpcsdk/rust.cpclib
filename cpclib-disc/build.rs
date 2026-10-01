use std::env;
use std::path::PathBuf;

fn main() {
    built::write_built_file().expect("Failed to acquire build-time information");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && env::var_os("CARGO_FEATURE_HFE").is_some()
    {
        println!("cargo:rerun-if-env-changed=DEP_HXCFE_INCLUDE");
        let hxcfe_sources = PathBuf::from(
            env::var_os("DEP_HXCFE_INCLUDE")
                .expect("hxcfe-sys did not provide its source directory")
        );
        let nt4_dir = hxcfe_sources.join("thirdpartylibs/adflib/Lib/Win32");
        let nt4_source = nt4_dir.join("nt4_dev.c");

        // hxcfe-sys omits this file even though its Windows ADF sources call it.
        println!("cargo:rerun-if-changed={}", nt4_source.display());
        cc::Build::new()
            .file(nt4_source)
            .include(nt4_dir)
            .compile("hxcfe_nt4");
    }
}
