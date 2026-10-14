//! bemu (`amsrust`), a pin-level CPC emulator serving an AMSpiriT-Lite
//! compatible debug API (`--debug-server`): debugged through the same
//! [`crate::amspiritlite::AmspiritLitePeer`], started here with the program
//! loaded and **paused** (`--debug-paused`), so every breakpoint is armed
//! before the first instruction runs; the session releases it at
//! `configurationDone` (see [`crate::peer::DapPeer::release`]).
//!
//! Not downloaded: bemu is built from its own repository. The executable is
//! the launch configuration's `emulatorPath`, else `$CPCLIB_BEMU`, else
//! `amsrust` from the `PATH`.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::amspiritlite::{port_to_serve_on, wait_until_listening};

/// The executable to start.
pub fn executable(arguments: &Value) -> PathBuf {
    arguments
        .get("emulatorPath")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("CPCLIB_BEMU").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("amsrust"))
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
pub fn launch(arguments: &Value, load: Load, port: u16) -> Result<(String, std::process::Child), String> {
    let program = executable(arguments);
    let port = port_to_serve_on(port);
    let mut command = std::process::Command::new(&program);
    let mut snapshot_file = None;
    match load {
        Load::Snapshot(snapshot) => {
            let path = std::env::temp_dir().join(format!("cpclib-dap-bemu-{}.sna", std::process::id()));
            fs_err::write(&path, snapshot).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
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
        format!(
            "cannot start bemu ({}): {e} - set \"emulatorPath\" in the launch configuration, \
             or $CPCLIB_BEMU, to its amsrust executable",
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
        assert_eq!(executable(&named), PathBuf::from("/opt/bemu/amsrust"));
    }

    #[test]
    fn a_missing_executable_says_how_to_name_it() {
        let arguments = json!({ "emulatorPath": "/nonexistent/amsrust", "headless": true });
        let error = launch(&arguments, Load::Boot, 0).unwrap_err();
        assert!(error.contains("emulatorPath"), "{error}");
    }
}
