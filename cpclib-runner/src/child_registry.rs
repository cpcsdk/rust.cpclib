use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

/// Global set of PIDs for child processes currently running.
/// Populated by `ExternRunner::inner_run`; used by `kill_all_children`.
static CHILD_PID_REGISTRY: LazyLock<Mutex<HashSet<u32>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

pub(crate) fn register_child_pid(pid: u32) {
    CHILD_PID_REGISTRY.lock().unwrap().insert(pid);
}

pub(crate) fn deregister_child_pid(pid: u32) {
    CHILD_PID_REGISTRY.lock().unwrap().remove(&pid);
}

/// Kill every child process still tracked in the registry.
/// Call this before exiting the parent process to avoid orphaned emulators.
pub fn kill_all_children() {
    let pids: Vec<u32> = CHILD_PID_REGISTRY.lock().unwrap().iter().copied().collect();
    for pid in pids {
        kill_pid(pid);
    }
}

/// Every process below `pid`, children first. An AppImage - the way most of
/// the emulators are shipped on Linux - is a launcher that starts the real
/// emulator as its child: killing the launcher alone leaves the emulator
/// running (and holding its debug port, so the next launch cannot reach its own).
#[cfg(unix)]
fn descendants_of(pid: u32) -> Vec<u32> {
    let children: Vec<u32> = std::process::Command::new("pgrep")
        .args(["-P", &pid.to_string()])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .split_whitespace()
                .filter_map(|p| p.parse().ok())
                .collect()
        })
        .unwrap_or_default();
    let mut all = Vec::new();
    for child in children {
        all.extend(descendants_of(child));
        all.push(child);
    }
    all
}

/// Kills `pid` and, on Unix, everything it started.
pub(crate) fn kill_pid(pid: u32) {
    #[cfg(unix)]
    {
        // the descendants are listed first: once the parent is gone they are
        // re-parented, and no longer found below it
        let victims: Vec<u32> = descendants_of(pid)
            .into_iter()
            .chain(std::iter::once(pid))
            .collect();
        for victim in victims {
            let _ = std::process::Command::new("kill")
                .args(["-9", &victim.to_string()])
                .status();
        }
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .status();
    }
}
