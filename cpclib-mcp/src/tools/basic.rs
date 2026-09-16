//! `basic_tokenize`/`basic_detokenize` - thin wrappers over
//! `cpclib_basic::BasicProgram`.

use base64::Engine;
use cpclib_basic::BasicProgram;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::{Value, json};

use crate::McpServer;
use crate::error::{ToolError, ToolErrorKind, ToolResult};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BasicTokenizeInput {
    /// Locomotive BASIC source text.
    pub code: String
}

/// Tokenize Locomotive BASIC source text into its on-disk/in-memory byte
/// form. Read-only.
pub(crate) fn basic_tokenize(input: BasicTokenizeInput) -> ToolResult {
    let program = BasicProgram::parse(&input.code)
        .map_err(|e| ToolError::new(ToolErrorKind::Basic, e.to_string()))?;
    let bytes = program.as_bytes();
    Ok(json!({
        "size": bytes.len(),
        "data_base64": base64::engine::general_purpose::STANDARD.encode(bytes)
    }))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BasicDetokenizeInput {
    /// Base64-encoded tokenized Locomotive BASIC bytes.
    pub data_base64: String
}

/// Decode tokenized Locomotive BASIC bytes back into source text. Read-only.
pub(crate) fn basic_detokenize(input: BasicDetokenizeInput) -> ToolResult {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&input.data_base64)
        .map_err(|e| ToolError::invalid_input(format!("data_base64 is not valid base64: {e}")))?;
    let program = BasicProgram::decode(&bytes)
        .map_err(|e| ToolError::new(ToolErrorKind::Basic, e.to_string()))?;
    Ok(json!({ "code": program.to_string() }))
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Value>, Json<Value>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = basic_router, vis = "pub(crate)")]
impl McpServer {
    #[tool(description = "Tokenize Locomotive BASIC source text into its on-disk byte form, \
                           base64-encoded. Read-only.")]
    async fn basic_tokenize(
        &self,
        Parameters(input): Parameters<BasicTokenizeInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(basic_tokenize(input))
    }

    #[tool(description = "Decode tokenized Locomotive BASIC bytes (base64-encoded) back into \
                           source text. Read-only.")]
    async fn basic_detokenize(
        &self,
        Parameters(input): Parameters<BasicDetokenizeInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(basic_detokenize(input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_then_detokenize_round_trips() {
        let code = "10 PRINT \"HELLO\"\n20 GOTO 10\n";
        let tokenized = basic_tokenize(BasicTokenizeInput {
            code: code.to_string()
        })
        .expect("a simple BASIC program should tokenize cleanly");
        let data_base64 = tokenized["data_base64"].as_str().unwrap().to_string();
        assert!(tokenized["size"].as_u64().unwrap() > 0);

        let detokenized =
            basic_detokenize(BasicDetokenizeInput { data_base64 }).expect("should decode back");
        let round_tripped = detokenized["code"].as_str().unwrap();
        assert!(round_tripped.contains("PRINT"), "{round_tripped}");
        assert!(round_tripped.contains("GOTO"), "{round_tripped}");
    }

    #[test]
    fn detokenize_rejects_invalid_base64() {
        let err = basic_detokenize(BasicDetokenizeInput {
            data_base64: "not valid base64 !!!".to_string()
        })
        .expect_err("garbage input should be rejected");
        assert_eq!(err.kind, "invalid_input");
    }
}
