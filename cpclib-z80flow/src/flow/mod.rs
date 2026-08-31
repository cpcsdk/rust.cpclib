//! The successor relation both control-flow views are built on.
//!
//! See the crate doc comment for why there are two views and not one walker:
//! the consumers genuinely disagree about what a `CALL`, a `RET` or a back-edge
//! means, and those disagreements are deliberate. [`Policy`] makes each one an
//! explicit choice instead of a difference between two bodies of code.

pub(crate) mod jump;
pub(crate) mod successors;
