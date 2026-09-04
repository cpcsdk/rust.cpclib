//! The `-D` symbol definitions a project's build rule passes to `basm`.
//!
//! Assembling a project's entry file is still not the same as *building* it.
//! In `birthtro`, `src/build.bnd` runs
//!
//! ```text
//! basm --snapshot sna.asm -o birthtro.sna
//!     -DFACE_SCR=\"{{FACE_SCR}}\" -DMUSIC_CFG=\"{{MUSIC_CFG}}\"
//!     -DSPRITE0_WIDTH={{SPRITE0_WIDTH}} ...
//! ```
//!
//! and `sna.asm` reaches `include MUSIC_CFG`, which resolves to nothing at all
//! without those. So the entry assembles into a different program again - or,
//! as here, fails outright - and every address read from it is fiction.
//!
//! Reading them from the build rule (rather than asking the user to restate
//! them in editor config) is deliberate: a definition restated in two places
//! is a definition that will eventually disagree with itself, and the values
//! here are exactly the ones a user changes when swapping an image or a tune.
//!
//! The values come from `{% set %}` variables, so the build file is expanded
//! through minijinja first - reusing [`crate::jinja`]'s environment,
//! the same expansion the bndbuild language features already run on.

use std::path::{Path, PathBuf};

/// What the build rule says to define, and where that was found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildDefinitions {
    /// `(symbol, value)` exactly as written after `-D`, value unquoted.
    pub values: Vec<(String, String)>,
    /// The build file they came from - for telling the user where to look.
    pub source: Option<PathBuf>,
    /// The `--sourcemap` file that same command writes, resolved against the
    /// build file's own directory.
    pub source_map: Option<PathBuf>
}

impl BuildDefinitions {
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Build files worth inspecting for `entry`, nearest first: its own directory,
/// then upwards to the project root.
fn candidate_build_files(entry: &Path) -> Vec<PathBuf> {
    const NAMES: &[&str] = &["build.bnd", "bnd.build", "bndbuild.yml"];
    let mut out = Vec::new();
    let mut dir = entry.parent();
    while let Some(current) = dir {
        for name in NAMES {
            let candidate = current.join(name);
            if candidate.is_file() {
                out.push(candidate);
            }
        }
        if crate::root::is_project_root(current) {
            break;
        }
        dir = current.parent();
    }
    out
}

/// The definitions the build passes when assembling `entry`.
///
/// Empty when no build file mentions it - which is not an error: plenty of
/// programs need no definitions at all.
pub fn definitions_for_entry(entry: &Path) -> BuildDefinitions {
    let Some(entry_name) = entry.file_name().and_then(|n| n.to_str())
    else {
        return BuildDefinitions::default();
    };

    for build_file in candidate_build_files(entry) {
        let Ok(text) = fs_err::read_to_string(&build_file)
        else {
            continue;
        };
        // `{% set FACE="face3" %}` and friends only mean something once
        // expanded; before that every value is a `{{ ... }}` placeholder.
        let Ok(expanded) = crate::jinja::expand(&text, build_file.parent())
        else {
            continue;
        };

        let values = definitions_in_rule_building(&build_file, entry)
            .unwrap_or_else(|| definitions_in_command_for(&expanded, entry_name));
        let source_map = source_map_in_command_for(&expanded, entry_name).map(|name| {
            build_file
                .parent()
                .map(|dir| dir.join(&name))
                .unwrap_or_else(|| PathBuf::from(&name))
        });
        if !values.is_empty() || source_map.is_some() {
            return BuildDefinitions {
                values,
                source: Some(build_file),
                source_map
            };
        }
    }
    BuildDefinitions::default()
}

/// The definitions from the first rule that has `entry` among its
/// dependencies.
///
/// This is the question that actually identifies the right rule. Matching on
/// "a command mentioning the file" is a guess that breaks as soon as the
/// command does not name it literally, whereas a rule's `dep:` list is the
/// build system's own statement of what it is built from. Dependencies are
/// globs as often as not - `birthtro` lists `"*.asm"`, never `sna.asm` - so
/// they are expanded with bndbuild's own `expand_glob_in` rather than compared
/// as text.
///
/// `None` when the build file cannot be loaded as a real bndbuild description,
/// leaving the caller its text-scanning fallback.
fn definitions_in_rule_building(build_file: &Path, entry: &Path) -> Option<Vec<(String, String)>> {
    let build_file_utf8 = camino::Utf8Path::from_path(build_file)?;
    let (_, builder) = cpclib_bndbuild::BndBuilder::from_path(build_file_utf8, true).ok()?;
    let base = build_file_utf8.parent()?;
    let entry = fs_err::canonicalize(entry).ok()?;

    for rule in builder.rules() {
        let builds_entry = rule.dependencies().iter().any(|dep| {
            cpclib_bndbuild::expand_glob_in(dep.as_str(), base)
                .iter()
                .any(|expanded| {
                    fs_err::canonicalize(base.as_std_path().join(expanded))
                        .is_ok_and(|p| p == entry)
                })
        });
        if !builds_entry {
            continue;
        }
        // Every command of the rule, not just the assembler's: only basm reads
        // `-D`, so taking them all costs nothing and avoids having to identify
        // which task is the assembler.
        let values: Vec<(String, String)> = rule
            .commands()
            .iter()
            .flat_map(|task| parse_definitions(task.args()))
            .collect();
        if !values.is_empty() {
            return Some(values);
        }
    }
    None
}

/// Fallback: extract `-D` definitions from whichever `basm` command in `text`
/// names `entry_name`.
///
/// Only used when the file will not load as a real bndbuild description -
/// [`definitions_in_rule_building`] is the reliable route, since it asks the
/// build system what each rule is built *from* rather than guessing from the
/// command text.
///
/// A command can span continuation lines, so the scan runs from the `basm`
/// invocation up to the next rule (`- tgt:`) or blank-line-separated block
/// rather than to end of line.
fn definitions_in_command_for(text: &str, entry_name: &str) -> Vec<(String, String)> {
    command_for(text, entry_name)
        .map(|command| parse_definitions(&command))
        .unwrap_or_default()
}

/// The `--sourcemap` file the build writes for `entry`, if it writes one.
///
/// Recovered from the same command line as the `-D` values, deliberately: the
/// map and the definitions describe one assemble, and reading them from one
/// place is what keeps them describing the same one. Guessing the name from
/// the entry's instead - `sna.asm` -> `sna.map` - works only for as long as
/// nobody renames it, and fails by silently assembling again.
fn source_map_in_command_for(text: &str, entry_name: &str) -> Option<String> {
    let command = command_for(text, entry_name)?;
    let mut words = command.split_whitespace();
    while let Some(word) = words.next() {
        if let Some(value) = word.strip_prefix("--sourcemap=") {
            return Some(unquote(value));
        }
        if word == "--sourcemap" {
            return words.next().map(unquote);
        }
    }
    None
}

fn unquote(value: &str) -> String {
    value.trim_matches(['"', '\'', '\\']).to_string()
}

/// The whole `basm` command a build file runs for `entry`, joined into one
/// line - it is routinely written across several with `|` and indentation.
fn command_for(text: &str, entry_name: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|line| {
        let lowered = line.to_lowercase();
        lowered.contains("basm") && line.contains(entry_name)
    })?;

    let mut command = String::new();
    for line in &lines[start..] {
        let trimmed = line.trim_start();
        // A new rule ends the command.
        if command.is_empty() || !trimmed.starts_with("- tgt:") {
            command.push(' ');
            command.push_str(line);
        }
        if !command.is_empty() && trimmed.starts_with("- tgt:") && command.trim() != line.trim() {
            break;
        }
        // A rule's other keys end the command too.
        if trimmed.starts_with("dep:") && !command.trim().is_empty() && command.contains("basm") {
            break;
        }
    }

    Some(command)
}

/// Pull `-DNAME=VALUE` / `--define NAME=VALUE` out of a command line.
///
/// Values arrive shell-escaped from the build file (`-DFACE_SCR=\"a/b.scr\"`),
/// so both the backslashes and the quotes come off - what basm ultimately
/// receives is the bare string.
fn parse_definitions(command: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let cleaned = command.replace('\\', "");
    let mut words = cleaned.split_whitespace().peekable();

    while let Some(word) = words.next() {
        let body = if let Some(rest) = word.strip_prefix("-D") {
            rest.to_string()
        }
        else if word == "--define" {
            match words.next() {
                Some(next) => next.to_string(),
                None => continue
            }
        }
        else {
            continue;
        };
        if body.is_empty() {
            continue;
        }

        // basm's own rule: no `=` means the symbol is defined as 1.
        let (name, value) = match body.split_once('=') {
            Some((name, value)) => (name, value),
            None => (body.as_str(), "1")
        };
        let value = value.trim_matches('"');
        if !name.is_empty() {
            out.push((name.to_string(), value.to_string()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `birthtro`'s real build rule, shape for shape: `{% set %}` variables,
    /// a multi-line command, shell-escaped string values and bare numbers.
    const BUILD: &str = r#"
{% set SNA="birthtro.sna" %}
{% set FACE="face3" %}
{%set SPRITE0_WIDTH=48//2 %}
{% set FACE_SCR="../data/" ~ FACE ~ "_cpc.scr" %}
{% set MUSIC_CFG="../data/music/ops_playerconfig.asm" %}

- tgt: test_sna
  dep: {{ SNA }}
  cmd: -emu --snapshot {{SNA}} run

- tgt: {{SNA}}
  dep:
    - "*.asm"
  cmd: |
    basm --snapshot sna.asm -o {{SNA}}
        -DFACE_SCR=\"{{FACE_SCR}}\"
        -DSPRITE0_WIDTH={{SPRITE0_WIDTH}}
        -DMUSIC_CFG=\"{{MUSIC_CFG}}\"
"#;

    fn defs_of(entry: &str) -> Vec<(String, String)> {
        let expanded = crate::jinja::expand(BUILD, None).expect("expands");
        definitions_in_command_for(&expanded, entry)
    }

    /// The values must arrive *expanded*: `{{FACE_SCR}}` is meaningless to the
    /// assembler, `../data/face3_cpc.scr` is the point.
    #[test]
    fn the_definitions_come_back_expanded_and_unquoted() {
        let defs = defs_of("sna.asm");
        assert!(
            defs.contains(&("FACE_SCR".to_string(), "../data/face3_cpc.scr".to_string())),
            "{defs:?}"
        );
        assert!(
            defs.contains(&(
                "MUSIC_CFG".to_string(),
                "../data/music/ops_playerconfig.asm".to_string()
            )),
            "the include that made the whole assemble fail: {defs:?}"
        );
    }

    /// A computed numeric value (`48//2`) has to survive as the number, not as
    /// the expression.
    #[test]
    fn a_computed_numeric_definition_is_expanded_too() {
        let defs = defs_of("sna.asm");
        assert!(
            defs.contains(&("SPRITE0_WIDTH".to_string(), "24".to_string())),
            "{defs:?}"
        );
    }

    /// Only the command that assembles *this* entry counts - a project has
    /// several rules, and the emulator rule above defines nothing.
    #[test]
    fn a_rule_that_does_not_assemble_this_entry_contributes_nothing() {
        assert!(defs_of("something-else.asm").is_empty());
    }

    #[test]
    fn a_bare_definition_defaults_to_one_like_basm_does() {
        assert_eq!(
            parse_definitions("basm x.asm -DDEBUG"),
            vec![("DEBUG".to_string(), "1".to_string())]
        );
    }

    #[test]
    fn the_long_form_is_understood_too() {
        assert_eq!(
            parse_definitions("basm x.asm --define WIDTH=24"),
            vec![("WIDTH".to_string(), "24".to_string())]
        );
    }
    /// The route that actually identifies the rule: the first one listing the
    /// entry among its dependencies.
    ///
    /// The fixture is shaped to defeat the easy wrong answers. An earlier rule
    /// carries `-D` definitions of its own but does *not* depend on the entry,
    /// so anything picking "the first rule with definitions" fails. And the
    /// real rule lists the glob `"*.asm"` rather than `sna.asm` - which is what
    /// `birthtro` does, and what makes text comparison useless.
    #[test]
    fn the_rule_that_depends_on_the_entry_is_the_one_that_counts() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let dir = tmp.path().as_std_path();
        std::fs::write(dir.join("sna.asm"), "    run start\nstart\n    ret\n").unwrap();
        std::fs::write(dir.join("other.asm"), "    ret\n").unwrap();
        std::fs::write(
            dir.join("build.bnd"),
            concat!(
                "- tgt: decoy\n",
                "  dep: other.asm\n",
                "  cmd: basm other.asm -DWRONG=1\n",
                "\n",
                "- tgt: game.sna\n",
                "  dep:\n",
                "    - \"*.asm\"\n",
                "  cmd: basm --snapshot sna.asm -o game.sna -DRIGHT=7\n"
            )
        )
        .unwrap();

        let defs = definitions_in_rule_building(&dir.join("build.bnd"), &dir.join("sna.asm"))
            .expect("the glob dependency must match the entry");

        assert!(
            defs.contains(&("RIGHT".to_string(), "7".to_string())),
            "{defs:?}"
        );
        assert!(
            !defs.iter().any(|(name, _)| name == "WRONG"),
            "the decoy rule does not build the entry: {defs:?}"
        );
    }
}
