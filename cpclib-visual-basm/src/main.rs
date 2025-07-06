use std::sync::Arc;

use cpclib_basm::*;

/// #[cfg(not(target_arch = "wasm32"))]
fn main() {
    let app = build_args_parser();
    claui::run(app, |matches| {
        dbg!(matches);

        match process(matches, Arc::new(())) {
            Ok((_env, warnings)) => {
                let warnings = warnings.iter().map(|w| format!("{w}")).collect::<Vec<_>>();

                eprintln!("{warnings:?}");
            },
            Err(e) => {
                eprintln!("{e}");
            }
        }
    })
    .unwrap();
}
