use camino::Utf8Path;
use cpclib_bndbuild::BndBuilder;
use std::fs;

#[test]
fn test_expansion_of_at_in_command() {
    // Setup: create a minimal build file with $@ in command
	// Use a path relative to the workspace root so the test works from any cwd
	let build_file = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/expansion/bndbuild.yml");

	for  _ in 0..20 {
    // Run the builder
		let (_, builder) = BndBuilder::from_path(build_file, false).unwrap();

		let rule = builder.get_rule("E20").unwrap();
		assert_eq!(
			rule.targets(),
			[Utf8Path::new("E10"), Utf8Path::new("E20"), Utf8Path::new("E30")]
		);
		let cmd = rule.command(0);
		assert_eq!(cmd.args(), "touch E10 E20 E30");

		let rule = builder.get_rule("E33").unwrap();
		assert_eq!(
			rule.targets(),
			[Utf8Path::new("E40"), Utf8Path::new("E35"), Utf8Path::new("E33")]
		);
		let cmd = rule.command(0);
		assert_eq!(cmd.args(), "touch E40 E35 E33");
	}

}
