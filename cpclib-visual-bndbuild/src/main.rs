#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions::default();
    //native_options.vsync = false;
    //native_options.hardware_acceleration = eframe::HardwareAcceleration::Off;
    //native_options.renderer = eframe::Renderer::Wgpu;

    eframe::run_native(
        "Visual BndBuild",
        native_options,
        Box::new(|cc| Box::new(cpclib_visual_bndbuild::BndBuildApp::new(cc)))
    )
    .unwrap();
}
