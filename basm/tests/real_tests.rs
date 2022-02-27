use std::process::{Command, Output};

use test_generator::test_resources;
use pretty_assertions::{assert_eq, assert_ne};

fn  command_for (fname: &str, output: &str) -> Output {

	Command::new("cargo")
		.args(["+nightly", "build"])
		.output()
		.expect("Unable to build basm");

	Command::new("../target/debug/basm")
		.args([
			"-I", "tests/asm/",
			"-i", fname, "-o", output])
		.output()
		.expect("Unable to launch basm")

}


#[test_resources("basm/tests/asm/good_*.asm")]
fn expect_success(fname: &str) {
    let fname = &fname["basm/tests/asm/".len()..];

    let output_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let res = command_for(fname, output_fname);
    if res.status.success() {
        // TODO - add additional checks
        let equiv_fname = fname.replace(".asm", ".equiv");
        if std::path::Path::new("tests/asm/").join(std::path::Path::new(&equiv_fname)).exists() {
            // control with an equivalent file
            let equiv_output_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
            let equiv_output_fname = equiv_output_file.path().as_os_str().to_str().unwrap();

            let res_equiv = command_for(&equiv_fname, equiv_output_fname);
            if !res_equiv.status.success() {
                panic!("Error while assembling the equivalent file.\n{}", String::from_utf8_lossy(&res.stderr))
            }

            let output_content = std::fs::read(output_fname).unwrap();
            let equiv_output_content = std::fs::read(equiv_output_fname).unwrap();
            assert_eq!(output_content, equiv_output_content, "Content differ between {} and {}.", fname, equiv_fname);

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

    let res = command_for(fname, output_fname);
    if !res.status.success() {
        let msg = dbg!(String::from_utf8_lossy(&res.stderr));
        if msg.contains("RUST_BACKTRACE") {
            eprintln!("Error when assembling {}. Failure due to a basm bug:\n{}", 
			fname,
			msg);
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

