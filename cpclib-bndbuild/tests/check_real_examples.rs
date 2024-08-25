use assert_cmd::assert::Assert;
use assert_cmd::Command;
use serial_test::serial;

fn launch(folder: &str, tgt: &str) -> Assert {
    Command::cargo_bin("bndbuild")
        .unwrap()
        .args(["-f", folder])
        .arg(tgt)
        .assert()
}

#[serial]
#[ignore]
#[test]
fn check_ucpm() {
    let ucpm_launch = |tgt: &str| launch("tests/ucpm", tgt).success();

    let distclean = || {
        ucpm_launch("distclean");
        assert!(!std::path::Path::new("./tests/ucpm/u c p m.dsk").exists());
    };

    let build_sna = || {
        ucpm_launch("sna");
        assert!(std::path::Path::new("./tests/ucpm/u c p m.dsk").exists());
    };

    distclean();
    build_sna();
    distclean();
}

#[serial]
#[ignore]
#[test]
fn check_orgams() {
    let orgams_launch = |tgt: &str| launch("tests/orgams", tgt).success();

    let distclean = || {
        orgams_launch("distclean");
        assert!(!std::path::Path::new("./tests/orgams/cpcfolder/BORDER").exists());
    };

    let build_sna = || {
        orgams_launch("cpcfolder/BORDER");
        assert!(std::path::Path::new("./tests/orgams/cpcfolder/BORDER").exists());
    };

    distclean();
    build_sna();
    distclean();
}

#[serial]
#[test]
fn check_jinja() {
    let jinja_launch = |tgt: &str, asm: &str| {
        Command::cargo_bin("bndbuild")
            .unwrap()
            .args(["-f", "tests/jinja"])
            .arg(format!("-DASSEMBLER={asm}"))
            .arg(tgt)
            .assert()
    };

    let distclean = |asm| {
        jinja_launch("distclean", asm);
        assert!(!std::path::Path::new("./tests/jinja/TEST").exists());
    };

    let build_test = |asm| {
        jinja_launch("TEST", asm);
        assert!(std::path::Path::new("./tests/jinja/TEST").exists());
    };

    let exec = |asm| {
        distclean(asm);
        build_test(asm);
        distclean(asm);
    };

    exec("basm");
    exec("rasm");
}

#[serial]
#[test]
fn check_delegated() {
    let delegated_launch = |tgt: &str| launch("tests/delegated", tgt).success();

    let distclean = || {
        delegated_launch("distclean");
        assert!(!std::path::Path::new("./tests/delegated/show.sna").exists());
        assert!(!std::path::Path::new("./tests/delegated/SHOW.SCR").exists());
    };

    let build_sna = || {
        delegated_launch("show.sna");
        assert!(std::path::Path::new("./tests/delegated/show.sna").exists());
        assert!(!std::path::Path::new("./tests/delegated/SHOW.SCR").exists());
    };

    distclean();
    build_sna();
    distclean();
}

#[serial]
#[test]
fn check_dummy() {
    let dummy_launch = |tgt: &str| launch("tests/dummy", tgt).success();

    let distclean = || {
        dummy_launch("distclean");
        assert!(!std::path::Path::new("./tests/dummy/dummy.sna").exists());
    };

    let build_sna = || {
        dummy_launch("dummy.sna");
        assert!(std::path::Path::new("./tests/dummy/dummy.sna").exists());
    };

    distclean();
    build_sna();
    distclean();
}

#[serial]
#[test]
fn check_hello_world() {
    let dummy_launch = |tgt: &str| launch("tests/hello_world", tgt).success();

    let distclean = || {
        dummy_launch("distclean");
        assert!(!std::path::Path::new("./tests/hello_world/hello1.dsk").exists());
        assert!(!std::path::Path::new("./tests/hello_world/hello2.dsk").exists());
    };

    let build = || {
        dummy_launch("dsk");
        assert!(std::path::Path::new("./tests/hello_world/hello1.dsk").exists());
        assert!(std::path::Path::new("./tests/hello_world/hello2.dsk").exists());
    };

    distclean();
    build();
    distclean();
}
