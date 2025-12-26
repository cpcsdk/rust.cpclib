#[test]
fn test_layered_dependencies_multiple_targets_grouping() {
    use cpclib_bndbuild::rules::graph::{TaskTargetsForLayer, TaskTargets};
    use std::collections::HashSet;
    use cpclib_common::camino::Utf8Path;

    let mut builder_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    builder_path.push("tests/layered_targets.yml");
    let builder_fname = builder_path.to_str().unwrap();

    let (_, builder) = BndBuilder::from_path(builder_fname).unwrap();
    let layered = builder.get_layered_dependencies();

    // There should be 2 layers: [A1,A2,A3,B1,B2,B3] -> [C]
    assert_eq!(layered.nb_layers(), 2);

    // Layer 0: tasks for A and B (producing A1,A2,A3 and B1,B2,B3)
    let layer1 = layered.dependencies_at_layer(1).unwrap();
    let mut all_targets: HashSet<&str> = HashSet::new();
    for task_targets in layer1.tasks.iter() {
        for t in task_targets.targets.iter() {
            all_targets.insert(t.as_str());
        }
    }
    dbg!(&all_targets);
    assert!(all_targets.contains("A1"));
    assert!(all_targets.contains("A2"));
    assert!(all_targets.contains("A3"));
    assert!(all_targets.contains("B1"));
    assert!(all_targets.contains("B2"));
    assert!(all_targets.contains("B3"));

    // Each task group in layer 0 should have 3 targets (A1,A2,A3 or B1,B2,B3)
    for task_targets in layer1.tasks.iter() {
        assert_eq!(task_targets.targets.len(), 3);
    }

    // Layer 1: C
    let layer0 = layered.dependencies_at_layer(0).unwrap();
    let mut c_found = false;
    for task_targets in layer0.tasks.iter() {
        for t in task_targets.targets.iter() {
            if t.as_str() == "C" {
                c_found = true;
            }
        }
    }
    assert!(c_found, "C should be present in the last layer");

    // --- Test tasks_ordered ---
    // Collect all TaskTargets from tasks_ordered
    let tasks_ordered: Vec<_> = layered.tasks_ordered().collect();
    // There should be 3 TaskTargets: one for C, one for A1/A2/A3, one for B1/B2/B3
    assert_eq!(tasks_ordered.len(), 3);

    // Check that one TaskTargets contains only C
    let c_task = tasks_ordered.iter().find(|tt| tt.targets().iter().any(|t| t.as_str() == "C"));
    assert!(c_task.is_some());
    assert_eq!(c_task.unwrap().targets().len(), 1);
    assert!(c_task.unwrap().targets().iter().any(|t| t.as_str() == "C"));

    // Check that the other two TaskTargets contain A1/A2/A3 and B1/B2/B3
    let mut found_a = false;
    let mut found_b = false;
    for tt in tasks_ordered.iter() {
        let targets: Vec<_> = tt.targets().iter().map(|t| t.as_str()).collect();
        if targets.contains(&"A1") && targets.contains(&"A2") && targets.contains(&"A3") {
            found_a = true;
            assert_eq!(targets.len(), 3);
        }
        if targets.contains(&"B1") && targets.contains(&"B2") && targets.contains(&"B3") {
            found_b = true;
            assert_eq!(targets.len(), 3);
        }
    }
    assert!(found_a, "Should find a TaskTargets with A1, A2, A3");
    assert!(found_b, "Should find a TaskTargets with B1, B2, B3");
}
use cpclib_bndbuild::{BndBuilder, rules::graph::LayeredDependenciesByTask};
use cpclib_common::camino::Utf8Path;
use std::{collections::HashSet, path::PathBuf};

#[test]
fn test_layered_dependencies_multiple_targets() {
    use std::path::PathBuf;
    // Setup: create rules with multiple targets per task
    let mut builder_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    builder_path.push("tests/dummy/bndbuild.yml");
    let builder_fname = builder_path.to_str().unwrap();
    let (_, builder) = BndBuilder::from_path(builder_fname).unwrap();

    // Get the dependency layers
    let layered = builder.get_layered_dependencies();
    for i in 0..layered.nb_layers() {
        let taskset = layered.dependencies_at_layer(i).unwrap();
        for task_targets in taskset.tasks.iter() {
            assert!(!task_targets.is_empty(), "Task group in layer {i} is empty");
        }
    }
}

#[test]
fn test_layered_dependencies_for_specific_target() {
    use std::path::PathBuf;
    // Compute the path relative to the current file's directory
    let mut builder_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    builder_path.push("tests/dummy/bndbuild.yml");
    let builder_fname = builder_path.to_str().unwrap();
    let (_, builder) = BndBuilder::from_path(builder_fname).unwrap();
    let target = Utf8Path::new("dummy_logo.o");
    let layered = builder.get_layered_dependencies_for(&target);
    for i in 0..layered.nb_layers() {
        let taskset = layered.dependencies_at_layer(i).unwrap();
        for task_targets in taskset.tasks.iter() {
            assert!(!task_targets.is_empty(), "Task group in layer {i} is empty");
        }
    }
}
