use cpclib_bndbuild::task::{InnerTask, StandardTaskArguments};

#[test]
fn test_args_and_ignore_from_from_command_and_arguments() {
    let std = StandardTaskArguments::new("toto.asm -o toto.o");
    let t = InnerTask::from_command_and_arguments("basm", std.clone()).unwrap();
    assert_eq!(t.args(), "toto.asm -o toto.o");
    assert_eq!(t.ignore_errors(), false);
    // also ensure the enum variant matches (Assembler for basm)
    assert!(matches!(t, InnerTask::Assembler(..)));
}

#[test]
fn test_args_and_ignore_from_deserialize() {
    // Using the leading '-' in the command should set the ignore flag when deserializing
    let t: InnerTask = serde_yaml::from_str("-basm toto.asm -o toto.o").unwrap();
    assert_eq!(t.args(), "toto.asm -o toto.o");
    assert_eq!(t.ignore_errors(), true);
    assert!(matches!(t, InnerTask::Assembler(..)));
}
