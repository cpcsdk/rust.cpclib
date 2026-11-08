//! Source-level debugging for Amstrad CPC assembly, over the Debug Adapter
//! Protocol.
//!
//! The emulator we drive (1984js) already implements a DAP server - stepping,
//! memory, disassembly, instruction breakpoints. What it cannot do is talk
//! about *source*: its `stackTrace` returns `line: 0` and an address, and it
//! only accepts breakpoints as addresses. Everything an editor wants is phrased
//! in files and lines.
//!
//! So this is not a debugger. It is the translation between the two, sitting
//! between the editor (north) and the emulator (south):
//!
//! * requests that are purely about source - `setBreakpoints` - are answered
//!   here, turned into addresses, and forwarded as `setInstructionBreakpoints`;
//! * answers that mention an address - `stackTrace` - are annotated on the way
//!   back with the file and line that address came from;
//! * everything else - stepping, memory, disassembly - is forwarded untouched.
//!
//! The mapping comes from [`cpclib_project::srcmap::SourceMap`], built by the
//! assembler during its listing pass.

#[cfg(feature = "adapter")]
pub mod ace;
#[cfg(feature = "adapter")]
mod advanced_breakpoints;
pub mod breakpoint_manager;
#[cfg(feature = "adapter")]
pub mod amspiritlite;
#[cfg(feature = "adapter")]
pub mod bemu;
#[cfg(feature = "adapter")]
pub mod basic;
#[cfg(feature = "adapter")]
pub mod basic_session;
pub mod callstack;
pub mod disassemble;
pub mod inspect;
#[cfg(feature = "adapter")]
pub mod js1984_bridge;
#[cfg(feature = "adapter")]
pub mod launch;
#[cfg(feature = "adapter")]
pub mod peer;
pub mod protocol;
mod text;
#[cfg(feature = "adapter")]
pub mod session;
#[cfg(feature = "adapter")]
pub mod sugarbox;
#[cfg(feature = "adapter")]
mod adapter;
#[cfg(feature = "adapter")]
pub use adapter::run_stdio;
