/// Contains some tests stolen to rasm source code
/// 

use std::process::{Command};


macro_rules! import_rasm_success {
($ (#define $name:ident $($code:expr)+);+ ;)  => {$(
			#[test]
			fn $name() {
				test_assemble(concat!($($code),+))
			}
	)+
}

}




import_rasm_success!{
	#define AUTOTEST_NOINCLUDE "truc equ 0:if truc:include'bite':endif:nop";
	#define AUTOTEST_ENHANCED_LD	"ld h,(ix+11): ld l,(ix+10): ld h,(iy+21): ld l,(iy+20): ld b,(ix+11): ld c,(ix+10):" 
	"ld b,(iy+21): ld c,(iy+20): ld d,(ix+11): ld e,(ix+10): ld d,(iy+21): ld e,(iy+20): ld hl,(ix+10): " 
	"ld hl,(iy+20):ld bc,(ix+10):ld bc,(iy+20): ld de,(ix+10):ld de,(iy+20)";
}


fn test_assemble(code: &str) {

	let input_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
	let input_fname = input_file.path().as_os_str().to_str().unwrap();
	std::fs::write(input_fname, code).unwrap();


	let output_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

	let res = Command::new("../target/debug/basm")
	.args(["-I", "tests/asm/", 
	"-i", 
	input_fname, 
	"-o", output_fname,
	])
	.output()
	.expect("Unable to launch basm");

	if !res.status.success() {
		panic!(
			"Failure to assemble.\n{}",
			String::from_utf8_lossy(&res.stderr)
		);
	}

}