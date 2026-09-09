//! Re-exports the DAP wire-protocol helpers now living in `cpclib-runner`
//! (`cpclib_runner::web::dap_protocol`), so they're defined once rather than
//! once here and once for this crate's own `Robot` automation of 1984js.
//! Kept as a module (not a straight `use` at every call site) so the many
//! existing `crate::protocol::...`/`protocol::...` references across this
//! crate don't all need touching.

pub use cpclib_runner::web::dap_protocol::*;
