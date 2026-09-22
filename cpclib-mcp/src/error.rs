//! Normalized tool-error envelope.
//!
//! Every tool in this crate returns `Result<Value, ToolError>` from its
//! plain (non-MCP) business-logic function. `ToolError` is what turns into
//! the `{"tool_error": true, "kind": ..., "message": ..., "details": ...}`
//! JSON object the MCP layer (`tools/mod.rs`) reports back to the agent via
//! `isError: true` content - never a protocol-level error, so the agent can
//! see *why* something failed and act on it. Structured source errors
//! (`AssemblerError`, `BndBuilderError`, `BasicError`, ...) keep their
//! useful fields in `details`; plain-`String` errors (`Robot`, `Disc`,
//! `Snapshot::load`) pass through as `message` with `details: null`.

use serde::Serialize;
use serde_json::Value;

/// Which family of underlying operation failed - lets an agent branch on
/// failure kind without string-matching `message`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolErrorKind {
    Assembler,
    Build,
    Basic,
    Snapshot,
    Amsdos,
    Disc,
    Robot,
    Io,
    InvalidInput,
    /// The tool itself panicked - a bug in the server, not in the request.
    Internal
}

impl ToolErrorKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Assembler => "assembler",
            Self::Build => "build",
            Self::Basic => "basic",
            Self::Snapshot => "snapshot",
            Self::Amsdos => "amsdos",
            Self::Disc => "disc",
            Self::Robot => "robot",
            Self::Io => "io",
            Self::InvalidInput => "invalid_input",
            Self::Internal => "internal"
        }
    }
}

/// The normalized error envelope every tool reports through `isError`.
#[derive(Debug, Clone, Serialize)]
pub struct ToolError {
    /// Always `true` - a stable marker so an agent can recognize this shape
    /// regardless of which tool produced it.
    pub tool_error: bool,
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>
}

impl ToolError {
    pub fn new(kind: ToolErrorKind, message: impl Into<String>) -> Self {
        Self {
            tool_error: true,
            kind: kind.as_str().to_string(),
            message: message.into(),
            details: None
        }
    }

    pub fn with_details(kind: ToolErrorKind, message: impl Into<String>, details: Value) -> Self {
        Self {
            tool_error: true,
            kind: kind.as_str().to_string(),
            message: message.into(),
            details: Some(details)
        }
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::InvalidInput, message)
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::Io, message)
    }

    /// Render as the exact JSON object reported to the agent.
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| {
            serde_json::json!({
                "tool_error": true,
                "kind": "io",
                "message": "failed to serialize the original error"
            })
        })
    }
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.kind, self.message)
    }
}

impl std::error::Error for ToolError {}

pub type ToolResult = Result<Value, ToolError>;
