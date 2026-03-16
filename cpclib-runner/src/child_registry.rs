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

fn kill_pid(pid: u32) {
    #[cfg(unix)]
    {
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status();
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .status();
    }
}
