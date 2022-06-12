
use cpclib_asm::basm_utils::*;
use klask::Settings;

///#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let app =  build_args_parser();
    klask::run_app(app, Settings::default(), |matches| {

        dbg!(matches);
        
        match process(matches) {
            Ok((_env, warnings)) => {
                let  warnings = warnings
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
