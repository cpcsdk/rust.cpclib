use std::fs::remove_dir;
use std::process::{Command, Output};

use cpclib_asm::assembler::Env;
use cpclib_asm::basm_utils::{build_args_parser, process, BasmError};
use cpclib_asm::error::AssemblerError;
use pretty_assertions::{assert_eq, assert_ne};
use test_generator::test_resources;

const BUILD_BASM: bool = true;



fn command_for_generated_test(fname: &str, output: &str) -> Result<(Env, Vec<AssemblerError>), BasmError> {
    let args_parser = build_args_parser();
    let args = args_parser.get_matches_from(&[
        "basm",
        "-I", "tests/asm/", 
        "-i", 
        "-o", output,
        fname, 
    ]);
    process(&args)
}

fn specific_test(folder: &str, fname: &str) {
  

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
/// TODO write tests specifics for this purpose
fn expect_listing_success(fname: &str) {
    let fname = &fname["basm/tests/asm/".len()..];

    let output_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();


    let listing_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let listing_fname = listing_file.path().as_os_str().to_str().unwrap();


    println!("Lisitng will be generated in {}", &listing_fname);
    let res = Command::new("../target/debug/basm")
        .args(["-I", "tests/asm/", 
        "-i", 
        fname, 
        "-o", output_fname,
        "--lst", listing_fname
        ])
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


#[test_resources("basm/tests/asm/good_*.asm")]
fn expect_success(fname: &str) {
    let fname = &fname["basm/tests/asm/".len()..];

    let output_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();



    let res = command_for_generated_test(fname, output_fname);
    if res.is_ok() {
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
            if !res_equiv.is_ok() {
                eprintln!(
                    "Error while assembling the equivalent file.\n{}",
                    res.err().unwrap().to_string()
                );
                panic!()
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
            res.err().unwrap().to_string()
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
    if res.is_err() {
       let msg = res.err().unwrap().to_string();

        if msg.contains("[Invalid file name]") {
                panic!("There is a memory issue there...{}",msg)
            }
    }
    else {
        eprintln!(
            "Error when assembling {}. Wrong success:\n",
            fname
            
        );
        panic!();
    }
}
