use zed::*;
use zed_extension_api as zed;

struct CpcLibExtension {}

fn get_path_to_language_server_executable() -> Result<String> {
    Ok("/home/romain/.cargo/bin/cpclib-lsp".to_string())
}

fn get_args_for_language_server() -> Result<Vec<String>> {
    Ok(Vec::with_capacity(0))
}

fn get_env_for_language_server() -> Result<Vec<(String, String)>> {
    Ok(Vec::with_capacity(0))
}

impl zed::Extension for CpcLibExtension {
    fn new() -> Self {
        Self {}
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree
    ) -> Result<zed::Command> {
        dbg!(language_server_id);
        dbg!(worktree);

        Ok(zed::Command {
            command: get_path_to_language_server_executable()?,
            args: get_args_for_language_server()?,
            env: get_env_for_language_server()?
        })
    }
}

zed::register_extension!(CpcLibExtension);
