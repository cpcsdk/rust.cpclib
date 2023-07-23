use assert_cmd::Command;

#[test]
fn test_dummy() {
    /*
    // TODO boilerplate to refactor to reuse directly the true binary executable
    let fname = "tests/dummy/bndbuild.yml";
    let file = std::fs::File::open(fname)
        .expect(&format!("Unable to access file builder file {fname}."));
    let mut rdr = BufReader::new(file);

    let rules: Rules = serde_yaml::from_reader(&mut rdr).expect("Unable to read the provided build file.");
    let path = std::path::Path::new(fname).parent().unwrap();
    std::env::set_current_dir(path).expect("Unable to change of working directory");

    let deps = rules
        .to_deps()
        .expect("Unable to build the dependency graph");

    deps.execute(std::p)
    */

    let builder_fname = "tests/dummy/bndbuild.yml";
    assert!(std::path::Path::new(builder_fname).exists());

    let mut cmd = Command::cargo_bin("bndbuild").unwrap();
    cmd.arg("-f")
        .arg(&format!("{builder_fname}"));
    cmd.arg("build");

    cmd.assert().success();
}
