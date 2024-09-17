use std::rc::Rc;

use cpclib_basm::*;

/// #[cfg(not(target_arch = "wasm32"))]
fn main() {
    let app = build_args_parser();
    claui::run(app, |matches| {
        dbg!(matches);

        match process(matches, Rc::new(())) {
            Ok((_env, warnings)) => {
                let warnings = warnings
                    .iter()
                    .map(|w| format!("{}", w))
                    .collect::<Vec<_>>();

                eprintln!("{:?}", warnings);
            },
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    })
    .unwrap();
}
