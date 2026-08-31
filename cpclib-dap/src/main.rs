//! The debug adapter, speaking DAP over stdio.
//!
//! Stdout carries protocol frames and nothing else - the same discipline the
//! language server keeps - so every diagnostic goes to stderr.

fn main() -> std::io::Result<()> {
    eprintln!("cpclib-dap: reading DAP on stdin, writing on stdout");
    cpclib_dap::run_stdio()
}
