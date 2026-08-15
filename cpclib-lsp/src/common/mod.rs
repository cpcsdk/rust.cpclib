//! Code shared by every language module: the document model and generic
//! rendering helpers (markdown/hover construction).

pub mod call_hierarchy;
pub mod colors;
pub mod document;
pub mod firmware_docs;
pub mod render;
pub mod symbols;

// Moved to `cpclib-project`, which the debug adapter shares. Re-exported under
// the old paths so the call sites that only ever wanted "the config" or "walk
// the workspace" do not all have to change.
pub use cpclib_project::{config, walk};
