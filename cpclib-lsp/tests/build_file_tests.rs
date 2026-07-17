//! Integration tests for build file LSP functionality

use std::collections::HashSet;

#[test]
fn test_build_tasks_include_assemblers() {
    // Test that task completions include assembler tasks
    let task_names: Vec<&str> = cpclib_bndbuild::lsp::TASK_TYPES
        .iter()
        .flat_map(|t| t.names.iter())
        .copied()
        .collect();

    assert!(task_names.contains(&"basm"), "Should include basm task");
    assert!(
        task_names.contains(&"assemble"),
        "Should include assemble alias"
    );
    assert!(task_names.contains(&"rasm"), "Should include rasm task");
    assert!(
        task_names.contains(&"sjasmplus"),
        "Should include sjasmplus task"
    );
}

#[test]
fn test_build_tasks_include_emulators() {
    // Test that task completions include emulator tasks
    let task_names: Vec<&str> = cpclib_bndbuild::lsp::TASK_TYPES
        .iter()
        .flat_map(|t| t.names.iter())
        .copied()
        .collect();

    assert!(task_names.contains(&"ace"), "Should include ace emulator");
    assert!(
        task_names.contains(&"winape"),
        "Should include winape emulator"
    );
    assert!(
        task_names.contains(&"cpcec"),
        "Should include cpcec emulator"
    );
}

#[test]
fn test_build_tasks_include_disk_ops() {
    // Test that task completions include disk operations
    let task_names: Vec<&str> = cpclib_bndbuild::lsp::TASK_TYPES
        .iter()
        .flat_map(|t| t.names.iter())
        .copied()
        .collect();

    assert!(task_names.contains(&"dsk"), "Should include dsk task");
    assert!(task_names.contains(&"sna"), "Should include sna task");
    assert!(
        task_names.contains(&"catalog"),
        "Should include catalog task"
    );
}

#[test]
fn test_build_tasks_include_image_tools() {
    // Test that task completions include image conversion tools
    let task_names: Vec<&str> = cpclib_bndbuild::lsp::TASK_TYPES
        .iter()
        .flat_map(|t| t.names.iter())
        .copied()
        .collect();

    assert!(
        task_names.contains(&"img2cpc"),
        "Should include img2cpc task"
    );
    assert!(
        task_names.contains(&"martine"),
        "Should include martine task"
    );
    assert!(
        task_names.contains(&"convgeneric"),
        "Should include convgeneric task"
    );
}

#[test]
fn test_build_tasks_have_descriptions() {
    // Ensure all tasks have a non-empty description and synopsis.
    // `example` is allowed to be empty: it only holds a real-world `cmd:` invocation
    // when one was actually found in a build file, and not every task type has one.
    for task in cpclib_bndbuild::lsp::TASK_TYPES {
        assert!(
            !task.description.is_empty(),
            "Task {:?} should have a description",
            task.names
        );
        assert!(
            !task.synopsis.is_empty(),
            "Task {:?} should have a synopsis",
            task.names
        );
    }
}

#[test]
fn test_build_tasks_examples_reference_their_command() {
    // `example` holds a real, verbatim `cmd:` invocation copied from an actual build
    // file (never fabricated). When present, it should actually invoke the task's own
    // command name.
    for task in cpclib_bndbuild::lsp::TASK_TYPES {
        let example = task.example;
        if example.is_empty() {
            continue;
        }
        let canonical = task.names[0];
        assert!(
            example.contains(canonical),
            "Task {:?} example should mention its command {:?}: {}",
            task.names,
            canonical,
            example
        );
    }
}

#[test]
fn test_build_keywords_complete() {
    // The rule-level keys must cover every key of the bndbuild schema
    // (schema.json): tgt/target/build, dep/dependency/requires,
    // cmd/command/launch/run, help, phony, constraint.
    let key_names: Vec<&str> = cpclib_bndbuild::lsp::RULE_KEYS
        .iter()
        .flat_map(|k| k.names.iter().copied())
        .collect();

    let expected = [
        "tgt",
        "target",
        "build",
        "dep",
        "dependency",
        "requires",
        "cmd",
        "command",
        "launch",
        "run",
        "help",
        "phony",
        "constraint"
    ];
    for keyword in &expected {
        assert!(
            key_names.contains(keyword),
            "Missing expected schema key: {}",
            keyword
        );
    }
}

#[test]
fn test_build_keywords_have_descriptions() {
    // Ensure all keywords have descriptions
    for (keyword, description) in cpclib_bndbuild::lsp::BUILD_KEYWORDS {
        assert!(
            !description.is_empty(),
            "Keyword '{}' should have a description",
            keyword
        );
    }
}

#[test]
fn test_no_duplicate_task_names() {
    // Ensure no task name appears in multiple TaskTypes
    let mut all_names = HashSet::new();
    let mut duplicates = Vec::new();

    for task in cpclib_bndbuild::lsp::TASK_TYPES {
        for name in task.names {
            if !all_names.insert(name) {
                duplicates.push(name);
            }
        }
    }

    assert!(
        duplicates.is_empty(),
        "Found duplicate task names: {:?}",
        duplicates
    );
}

#[test]
fn test_task_types_synchronized_with_all_applications() {
    // This test ensures TASK_TYPES stays in sync with ALL_APPLICATIONS
    // It duplicates the test in lsp.rs but provides an integration test perspective
    use std::collections::HashSet;

    let mut all_commands = HashSet::new();
    for (cmds, _clearable) in cpclib_bndbuild::ALL_APPLICATIONS {
        for cmd in *cmds {
            all_commands.insert(*cmd);
        }
    }

    let mut task_type_commands = HashSet::new();
    for task in cpclib_bndbuild::lsp::TASK_TYPES {
        for name in task.names {
            task_type_commands.insert(*name);
        }
    }

    let missing: Vec<_> = all_commands.difference(&task_type_commands).collect();

    assert!(
        missing.is_empty(),
        "TASK_TYPES is missing commands from ALL_APPLICATIONS: {:?}",
        missing
    );

    assert_eq!(
        all_commands.len(),
        task_type_commands.len(),
        "TASK_TYPES and ALL_APPLICATIONS should have same number of commands"
    );
}

#[test]
fn test_file_operation_tasks_present() {
    // Ensure file operation tasks are available
    let task_names: Vec<&str> = cpclib_bndbuild::lsp::TASK_TYPES
        .iter()
        .flat_map(|t| t.names.iter())
        .copied()
        .collect();

    assert!(task_names.contains(&"cp"), "Should include cp (copy) task");
    assert!(task_names.contains(&"mv"), "Should include mv (move) task");
    assert!(
        task_names.contains(&"rm"),
        "Should include rm (remove) task"
    );
    assert!(task_names.contains(&"mkdir"), "Should include mkdir task");
}

#[test]
fn test_audio_tasks_present() {
    // Ensure audio/music tasks are available
    let task_names: Vec<&str> = cpclib_bndbuild::lsp::TASK_TYPES
        .iter()
        .flat_map(|t| t.names.iter())
        .copied()
        .collect();

    assert!(
        task_names.contains(&"at3"),
        "Should include Arkos Tracker 3 task"
    );
    assert!(
        task_names.contains(&"ArkosTracker3"),
        "Should include ArkosTracker3 alias"
    );
    assert!(task_names.contains(&"ayt"), "Should include AYT task");
}

#[test]
fn test_external_tool_tasks_present() {
    // Ensure external tool integration tasks are available
    let task_names: Vec<&str> = cpclib_bndbuild::lsp::TASK_TYPES
        .iter()
        .flat_map(|t| t.names.iter())
        .copied()
        .collect();

    assert!(task_names.contains(&"extern"), "Should include extern task");
    assert!(task_names.contains(&"echo"), "Should include echo task");
}
