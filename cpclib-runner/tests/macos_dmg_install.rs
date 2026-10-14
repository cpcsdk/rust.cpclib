//! The real macOS installs of the DMG-based emulators: AMSpiriT Lite (a DMG in
//! 1.14.3, a zip in 1.16.0) and Cadence.
//!
//! Ignored by default because it downloads and wipes each emulator's cache
//! folder:
//!
//! ```text
//! cargo test -p cpclib-runner --test macos_dmg_install -- --ignored --nocapture --test-threads=1
//! ```
#![cfg(target_os = "macos")]

use cpclib_runner::event::DiscardObserver;
use cpclib_runner::runner::emulator::Emulator;
use cpclib_runner::runner::emulator::amspiritlite::AmspiritLiteVersion;
use cpclib_runner::runner::emulator::cadence::CadenceVersion;

/// Installs `emu` from an empty cache, twice, and returns its executable.
fn install_twice(emu: Emulator) -> std::path::PathBuf {
    let conf = emu.configuration::<DiscardObserver>();
    let _ = std::fs::remove_dir_all(conf.cache_folder());
    conf.install(&DiscardObserver).expect("fresh install");
    // over an existing cache
    conf.install(&DiscardObserver).expect("second install");
    conf.exec_fname().into_std_path_buf()
}

fn assert_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(path)
        .unwrap_or_else(|e| panic!("{} is missing: {e}", path.display()))
        .permissions()
        .mode();
    assert!(mode & 0o111 != 0, "{} is not executable", path.display());
}

fn assert_nothing_mounted() {
    let info = std::process::Command::new("hdiutil")
        .arg("info")
        .output()
        .unwrap();
    let info = String::from_utf8_lossy(&info.stdout);
    assert!(!info.contains("_mount"), "a dmg is still mounted:\n{info}");
}

#[test]
#[ignore = "downloads AMSpiriT Lite and wipes its cache"]
fn amspirit_lite_installs() {
    for version in [AmspiritLiteVersion::V1_16_0] {
        let exec = install_twice(Emulator::AmspiritLite(version.clone()));
        assert!(
            exec.to_string_lossy().contains(".app/Contents/MacOS/"),
            "{version:?}: {}",
            exec.display()
        );
        assert_executable(&exec);
        assert_nothing_mounted();
    }
}

#[test]
#[ignore = "downloads Cadence and wipes its cache"]
fn cadence_installs() {
    for version in [CadenceVersion::V1_1, CadenceVersion::V1_4] {
        let exec = install_twice(Emulator::Cadence(version.clone()));
        assert_executable(&exec);
        let folder = exec.parent().unwrap();
        assert_executable(&folder.join("cadence.app/Contents/MacOS/cadence"));
        assert_nothing_mounted();
    }
}
