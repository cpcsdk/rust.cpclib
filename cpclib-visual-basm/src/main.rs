mod basm_app;
use basm_app::*;
 use eframe::egui;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let app = BasmApp::default();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Visual BASM", 
        native_options, 
        Box::new(|cc| Box::new(app))
    );
}
