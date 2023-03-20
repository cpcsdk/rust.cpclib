use cpclib_asm::basm_utils::*;

/// #[cfg(not(target_arch = "wasm32"))]
fn main() {
    let app = build_args_parser();
    claui::run(app, |matches| {
        dbg!(matches);

        match process(matches) {
            Ok((_env, warnings)) => {
                let warnings = warnings
                    .iter()
                    .map(|w| format!("{}", w))
                    .collect::<Vec<_>>();

                eprintln!("{:?}", warnings);
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    });
}
