//! Debugging a bndbuild rule rather than a bare file.
//!
//! The shapes here are taken from a real build file: the emulator tool carries
//! the `-` prefix that ignores its errors, and the snapshot comes from a
//! template variable that is already expanded by the time a rule is read.

use cpclib_dap::launch::debuggable_rules;

fn build_file(body: &str) -> (camino_tempfile::Utf8TempDir, std::path::PathBuf) {
    let tmp = camino_tempfile::tempdir().unwrap();
    let path = tmp.path().join("bndbuild.yml");
    std::fs::write(&path, body).unwrap();
    let std_path = path.as_std_path().to_path_buf();
    (tmp, std_path)
}

/// The rule from the report: `-emu`, several spaces, `run` at the end.
#[test]
fn a_rule_with_an_error_ignoring_emulator_is_offered() {
    let (_tmp, path) =
        build_file("- tgt: test_sna\n  cmd: -emu --emulator ace   --snapshot demo.sna run\n");
    assert_eq!(debuggable_rules(&path), vec!["test_sna".to_string()]);
}

#[test]
fn a_plain_emulator_rule_is_offered() {
    let (_tmp, path) = build_file("- tgt: run\n  cmd: emu --snapshot demo.sna run\n");
    assert_eq!(debuggable_rules(&path), vec!["run".to_string()]);
}

/// Rules that build things are not debuggable - there is no emulator command
/// to rewrite.
#[test]
fn rules_that_do_not_launch_an_emulator_are_not_offered() {
    let (_tmp, path) = build_file(
        "- tgt: demo.sna\n  cmd: basm src/main.asm -o demo.sna\n\
         - tgt: disc\n  cmd: dsk demo.dsk format\n"
    );
    assert!(debuggable_rules(&path).is_empty());
}

/// Several rules, only the ones that qualify.
#[test]
fn only_the_emulator_rules_are_offered() {
    let (_tmp, path) = build_file(
        "- tgt: demo.sna\n  cmd: basm src/main.asm -o demo.sna\n\
         - tgt: test_sna\n  cmd: -emu --snapshot demo.sna run\n\
         - tgt: orgams\n  cmd: emu --snapshot demo.sna orgams\n"
    );
    assert_eq!(debuggable_rules(&path), vec!["test_sna".to_string()]);
}

/// The shape of a real project: a rule whose command launches the emulator,
/// depending on a snapshot built by a *different* rule that declares several
/// targets at once and depends on globs.
///
/// Debugging `test_sna` must build `birthtro.sna` and must **not** run
/// `test_sna` itself - its command is the Winape launch.
#[test]
fn a_real_shaped_build_file_offers_the_emulator_rule_only() {
    let (_tmp, path) = build_file(
        "- tgt: test_sna\n  dep: birthtro.sna\n  cmd: -emu --emulator winape   --snapshot birthtro.sna run\n\
         - tgt: birthtro.sna C0.300 sna.lst\n  dep:\n    - \"*.asm\"\n  cmd: basm main.asm -o birthtro.sna\n"
    );
    assert_eq!(debuggable_rules(&path), vec!["test_sna".to_string()]);
}

/// A missing or unreadable build file yields nothing rather than failing the
/// whole command - the picker just has nothing to offer.
#[test]
fn an_unreadable_build_file_offers_nothing() {
    assert!(debuggable_rules(std::path::Path::new("/nonexistent/bndbuild.yml")).is_empty());
}

/// Templates are expanded before a rule is read.
///
/// The build file says `--snapshot {{SNA}}`; what must come back is
/// `birthtro.sna`. If the raw `{{SNA}}` reached us, the snapshot would be
/// looked for under that literal name and the session would fail - so this
/// pins that `BndBuilder::from_path` really does the minijinja pass and that
/// the rule text we read is the expanded one.
#[test]
fn a_templated_snapshot_name_is_expanded_before_we_read_it() {
    let (_tmp, path) = build_file(
        "{% set SNA=\"birthtro.sna\" %}\n\
         - tgt: test_sna\n  dep: {{ SNA }}\n  cmd: -emu --emulator winape   --snapshot {{SNA}} run\n\
         - tgt: {{SNA}}\n  cmd: basm main.asm -o {{SNA}}\n"
    );
    assert_eq!(debuggable_rules(&path), vec!["test_sna".to_string()]);

    // ...and the snapshot the rule names comes back expanded.
    let snapshot = snapshot_named_by(&path, "test_sna");
    assert_eq!(
        snapshot.as_deref(),
        Some("birthtro.sna"),
        "template expanded"
    );
}

/// The snapshot a rule's emulator command names, read the same way the launch
/// path reads it.
fn snapshot_named_by(build_file: &std::path::Path, target: &str) -> Option<String> {
    let utf8 = cpclib_common::camino::Utf8Path::from_path(build_file)?;
    let (_, builder) = cpclib_bndbuild::BndBuilder::from_path(utf8, true).ok()?;
    let rule = builder
        .rules()
        .iter()
        .find(|r| r.targets().iter().any(|t| t.as_str() == target))?;
    for task in rule.commands() {
        let rendered = task.to_string();
        let (program, arguments) = rendered.split_once(' ')?;
        let program = program.strip_prefix('-').unwrap_or(program);
        if !cpclib_bndbuild::task::EMUCTRL_CMDS.contains(&program) {
            continue;
        }
        let rewritten = cpclib_bndbuild::pipeline::debug::debug_arguments(arguments)?;
        return cpclib_bndbuild::pipeline::debug::snapshot_of(&rewritten);
    }
    None
}
