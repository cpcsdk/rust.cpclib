use cpclib_bndbuild::task::{InnerTask, StandardTaskArguments};

#[test]
fn test_from_command_and_arguments_various() {
    // basm -> Assembler
    let std = StandardTaskArguments::new("toto.asm -o toto.o");
    let t = InnerTask::from_command_and_arguments("basm", std.clone()).unwrap();
    assert!(matches!(t, InnerTask::Assembler(..)));
    assert_eq!(t.args(), "toto.asm -o toto.o");

    // rm -> Rm
    let std = StandardTaskArguments::new("file1.txt");
    let t = InnerTask::from_command_and_arguments("rm", std.clone()).unwrap();
    assert!(matches!(t, InnerTask::Rm(..)));
    assert_eq!(t.args(), "file1.txt");

    // img2cpc -> ImgToCpc
    let std = StandardTaskArguments::new("in.png out.cpc");
    let t = InnerTask::from_command_and_arguments("img2cpc", std.clone()).unwrap();
    assert!(matches!(t, InnerTask::ImgToCpc(..)));
    assert_eq!(t.args(), "in.png out.cpc");

    // xfer -> Xfer
    let std = StandardTaskArguments::new("send.m4");
    let t = InnerTask::from_command_and_arguments("xfer", std.clone()).unwrap();
    assert!(matches!(t, InnerTask::Xfer(..)));
    assert_eq!(t.args(), "send.m4");
}

#[test]
fn test_quoted_join_behavior_preserved() {
    // Simulate Python-side quoting: each arg wrapped in double quotes and inner quotes escaped
    let parts = vec!["file name.txt", "arg\"withquote", "-o", "out.bin"];
    let quoted: Vec<String> = parts
        .into_iter()
        .map(|s| format!("\"{}\"", s.replace('"', "\\\"")))
        .collect();
    let joined = quoted.join(" ");

    let std = StandardTaskArguments::new(joined.clone());
    let t = InnerTask::from_command_and_arguments("basm", std.clone()).unwrap();
    // The args() should return the exact joined string (quotes preserved)
    assert_eq!(t.args(), joined);
}
