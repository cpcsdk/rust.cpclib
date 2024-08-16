use assert_cmd::Command;
use ntest::timeout;
use serial_test::serial;

#[test]
#[serial]
#[timeout(60000)]
fn assemble_z80tests_correct() {
    let mut cmd = Command::cargo_bin("emucontrol").unwrap();
    cmd.args(&[
        "--drivea",
        "./tests/orgams-ff.dsk",
        "orgams",
        "--src",
        "z80tests.o"
    ])
    .assert()
    .success();
}

#[test]
#[serial]
#[timeout(60000)]
fn assemble_z80tests_albireo_correct() {
    let mut cmd = Command::cargo_bin("emucontrol").unwrap();
    cmd.args(&["--albireo", "./tests/ORG/", "orgams", "--src", "z80tests.o"])
        .assert()
        .success();
}

#[test]
#[serial]
#[timeout(60000)]
fn assemble_z80tests_tgt_incorrect() {
    let mut cmd = Command::cargo_bin("emucontrol").unwrap();
    cmd.args(&[
        "--drivea",
        "./tests/orgams-ff.dsk",
        "orgams",
        "--src",
        "z80tests.o",
        "--dst",
        "tooooloooong.ext"
    ])
    .assert()
    .failure();
}

#[test]
#[serial]
#[timeout(60000)]
fn assemble_z80tests_wrong_input_file() {
    let mut cmd = Command::cargo_bin("emucontrol").unwrap();
    cmd.args(&[
        "--drivea",
        "./tests/orgams-ff.dsk",
        "orgams",
        "--src",
        "wrong.o"
    ])
    .assert()
    .failure();
}
