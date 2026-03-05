use cpclib_bndbuild::task::{InnerTask, TaskKind};

/// Test that all embedded tasks are properly classified
#[test]
fn test_task_classification() {
    let mut embedded_count = 0;
    let mut delegated_count = 0;
    let mut emulated_count = 0;

    for task in InnerTask::all() {
        match task.kind() {
            TaskKind::Embedded => {
                embedded_count += 1;
                println!("Embedded: {}", task);
            },
            TaskKind::Delegated => {
                delegated_count += 1;
                println!("Delegated: {}", task);
            },
            TaskKind::Emulated => {
                emulated_count += 1;
                println!("Emulated: {}", task);
            }
        }
    }

    println!("\nSummary:");
    println!("  Embedded: {}", embedded_count);
    println!("  Delegated: {}", delegated_count);
    println!("  Emulated: {}", emulated_count);

    // Ensure we have some tasks in each category
    assert!(
        embedded_count > 0,
        "Should have at least some embedded tasks"
    );
    assert!(
        delegated_count > 0,
        "Should have at least some delegated tasks"
    );
}

mod integration {
    use assert_cmd::Command;

    use super::*;

    /// Test that all embedded commands properly handle --help option
    #[test]
    fn test_embedded_commands_help() {
        for task in InnerTask::all() {
            if task.kind() == TaskKind::Embedded {
                let command_name = get_command_name(&task);

                println!("Testing --help for: {}", command_name);

                let mut cmd = Command::cargo_bin("bndbuild").unwrap();
                cmd.arg("--direct")
                    .arg(&command_name)
                    .arg("--")
                    .arg("--help");

                let assert = cmd.assert();
                assert.success().code(0);
            }
        }
    }

    /// Test that all embedded commands properly handle --version option
    #[test]
    fn test_embedded_commands_version() {
        for task in InnerTask::all() {
            if task.kind() == TaskKind::Embedded {
                let command_name = get_command_name(&task);

                println!("Testing --version for: {}", command_name);

                let mut cmd = Command::cargo_bin("bndbuild").unwrap();
                cmd.arg("--direct")
                    .arg(&command_name)
                    .arg("--")
                    .arg("--version");

                let assert = cmd.assert();
                assert.success().code(0);
            }
        }
    }

    /// Helper function to extract the command name from an InnerTask
    fn get_command_name(task: &InnerTask) -> String {
        // Convert the task to string representation and extract the command
        let task_str = format!("{}", task);
        task_str
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_start_matches('-')
            .to_string()
    }
}
