mod basm_app;
use basm_app::*;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let app = BasmApp::default();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
