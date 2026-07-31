use zed::*;
use zed_extension_api as zed;

struct CpcLibExtension {}

impl zed::Extension for CpcLibExtension {
    fn new() -> Self {
        Self {}
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree
    ) -> Result<zed::Command> {
        // Use Zed's which() to find the binary in PATH
        // This properly handles PATH lookup from the WASM environment
        let command = worktree
            .which("cpclib-lsp")
            .ok_or_else(|| {
                "cpclib-lsp not found in PATH. Please install it with: cargo install --path cpclib-lsp".to_string()
            })?;

        Ok(zed::Command {
            command,
            args: vec![],
            env: Default::default(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        // Enable all LSP features including semantic tokens
        Ok(Some(serde_json::json!({
            "semanticTokens": {
                "enable": true
            }
        })))
    }
}

zed::register_extension!(CpcLibExtension);
