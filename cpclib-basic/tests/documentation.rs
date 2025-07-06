use cpclib_basic::*;
use paste::paste;

macro_rules! generate_test_for {
	( $($name:ident: $code: expr_2021),+) => { $(
		paste!{
			#[test]
			fn [< documentation_example_ $name:lower>] () {
				dbg!(
					BasicProgram::parse($code)
				).expect("Parse error");
			}
		}
	)+};
}

generate_test_for! {
    ABS: "10 PRINT ABS(-67.98)",
    ATN: "10 PRINT ATN(1)",
    CINT: "10 n=1.9999\n20 PRINT CINT(n)"
}
