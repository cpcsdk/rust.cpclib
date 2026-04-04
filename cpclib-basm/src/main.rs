#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications,
    nonstandard_style,
    rust_2018_idioms,
    unused,
    warnings
)]
#![deny(clippy::pedantic)]
#![allow(clippy::cast_possible_truncation)]

use std::sync::Arc;

use cpclib_asm::EnvEventObserver;
use cpclib_basm::{build_args_parser, process};

static DESC_BEFORE: &str = const_format::formatc!(
    "Profile {} compiled: {}",
    cpclib_basm::built_info::PROFILE,
    cpclib_basm::built_info::BUILT_TIME_UTC
);

fn basm() -> i32 {
    let start = std::time::Instant::now();
    let o: Arc<dyn EnvEventObserver> = Arc::new(());

    let parser = build_args_parser().before_help(DESC_BEFORE);
    let args: Vec<String> = std::env::args().collect();
    let matches = match parser.try_get_matches_from(args) {
        Ok(m) => m,
        Err(e)
            if e.kind() == cpclib_common::clap::error::ErrorKind::DisplayHelp
                || e.kind() == cpclib_common::clap::error::ErrorKind::DisplayVersion =>
        {
            o.emit_stdout(&e.to_string());
            return 0;
        },
        Err(e) => {
            o.emit_stderr(&e.to_string());
            return -1;
        }
    };

    match process(&matches, o.clone()) {
        Ok((env, warnings)) => {
            for warning in warnings {
                o.emit_stderr(&format!("{warning}"));
            }

            let report = env.report(&start);
            o.emit_stdout(&report.to_string());

            0
        },
        Err(e) => {
            o.emit_stderr(&format!("Error while assembling.\n{e}"));
            -1
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn main() -> Result<(), i32> {
    let code = basm();
    if code != 0 { Err(code) } else { Ok(()) }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use std::process::exit;

    let code = std::thread::Builder::new()
        .stack_size(1024 * 1024 * 1024)
        .spawn(basm)
        .unwrap()
        .join()
        .unwrap();
    exit(code);
}
