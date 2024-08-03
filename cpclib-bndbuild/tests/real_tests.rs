// tests cannot be run in parallel as they change cwd
use std::sync::{LazyLock, Mutex};

use cpclib_bndbuild::BndBuilder;
use test_generator;

static MUT: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[test_generator::test_resources("cpclib-bndbuild/tests/valid/parse*.yml")]
fn expect_successful_parse(real_fname: &str) {
    let _guard = MUT.lock();

    println!("{}", real_fname);
    println!("{}", std::env::current_dir().unwrap().display());
    // std::env::set_current_dir("..");

    let real_fname = std::path::Path::new("..")
        .join(real_fname)
        .display()
        .to_string();
    println!("{}", real_fname);

    let backup = std::env::current_dir().unwrap();
    let builder = BndBuilder::from_fname(real_fname);
    std::env::set_current_dir(backup);
    if let Err(e) = builder {
        eprintln!("{}", e);
        panic!();
    }
    assert!(true) // must succedd
}

#[test_generator::test_resources("cpclib-bndbuild/tests/invalid/parse*.yml")]
fn expect_parse_error(real_fname: &str) {
    let _guard = MUT.lock();

    println!("{}", real_fname);
    println!("{}", std::env::current_dir().unwrap().display());
    // 	std::env::set_current_dir("..");

    let builder = BndBuilder::from_fname(real_fname);
    if let Err(e) = &builder {
        eprintln!("{}", e);
    }
    assert!(builder.is_err()); // must fail
}
