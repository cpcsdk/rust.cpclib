use nom::branch::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::multi::*;
use nom::sequence::*;
use nom::*;

use std::str;

#[derive(Debug, Clone)]
pub(crate) enum XferCommand {
    Cd(Option<String>),
    Pwd,
    Reset,
    Reboot,
    Help,
    Ls(Option<String>),
    /// Launch a file from the host
    LaunchHost(String),
    /// Launch a file from the M4
    LaunchM4(String),
}

// TODO find a way to reduce code duplicaiton

fn ls_path(input: &str) -> IResult<&str, XferCommand> {
    map(
        preceded(tuple((tag_no_case("ls"), space1)), alpha1),
        |path: &str| XferCommand::Ls(Some(path.to_string())),
    )(input)
}

fn ls_no_path(input: &str) -> IResult<&str, XferCommand> {
    value(XferCommand::Ls(None), tag_no_case("ls"))(input)
}

fn ls(input: &str) -> IResult<&str, XferCommand> {
    alt((ls_path, ls_no_path))(input)
}

fn cd_path(input: &str) -> IResult<&str, XferCommand> {
    map(
        preceded(tuple((tag_no_case("cd"), space1)), rest),
        |path: &str| XferCommand::Cd(Some(path.to_string())),
    )(input)
}

fn cd_no_path(input: &str) -> IResult<&str, XferCommand> {
    value(XferCommand::Cd(None), tag_no_case("cd"))(input)
}

fn cd(input: &str) -> IResult<&str, XferCommand> {
    alt((cd_path, cd_no_path))(input)
}


fn launch(input: &str) -> IResult<&str, XferCommand> {
    map(
        preceded(
            tuple((tag_no_case("launch"), space1)), 
            rest
        ),
        |path: &str| XferCommand::LaunchHost(path.to_string())
    )(input)
}

fn no_arg(input: &str) -> IResult<&str, XferCommand> {
    alt((
        map(tag_no_case("pwd"), { |_| XferCommand::Pwd }),
        map(tag_no_case("help"), { |_| XferCommand::Help }),
        map(tag_no_case("reboot"), { |_| XferCommand::Reboot }),
        map(tag_no_case("reset"), { |_| XferCommand::Reset }),
        map(rest, {|fname: &str| XferCommand::LaunchM4(fname.to_string())})
    ))(input)
}

/// Launch the parsing of the line
pub(crate) fn parse_command(input: &str) -> IResult<&str, XferCommand> {
    alt((cd, ls, launch, no_arg))(input)
}
