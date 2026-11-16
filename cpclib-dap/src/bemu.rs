//! bemu (`amsrust`), a pin-level CPC emulator serving an AMSpiriT-Lite
//! compatible debug API (`--debug-server`): debugged through the same
//! [`crate::amspiritlite::AmspiritLitePeer`], started here with the program
//! loaded and **paused** (`--debug-paused`), so every breakpoint is armed
//! before the first instruction runs; the session releases it at
//! `configurationDone` (see [`crate::peer::DapPeer::release`]).
//!
//! Two ways to show it:
//!
//! - **in an editor tab**, the default: its web build (downloaded by
//!   bndbuild from the releases of github.com/rgiot/bemu, or
//!   `$CPCLIB_BEMU_WEB`) served over loopback by
//!   [`cpclib_runner::web::bemu`], which relays the debug API to the page;
//! - **in its own window** (`"bemuWindow": true`): the native executable,
//!   the launch configuration's `emulatorPath`, else `$CPCLIB_BEMU`, else the
//!   one bndbuild installs (downloaded from the same releases if missing),
//!   else `amsrust` from the `PATH`.
//!
//! `"headless": true` starts the native one without a window (tests, CI).

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::amspiritlite::{port_to_serve_on, wait_until_listening};

/// Whether bemu is shown in an editor tab (its web build) rather than its
/// own window: unless the launch asks for the window (`"bemuWindow": true`),
/// or for none (`"headless": true`), or names an executable (`emulatorPath`).
pub fn in_tab(arguments: &Value) -> bool {
    let asked = |name: &str| arguments.get(name).and_then(Value::as_bool) == Some(true);
    !asked("bemuWindow")
        && !asked("headless")
        && arguments
            .get("emulatorPath")
            .and_then(Value::as_str)
            .is_none()
}

/// How long a launch waits for bemu's page to be opened and its emulator to
/// start.
pub const PAGE_OPEN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

/// The executable named by the launch configuration or the environment.
fn named_executable(arguments: &Value) -> Option<PathBuf> {
    arguments
        .get("emulatorPath")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("CPCLIB_BEMU").map(PathBuf::from))
}

/// The executable to start: the one named, else bndbuild's (installed if
/// it is not yet), else `amsrust` from the `PATH` - with why bndbuild's
/// could not be installed then, for the error if that one does not start
/// either.
pub fn executable(arguments: &Value) -> (PathBuf, Option<String>) {
    if let Some(named) = named_executable(arguments) {
        return (named, None);
    }
    use cpclib_runner::runner::emulator::Emulator;
    use cpclib_runner::runner::emulator::bemu::BemuVersion;
    let configuration = Emulator::Bemu(BemuVersion::default())
        .configuration::<cpclib_common::event::DiscardObserver>();
    let installed = if configuration.is_cached() {
        Ok(())
    }
    else {
        configuration.install(&cpclib_common::event::DiscardObserver)
    };
    match installed {
        Ok(()) => (configuration.exec_fname().into_std_path_buf(), None),
        Err(e) => (PathBuf::from("amsrust"), Some(e))
    }
}

/// bemu's web build in a page, loaded with `load`, the machine configured
/// by the launch's `bemuConfig` (`Bemu.create`'s configuration: `model`,
/// `crtc`, `ramKb`...). The page is the caller's to open; its API is the
/// returned server's endpoint.
pub fn serve_in_tab(
    arguments: &Value,
    load: Load
) -> Result<cpclib_runner::web::bemu::BemuPage, String> {
    use cpclib_runner::web::bemu;
    let package = bemu::package_root()?;
    let load = match load {
        Load::Snapshot(snapshot) => bemu::Load::Snapshot(snapshot.to_vec()),
        Load::Disk(disk) => {
            bemu::Load::Disc(
                fs_err::read(disk).map_err(|e| format!("cannot read {}: {e}", disk.display()))?
            )
        },
        Load::Boot => bemu::Load::Boot
    };
    let config = arguments
        .get("bemuConfig")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    bemu::serve(&package, load, config).map_err(|e| format!("cannot serve bemu's page: {e}"))
}

/// What bemu starts with.
pub enum Load<'a> {
    /// A snapshot: loaded and held paused until released.
    Snapshot(&'a [u8]),
    /// A disc in drive A, the machine booting (running) as usual.
    Disk(&'a Path),
    /// A cold boot to BASIC (running), for a native BASIC session.
    Boot
}

/// Start bemu with its debug server on `port` (or a free one if that is
/// taken), in a window unless the launch says `"headless": true`. Returns
/// the endpoint and the process.
pub fn launch(
    arguments: &Value,
    load: Load,
    port: u16
) -> Result<(String, std::process::Child), String> {
    let (program, not_installed) = executable(arguments);
    let port = port_to_serve_on(port);
    let mut command = std::process::Command::new(&program);
    let mut snapshot_file = None;
    match load {
        Load::Snapshot(snapshot) => {
            let path =
                std::env::temp_dir().join(format!("cpclib-dap-bemu-{}.sna", std::process::id()));
            fs_err::write(&path, snapshot)
                .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
            command.arg("--sna").arg(&path).arg("--debug-paused");
            snapshot_file = Some(path);
        },
        Load::Disk(disk) => {
            command.arg("--dsk").arg(disk).arg("--debug-server");
        },
        Load::Boot => {
            command.arg("--debug-server");
        }
    }
    command.arg("--debug-port").arg(port.to_string());
    if arguments.get("headless").and_then(Value::as_bool) != Some(true) {
        command.arg("--window");
    }
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    crate::amspiritlite::strip_snap_leaked_env_vars(&mut command);
    let mut child = command.spawn().map_err(|e| {
        let download = not_installed
            .map(|why| format!(" (bndbuild could not download it: {why})"))
            .unwrap_or_default();
        format!(
            "cannot start bemu ({}): {e}{download} - set \"emulatorPath\" in the launch \
             configuration, or $CPCLIB_BEMU, to its amsrust executable",
            program.display()
        )
    })?;

    let endpoint = format!("http://127.0.0.1:{port}");
    let listening = wait_until_listening(&endpoint, std::time::Duration::from_secs(30));
    // Read by then: bemu loads the snapshot before it serves.
    if let Some(path) = snapshot_file {
        let _ = fs_err::remove_file(path);
    }
    if let Err(e) = listening {
        // Dropping a `Child` does not stop it: a bemu that never served would
        // outlive the failed launch.
        let _ = child.kill();
        let _ = child.wait();
        return Err(e.replace("AMSpiriT Lite", "bemu"));
    }
    Ok((endpoint, child))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn the_executable_is_the_launch_configurations_first() {
        let named = json!({ "emulatorPath": "/opt/bemu/amsrust" });
        assert_eq!(
            executable(&named),
            (PathBuf::from("/opt/bemu/amsrust"), None)
        );
    }

    #[test]
    fn bemu_is_in_a_tab_unless_asked_otherwise() {
        assert!(in_tab(&json!({})));
        assert!(in_tab(&json!({ "bemuWindow": false })));
        assert!(!in_tab(&json!({ "bemuWindow": true })));
        assert!(!in_tab(&json!({ "headless": true })));
        assert!(!in_tab(&json!({ "emulatorPath": "/opt/bemu/amsrust" })));
    }

    #[test]
    fn a_missing_executable_says_how_to_name_it() {
        let arguments = json!({ "emulatorPath": "/nonexistent/amsrust", "headless": true });
        let error = launch(&arguments, Load::Boot, 0).unwrap_err();
        assert!(error.contains("emulatorPath"), "{error}");
    }
}
