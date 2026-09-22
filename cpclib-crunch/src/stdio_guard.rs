//! Give a native cruncher's raw `printf` (fd 1) somewhere harmless to go,
//! while keeping a private handle to the *real* stdout for a CLI's own
//! report.
//!
//! The same hazard and fix `cpclib-mcp`'s protocol stream needed (see its
//! own `main.rs`), just for readable text instead of JSON-RPC frames: a
//! vendored C/C++ cruncher (Shrinkler in particular, confirmed live -
//! `size_map`'s own progress table and "Verifying... OK" line landed
//! straight in the middle of this binary's report) prints its own progress
//! directly to the real process stdout, bypassing Rust's `io::stdout()`
//! entirely - so nothing this crate does with its own `Write` calls can
//! stop it from interleaving. Moving *our* output to a private duplicate of
//! the original stdout, and pointing descriptor 1 (what the native
//! `printf` actually writes to) at stderr instead, is the only thing that
//! separates the two reliably.
//!
//! Call [`private_stdout`] once, as early as possible in `main` - before
//! any cruncher can run - and write a CLI's own output through the handle
//! it returns (falling back to plain `io::stdout()` on a platform or a
//! failure this doesn't support, in which case a cruncher's own noise may
//! still leak into the report - the honest degraded behavior, not a
//! silent corruption).

/// SAFETY: plain fd duplication at process start, before any other thread
/// touches stdout; the duplicate is owned exclusively by the returned File.
#[cfg(unix)]
pub fn private_stdout() -> Option<std::fs::File> {
    use std::os::fd::FromRawFd;

    unsafe {
        let dup = libc::dup(1);
        if dup < 0 || libc::dup2(2, 1) < 0 {
            return None;
        }
        Some(std::fs::File::from_raw_fd(dup))
    }
}

/// Windows counterpart of the Unix version above. Two layers of "stdout"
/// exist and both must be redirected - see `cpclib-mcp`'s own copy of this
/// same code for the full reasoning (Win32 standard handle vs. the C
/// runtime's descriptor 1, which is what a vendored C/C++ cruncher's
/// `printf` actually writes to). Untested - no Windows environment was
/// available to verify it live, unlike the Unix path above.
#[cfg(windows)]
pub fn private_stdout() -> Option<std::fs::File> {
    use std::os::windows::io::FromRawHandle;

    use windows_sys::Win32::Foundation::{DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Console::{GetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    unsafe extern "C" {
        fn _dup2(source: i32, target: i32) -> i32;
    }

    // SAFETY: Win32/CRT calls at process start; every handle passed is either
    // a standard handle just queried or the duplicate created below, whose
    // ownership moves into the returned File.
    unsafe {
        let stdout = GetStdHandle(STD_OUTPUT_HANDLE);
        let stderr = GetStdHandle(STD_ERROR_HANDLE);
        let unusable = |h: HANDLE| h.is_null() || h == INVALID_HANDLE_VALUE;
        if unusable(stdout) || unusable(stderr) {
            return None;
        }

        let mut private: HANDLE = std::ptr::null_mut();
        let process = GetCurrentProcess();
        if DuplicateHandle(process, stdout, process, &mut private, 0, 0, DUPLICATE_SAME_ACCESS) == 0 || unusable(private) {
            return None;
        }

        if _dup2(2, 1) < 0 {
            return None;
        }
        SetStdHandle(STD_OUTPUT_HANDLE, stderr);

        Some(std::fs::File::from_raw_handle(private as _))
    }
}

#[cfg(not(any(unix, windows)))]
pub fn private_stdout() -> Option<std::fs::File> {
    None
}
