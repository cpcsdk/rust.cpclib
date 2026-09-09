use zed::*;
use zed_extension_api as zed;

/// The one debug adapter this extension registers - see
/// `[debug_adapters.cpclib-dap]` in `extension.toml` and
/// `debug_adapter_schemas/cpclib-dap.json` (the per-field config schema,
/// mirroring `cpclib-vscode/package.json`'s own `debuggers[0]
/// .configurationAttributes.launch.properties`, minus the fields - like
/// `openInWebview` - that only mean something to VS Code's own webview).
const DEBUG_ADAPTER_NAME: &str = "cpclib-dap";

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

    /// Resolves the process to spawn for a debug session: the same
    /// `cpclib-lsp` binary the language server itself uses, run as its
    /// `dap` subcommand - mirrors `cpclib-vscode/src/debug/
    /// debugAdapterFactory.ts`'s `DebugAdapterExecutable(resolveAdapterPath
    /// (), ['dap'], ...)` exactly, down to reusing the very same binary
    /// resolution (`worktree.which`) as `language_server_command` above, so
    /// nothing extra needs installing beyond `cpclib-lsp` itself.
    fn get_dap_binary(
        &mut self,
        adapter_name: String,
        config: DebugTaskDefinition,
        user_provided_debug_adapter_path: Option<String>,
        worktree: &zed::Worktree,
    ) -> Result<DebugAdapterBinary> {
        if adapter_name != DEBUG_ADAPTER_NAME {
            return Err(format!("unknown debug adapter: {adapter_name}"));
        }

        let command = user_provided_debug_adapter_path
            .or_else(|| worktree.which("cpclib-lsp"))
            .ok_or_else(|| {
                "cpclib-lsp not found in PATH. Please install it with: cargo install --path cpclib-lsp".to_string()
            })?;

        Ok(DebugAdapterBinary {
            command: Some(command),
            arguments: vec!["dap".to_string()],
            envs: Vec::new(),
            cwd: None,
            connection: None,
            request_args: StartDebuggingRequestArguments {
                configuration: config.config,
                request: StartDebuggingRequestArgumentsRequest::Launch,
            },
        })
    }

    /// cpclib-dap only ever launches a fresh session - there is no running
    /// process on a real (or emulated) CPC to attach to, so this never
    /// needs to actually inspect `config`.
    fn dap_request_kind(
        &mut self,
        adapter_name: String,
        _config: serde_json::Value,
    ) -> Result<StartDebuggingRequestArgumentsRequest> {
        if adapter_name != DEBUG_ADAPTER_NAME {
            return Err(format!("unknown debug adapter: {adapter_name}"));
        }
        Ok(StartDebuggingRequestArgumentsRequest::Launch)
    }

    /// Translates Zed's generic "New Process Debugger" launch config (a
    /// program/cwd/args/envs shape meant for spawning an arbitrary
    /// executable) into cpclib-dap's own JSON (see
    /// `debug_adapter_schemas/cpclib-dap.json`): the generic `program`
    /// becomes our `program` (the .asm/.bas/.sna/.dsk file to debug), and
    /// the generic `stop_on_entry` (Zed's own "stop on entry" toggle) is
    /// passed straight through. `cwd`/`args`/`envs` have no equivalent in
    /// our schema - cpclib-dap resolves paths relative to `program`/
    /// `buildFile`, not a working directory, and takes no argv - so they're
    /// dropped rather than guessed at. A user who needs `rule`/`buildFile`/
    /// `emulator`/`watchLabels`/`topOfStack` sets them directly in a
    /// `.zed/debug.json` scenario instead of through this generic form.
    fn dap_config_to_scenario(&mut self, config: DebugConfig) -> Result<DebugScenario> {
        if config.adapter != DEBUG_ADAPTER_NAME {
            return Err(format!("unknown debug adapter: {}", config.adapter));
        }

        let DebugRequest::Launch(launch) = config.request
        else {
            return Err("cpclib-dap only supports launching a fresh session, not attaching to one".to_string());
        };

        let mut adapter_config = serde_json::json!({
            "request": "launch",
            "program": launch.program,
        });
        if let Some(stop_on_entry) = config.stop_on_entry {
            adapter_config["stopOnEntry"] = serde_json::json!(stop_on_entry);
        }

        Ok(DebugScenario {
            label: config.label,
            adapter: config.adapter,
            build: None,
            config: adapter_config.to_string(),
            tcp_connection: None,
        })
    }
}

zed::register_extension!(CpcLibExtension);
