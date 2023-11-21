use std::str;

use cpclib_common::winnow::ascii::{space0, space1};
use cpclib_common::winnow::combinator::{alt, preceded, rest};
use cpclib_common::winnow::token::{tag_no_case, take_till1};
use cpclib_common::winnow::{PResult, Parser};

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

fn ls_path(input: &mut &str) -> PResult<XferCommand> {
    preceded((tag_no_case("ls"), space1), rest)
        .map(|path: &str| XferCommand::Ls(Some(path.to_string())))
        .parse_next(input)
}

fn ls_no_path(input: &mut &str) -> PResult<XferCommand> {
    tag_no_case("ls")
        .value(XferCommand::Ls(None))
        .parse_next(input)
}

fn ls(input: &mut &str) -> PResult<XferCommand> {
    alt((ls_path, ls_no_path)).parse_next(input)
}

fn cd_path(input: &mut &str) -> PResult<XferCommand> {
    preceded((tag_no_case("cd"), space1), rest)
        .map(|path: &str| XferCommand::Cd(Some(path.to_string())))
        .parse_next(input)
}

fn cd_no_path(input: &mut &str) -> PResult<XferCommand> {
    tag_no_case("cd")
        .value(XferCommand::Cd(None))
        .parse_next(input)
}

fn cd(input: &mut &str) -> PResult<XferCommand> {
    alt((cd_path, cd_no_path)).parse_next(input)
}

fn launch(input: &mut &str) -> PResult<XferCommand> {
    preceded((tag_no_case("launch"), space1), rest)
        .map(|path: &str| XferCommand::LaunchHost(path.to_string()))
        .parse_next(input)
}

fn local(input: &mut &str) -> PResult<XferCommand> {
    preceded((tag_no_case("!"), space0), rest)
        .map(|path: &str| XferCommand::LocalCommand(path.to_string()))
        .parse_next(input)
}

/// PUT a file on the M4 with defining a directory
fn put(input: &mut &str) -> PResult<XferCommand> {
    preceded(
        (tag_no_case("put"), space1),
        take_till1(char::is_whitespace)
    )
    .map(|path: &str| XferCommand::Put(path.to_string()))
    .parse_next(input)
}

/// Delete a file from the M4
fn rm(input: &mut &str) -> PResult<XferCommand> {
    preceded(
        (
            alt((
                tag_no_case("rm"),
                tag_no_case("delete"),
                tag_no_case("del"),
                tag_no_case("era")
            )),
            space1
        ),
        take_till1(char::is_whitespace)
    )
    .map(|path: &str| XferCommand::Era(path.to_string()))
    .parse_next(input)
}

fn no_arg(input: &mut &str) -> PResult<XferCommand> {
    alt((
        tag_no_case("pwd").value(XferCommand::Pwd),
        tag_no_case("help").value(XferCommand::Help),
        tag_no_case("reboot").value(XferCommand::Reboot),
        tag_no_case("reset").value(XferCommand::Reset),
        alt((tag_no_case("exit"), tag_no_case("quit"))).value(XferCommand::Exit),
        rest.map({ |fname: &str| XferCommand::LaunchM4(fname.to_string()) })
    ))
    .parse_next(input)
}

/// Launch the parsing of the line
pub(crate) fn parse_command(input: &mut &str) -> PResult<XferCommand> {
    alt((cd, ls, launch, local, put, rm, no_arg)).parse_next(input)
}
