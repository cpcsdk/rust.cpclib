use assert_cmd::Command;

// TODO find why this test does not work anymore
fn test_build_dummy() {
    // TODO boilerplate to refactor to reuse directly the true binary executable
    // let fname = "tests/dummy/bndbuild.yml";
    // let file = std::fs::File::open(fname)
    // .expect(&format!("Unable to access file builder file {fname}."));
    // let mut rdr = BufReader::new(file);
    //
    // let rules: Rules = serde_yaml::from_reader(&mut rdr).expect("Unable to read the provided build file.");
    // let path = std::path::Path::new(fname).parent().unwrap();
    // std::env::set_current_dir(path).expect("Unable to change of working directory");
    //
    // let deps = rules
    // .to_deps()
    // .expect("Unable to build the dependency graph");
    //
    // deps.execute(std::p)

    let builder_fname = "tests/dummy/bndbuild.yml";
   // assert!(std::path::Path::new(builder_fname).exists());

    let mut cmd = Command::cargo_bin("bndbuild").unwrap();
    cmd.arg("-f").arg(&format!("{builder_fname}"));
    cmd.arg("build");

    cmd.assert().success();
}


#[test]
fn test_dummy_phony() {
    use cpclib_bndbuild::BndBuilder;
    use cpclib_common::itertools::Itertools;

    let builder_fname = "tests/dummy/bndbuild.yml";
    let builder = BndBuilder::from_fname(builder_fname).unwrap();

    println!("{:#?}", builder.rules().iter().map(|r| r.targets()).collect_vec());

    assert!(builder.get_rule("m4").unwrap().is_phony());
    assert!(dbg!(builder.get_rule("build").expect("build is missing")).is_phony());
    assert!(builder.get_rule("winape").unwrap().is_phony());

    assert!(!builder.get_rule("distclean").unwrap().is_phony());
    assert!(!builder.get_rule("clean").unwrap().is_phony());
    assert!(!builder.get_rule("dummy_logo.o").unwrap().is_phony());
    assert!(!builder.get_rule("dummy.sna").unwrap().is_phony());

}
