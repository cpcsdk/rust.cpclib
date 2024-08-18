use assert_cmd::{assert::Assert, Command};

fn launch(folder: &str, tgt: &str) -> Assert {
	Command::cargo_bin("bndbuild").unwrap()
	.args(["-f", folder])
	.arg(tgt)
	.assert()
}


#[ignore]
#[test]
fn check_ucpm() {

	let ucpm_launch = |tgt: &str| {
		launch("tests/ucpm", tgt).success()
	};

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