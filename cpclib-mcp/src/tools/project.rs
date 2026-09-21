//! `project_context` - what the build actually feeds `basm`, so an agent can
//! analyse a file the way the project assembles it.
//!
//! The static tools (`suggest_optimizations`, `search_reorderings`, ...) see
//! one file at a time and need the include directories and `-D` defines the
//! build passes on its command line, or a conditional file (`if
//! LINKED_VERSION`) does not even assemble. Those are already written down
//! in the build file's `basm` command lines - this reads them out, resolved
//! against the build file's directory, in exactly the shape the other tools'
//! `include_dirs` / `defines` inputs take.

use camino::{Utf8Path, Utf8PathBuf};
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::{Value, json};

use crate::McpServer;
use crate::error::{ToolError, ToolResult};
use crate::tools::build::open_builder;

/// Splits a command line into words, honouring single and double quotes.
fn split_words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut in_word = false;
    for c in line.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => current.push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                in_word = true;
            },
            (None, c) if c.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut current));
                    in_word = false;
                }
            },
            (None, c) => {
                current.push(c);
                in_word = true;
            }
        }
    }
    if in_word {
        words.push(current);
    }
    words
}

/// One `basm` command line, decomposed.
#[derive(Debug, PartialEq, Eq)]
struct BasmInvocation {
    inputs: Vec<String>,
    include_dirs: Vec<String>,
    defines: Vec<String>,
    outputs: Vec<String>
}

/// Decomposes `command` when it is a `basm` call (`basm`, `extern basm`,
/// `basm.exe`); `None` for any other command.
fn parse_basm_command(command: &str) -> Option<BasmInvocation> {
    let mut words = split_words(command);
    if words.first().is_some_and(|w| w == "extern") {
        words.remove(0);
    }
    let program = words.first()?;
    let name = program.rsplit(['/', '\\']).next()?;
    if !(name == "basm" || name == "basm.exe") {
        return None;
    }

    let mut inv = BasmInvocation {
        inputs: Vec::new(),
        include_dirs: Vec::new(),
        defines: Vec::new(),
        outputs: Vec::new()
    };
    let mut args = words.into_iter().skip(1);
    while let Some(arg) = args.next() {
        // Options taking a value, as `-X value`, `-Xvalue` or `--long=value`.
        let take = |short: &str, long: &str, arg: &str, args: &mut dyn Iterator<Item = String>| -> Option<String> {
            if arg == short || arg == long {
                return args.next();
            }
            if let Some(v) = arg.strip_prefix(&format!("{long}=")) {
                return Some(v.to_string());
            }
            arg.strip_prefix(short).filter(|v| !v.is_empty() && !short.starts_with("--")).map(str::to_string)
        };
        if let Some(v) = take("-I", "--include", &arg, &mut args) {
            inv.include_dirs.push(v);
        }
        else if let Some(v) = take("-D", "--define", &arg, &mut args) {
            inv.defines.push(v);
        }
        else if let Some(v) = take("-o", "--output", &arg, &mut args) {
            inv.outputs.push(v);
        }
        else if arg == "--sym" || arg == "--lst" || arg == "--listing" || arg == "--symbols_output" {
            if let Some(v) = args.next() {
                inv.outputs.push(v);
            }
        }
        else if !arg.starts_with('-') {
            inv.inputs.push(arg);
        }
    }
    Some(inv)
}

/// Resolves `input` the way `basm` would: relative to the working
/// directory first, then each include directory.
fn resolve_input(base: &Utf8Path, input: &str, include_dirs: &[Utf8PathBuf]) -> Option<Utf8PathBuf> {
    std::iter::once(base.to_path_buf())
        .chain(include_dirs.iter().cloned())
        .map(|dir| dir.join(input))
        .find(|p| p.is_file())
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectContextInput {
    /// Path to the build file (e.g. `build.bnd`/`bndbuild.yml`) or a
    /// directory containing one.
    pub bnd_path: String,
    /// Only rules producing this target; default: every rule.
    pub target: Option<String>
}

pub(crate) fn project_context(input: ProjectContextInput) -> ToolResult {
    let (resolved, builder) = open_builder(&input.bnd_path)?;
    let base = resolved
        .parent()
        .map(Utf8Path::to_path_buf)
        .unwrap_or_else(|| Utf8PathBuf::from("."));
    let base = if base.is_relative() {
        Utf8PathBuf::from_path_buf(std::env::current_dir().map_err(|e| ToolError::io(e.to_string()))?)
            .map_err(|_| ToolError::io("non-UTF-8 working directory"))?
            .join(&base)
    }
    else {
        base
    };

    let mut rules = Vec::new();
    for rule in builder.rules() {
        if let Some(t) = &input.target
            && !rule.targets().iter().any(|x| x.as_str() == t)
        {
            continue;
        }
        let mut basm_calls = Vec::new();
        let mut other_commands = Vec::new();
        for task in rule.commands() {
            let command = task.to_string();
            match parse_basm_command(&command) {
                Some(inv) => {
                    let include_dirs: Vec<Utf8PathBuf> =
                        inv.include_dirs.iter().map(|d| base.join(d)).collect();
                    let resolved_inputs: Vec<Value> = inv
                        .inputs
                        .iter()
                        .map(|i| json!(resolve_input(&base, i, &include_dirs).map(|p| p.to_string())))
                        .collect();
                    basm_calls.push(json!({
                        "command": command,
                        "inputs": inv.inputs,
                        "resolved_inputs": resolved_inputs,
                        "include_dirs": include_dirs.iter().map(|d| d.as_str()).collect::<Vec<_>>(),
                        "defines": inv.defines,
                        "outputs": inv.outputs
                    }));
                },
                None => other_commands.push(command)
            }
        }
        rules.push(json!({
            "targets": rule.targets().iter().map(|t| t.as_str()).collect::<Vec<_>>(),
            "dependencies": rule.dependencies().iter().map(|d| d.as_str()).collect::<Vec<_>>(),
            "basm": basm_calls,
            "other_commands": other_commands
        }));
    }
    Ok(json!({
        "bnd_path": resolved.as_str(),
        "working_directory": base.as_str(),
        "note": "commands run from working_directory; pass a basm call's `include_dirs` and \
                 `defines` to the analysis tools' fields of the same names",
        "rules": rules
    }))
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Value>, Json<Value>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = project_router, vis = "pub(crate)")]
impl McpServer {
    #[tool(description = "Read-only: for each rule of a build file, the `basm` command lines it \
                           runs, decomposed into inputs (with the real file each resolves to), \
                           include directories (absolute) and -D defines (e.g. \
                           LINKED_VERSION=1) - exactly what to pass as `include_dirs` / \
                           `defines` to suggest_optimizations / search_reorderings so a \
                           conditional or multi-file project is analysed the way the build \
                           assembles it.")]
    async fn project_context(
        &self,
        Parameters(input): Parameters<ProjectContextInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(project_context(input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etchys_real_command_line_is_decomposed() {
        let inv = parse_basm_command(
            "basm -I src main.asm -o etch_a_sketch.sna --override -DLINKED_VERSION=0"
        )
        .unwrap();
        assert_eq!(inv.inputs, vec!["main.asm"]);
        assert_eq!(inv.include_dirs, vec!["src"]);
        assert_eq!(inv.defines, vec!["LINKED_VERSION=0"]);
        assert_eq!(inv.outputs, vec!["etch_a_sketch.sna"]);
    }

    #[test]
    fn option_spellings_and_quoting_are_handled() {
        let inv = parse_basm_command(
            "extern basm --include=a -I b --define FOO=\"x y\" -D BAR src/link.asm --sym out.sym --snapshot --output 'my file.sna'"
        )
        .unwrap();
        assert_eq!(inv.include_dirs, vec!["a", "b"]);
        assert_eq!(inv.defines, vec!["FOO=x y", "BAR"]);
        assert_eq!(inv.inputs, vec!["src/link.asm"]);
        assert_eq!(inv.outputs, vec!["out.sym", "my file.sna"]);
    }

    #[test]
    fn other_commands_are_not_basm() {
        assert_eq!(parse_basm_command("dsk SKY.DSK format --format data42"), None);
        assert_eq!(parse_basm_command("-rm *.sym"), None);
        assert!(parse_basm_command("/usr/bin/basm x.asm").is_some());
    }

    #[test]
    fn an_input_resolves_through_the_include_dirs() {
        let dir = camino_tempfile::tempdir().unwrap();
        let base = dir.path().to_path_buf();
        fs_err::create_dir(base.join("src")).unwrap();
        fs_err::write(base.join("src/main.asm"), " nop\n").unwrap();
        assert_eq!(
            resolve_input(&base, "main.asm", &[base.join("src")]),
            Some(base.join("src/main.asm"))
        );
        assert_eq!(resolve_input(&base, "absent.asm", &[base.join("src")]), None);
    }
}
