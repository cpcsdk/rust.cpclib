//! Small serializable tag stashed in `CallHierarchyItem.data`, round-tripped
//! between `textDocument/prepareCallHierarchy` and
//! `callHierarchy/incomingCalls`/`callHierarchy/outgoingCalls` - those two
//! only receive the `CallHierarchyItem` back, not the original request
//! context, so this is how `backend.rs` and the two domain analyzers learn
//! what kind of "function" a given item actually is (a basm label, or a
//! BASIC line - standalone or embedded in a `LOCOMOTIVE` block) without
//! re-parsing its `name`/`range`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CallHierarchyData {
    /// A basm global label (see `label_scope_at_line`). `name` is the
    /// label's own spelling as written; comparisons elsewhere are
    /// case-insensitive.
    AsmLabel { name: String },
    /// A Locomotive BASIC line. `block_start_line` is `Some(doc_line)` (the
    /// document line of the `LOCOMOTIVE` block's first BASIC-content line,
    /// 0-based - the same value threaded through `locomotive_basic_*` twin
    /// functions elsewhere) when this line is embedded in a `.asm` file, or
    /// `None` for a standalone `.bas` document.
    BasicLine {
        line_number: u16,
        block_start_line: Option<u32>
    }
}

impl CallHierarchyData {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(value.clone()).ok()
    }
}
