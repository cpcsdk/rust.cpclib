use std::env;
use std::path::Path;

use cc;

// no idea why it is not disabled in wasm32 :(
#[cfg(target_arch = "wasm32")]
fn build() {

}
#[cfg(not(target_arch = "wasm32"))]
fn build() 
{
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
}


fn main() {
    build();
}
