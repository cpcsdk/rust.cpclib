use std::str;

use cpclib_common::nom;
use nom::branch::alt;
use nom::bytes::complete::{tag_no_case, take_till};
use nom::character::complete::{space0, space1};
use nom::combinator::{map, rest, value};
use nom::sequence::{preceded, tuple};
use nom::IResult;

#[derive(Debug, Clone)]
pub(crate) enum XferCommand {
    Cd(Option<String>),
    Exit,
    /// Put a file on the M4
    Put(String),
    /// Remove a file on the M4
    Era(String),
    /// Request the current working directory
    Pwd,
    Reset,
    Reboot,
    Help,
    Ls(Option<String>),
    /// Launch a file from the host
    LaunchHost(String),
    /// Launch a file from the M4
    LaunchM4(String),
    /// Launch a command on the host machine
    LocalCommand(String)
}

// TODO find a way to reduce code duplicaiton

fn ls_path(input: &str) -> IResult<&str, XferCommand> {
    map(
        preceded(tuple((tag_no_case("ls"), space1)), rest),
        |path: &str| XferCommand::Ls(Some(path.to_string()))
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
        |path: &str| XferCommand::Cd(Some(path.to_string()))
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
        preceded(tuple((tag_no_case("launch"), space1)), rest),
        |path: &str| XferCommand::LaunchHost(path.to_string())
    )(input)
}

fn local(input: &str) -> IResult<&str, XferCommand> {
    map(
        preceded(tuple((tag_no_case("!"), space0)), rest),
        |path: &str| XferCommand::LocalCommand(path.to_string())
    )(input)
}

/// PUT a file on the M4 with defining a directory
fn put(input: &str) -> IResult<&str, XferCommand> {
    map(
        preceded(
            tuple((tag_no_case("put"), space1)),
            take_till(char::is_whitespace)
        ),
        |path: &str| XferCommand::Put(path.to_string())
    )(input)
}

/// Delete a file from the M4
fn rm(input: &str) -> IResult<&str, XferCommand> {
    map(
        preceded(
            tuple((
                alt((
                    tag_no_case("rm"),
                    tag_no_case("delete"),
                    tag_no_case("del"),
                    tag_no_case("era")
                )),
                space1
            )),
            take_till(char::is_whitespace)
        ),
        |path: &str| XferCommand::Era(path.to_string())
    )(input)
}

fn no_arg(input: &str) -> IResult<&str, XferCommand> {
    alt((
        map(tag_no_case("pwd"), |_| XferCommand::Pwd),
        map(tag_no_case("help"), |_| XferCommand::Help),
        map(tag_no_case("reboot"), |_| XferCommand::Reboot),
        map(tag_no_case("reset"), |_| XferCommand::Reset),
        map(alt((tag_no_case("exit"), tag_no_case("quit"))), {
            |_| XferCommand::Exit
        }),
        map(rest, {
            |fname: &str| XferCommand::LaunchM4(fname.to_string())
        })
    ))(input)
}

/// Launch the parsing of the line
pub(crate) fn parse_command(input: &str) -> IResult<&str, XferCommand> {
    alt((cd, ls, launch, local, put, rm, no_arg))(input)
}
