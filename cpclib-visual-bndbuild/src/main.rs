#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use cpclib_common::clap::*;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions::default();
    // native_options.vsync = false;
    // native_options.hardware_acceleration = eframe::HardwareAcceleration::Off;
    // native_options.renderer = eframe::Renderer::Wgpu;

    let matches = Command::new("Visual BndBuild")
        .arg(Arg::new("INPUT").help("Path to bndbuild.yml input file"))
        .get_matches();

    eframe::run_native(
        "Visual BndBuild",
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(cpclib_visual_bndbuild::BndBuildApp::new(
                cc,
                matches.get_one::<String>("INPUT")
            )))
        })
    )
    .unwrap();
}
