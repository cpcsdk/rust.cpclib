use std::fs::remove_dir;
use std::process::{Command, Output};

use pretty_assertions::{assert_eq, assert_ne};
use test_generator::test_resources;

const BUILD_BASM: bool = true;

fn build_basm() {
    if BUILD_BASM {
        eprintln!("> Build basm");
        Command::new("cargo")
            .args(["+nightly", "build"])
            .output()
            .expect("Unable to build basm");
    }
}

fn command_for_generated_test(fname: &str, output: &str) -> Output {
    build_basm();

    eprintln!("> Run  basm");
    Command::new("../target/debug/basm")
        .args(["-I", "tests/asm/", "-i", fname, "-o", output])
        .output()
        .expect("Unable to launch basm")
}

fn specific_test(folder: &str, fname: &str) {
    build_basm();

    let output_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let res = Command::new("../target/debug/basm")
        .args(["-I", folder, "-i", fname, "-o", output_fname])
        .output()
        .expect("Unable to launch basm");

    if !res.status.success() {
        panic!(
            "Failure to assemble {}.\n{}",
            fname,
            String::from_utf8_lossy(&res.stderr)
        );
    }
}

#[test]
#[ignore]
fn test_roudoudou_generated_code() {
    std::fs::create_dir("generated_sprites");
    specific_test("tests/asm/roudoudou", "rasm_sprites.asm");
    std::fs::remove_dir("generated_sprites");
}

#[test_resources("basm/tests/asm/good_*.asm")]
fn expect_success(fname: &str) {
    let fname = &fname["basm/tests/asm/".len()..];

    let output_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let res = command_for_generated_test(fname, output_fname);
    if res.status.success() {
        // TODO - add additional checks
        let equiv_fname = fname.replace(".asm", ".equiv");
        if std::path::Path::new("tests/asm/")
            .join(std::path::Path::new(&equiv_fname))
            .exists()
        {
            // control with an equivalent file
            let equiv_output_file =
                tempfile::NamedTempFile::new().expect("Unable to build temporary file");
            let equiv_output_fname = equiv_output_file.path().as_os_str().to_str().unwrap();

            let res_equiv = command_for_generated_test(&equiv_fname, equiv_output_fname);
            if !res_equiv.status.success() {
                panic!(
                    "Error while assembling the equivalent file.\n{}",
                    String::from_utf8_lossy(&res.stderr)
                )
            }

            let output_content = std::fs::read(output_fname).unwrap();
            let equiv_output_content = std::fs::read(equiv_output_fname).unwrap();
            assert_eq!(
                output_content, equiv_output_content,
                "Content differ between {} and {}.",
                fname, equiv_fname
            );
        }
    }
    else {
        eprintln!(
            "Error when assembling {}:\n{}",
            fname,
            String::from_utf8_lossy(&res.stderr)
        );
        panic!()
    }
}

#[test_resources("basm/tests/asm/bad_*.asm")]
fn expect_failure(fname: &str) {
    let fname = &fname["basm/tests/asm/".len()..];

    let output_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let res = command_for_generated_test(fname, output_fname);
    if !res.status.success() {
        let msg = dbg!(String::from_utf8_lossy(&res.stderr));
        if msg.contains("panicked at") {
            eprintln!(
                "Error when assembling {}. Failure due to a basm bug:\n{}",
                fname, msg
            );
            panic!()
        } else {
            panic!()
        }
    }
    else {
        panic!(
            "Error when assembling {}. Wrong success:\n{}",
            fname,
            String::from_utf8_lossy(&res.stdout)
        );
    }
}
