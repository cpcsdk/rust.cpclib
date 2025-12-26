#[test]
fn test_layered_dependencies_multiple_targets_grouping() {
    use cpclib_bndbuild::rules::graph::{TaskTargetsForLayer, TaskTargets};
    use std::collections::HashSet;
    use cpclib_common::camino::Utf8Path;

    let builder_fname = "tests/layered_targets.yml";
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
}
use cpclib_bndbuild::{BndBuilder, rules::graph::LayeredDependenciesByTask};
use cpclib_common::camino::Utf8Path;
use std::collections::HashSet;

#[test]
fn test_layered_dependencies_multiple_targets() {
    // Setup: create rules with multiple targets per task
    // This is a minimal example; in real tests, use actual Rule/Rules construction
    // For now, we assume a builder can be loaded from a test YAML file
    let builder_fname = "tests/dummy/bndbuild.yml";
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
    let builder_fname = "tests/dummy/bndbuild.yml";
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
