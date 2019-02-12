use nom;
use nom::types::CompleteStr;
use nom::{alpha1, eof, preceded, space0, space1};
use nom::{alphanumeric, multispace, space};
use std::str;

#[derive(Debug)]
pub(crate) enum XferCommand {
    Cd(Option<String>),
    Pwd,
    Reset,
    Reboot,
    Ls(Option<String>),
    Local(Option<String>),
}

// TODO find a way to reduce code duplicaiton

named!(ls_path<CompleteStr, XferCommand>,
do_parse!(
	tag_no_case!("ls") >>
	space1 >>
	path: alpha1>>
	(
		XferCommand::Ls(Some(path.to_string()))
	)
)
);

named!(ls_no_path<CompleteStr, XferCommand>,
do_parse!(
	tag_no_case!("ls") >>
	(
		XferCommand::Ls(None)
	)
)
);

named!(ls<CompleteStr, XferCommand>,
alt!(ls_path | ls_no_path)
);

named!(cd_path<CompleteStr, XferCommand>,
do_parse!(
	tag_no_case!("cd") >>
	space1 >>
	path: alpha1>>
	(
		XferCommand::Cd(Some(path.to_string()))
	)
)
);

named!(cd_no_path<CompleteStr, XferCommand>,
do_parse!(
	tag_no_case!("cd") >>
	(
		XferCommand::Cd(None)
	)
)
);

named!(cd<CompleteStr, XferCommand>,
alt!(cd_path | cd_no_path)
);

named!(no_arg<CompleteStr, XferCommand>,
alt!(
	tag_no_case!("pwd") => 	{|_|{XferCommand::Pwd}} |
	tag_no_case!("reboot") => 	{|_|{XferCommand::Reboot}} |
	tag_no_case!("reset") => 	{|_|{XferCommand::Reset}}
)
);

named!( parse_command_inner<CompleteStr, XferCommand>,
	  alt!(cd | ls | no_arg)
);

pub(crate) fn parse_command(cmd: &str) -> nom::IResult<CompleteStr, XferCommand> {
    parse_command_inner(cmd.into())
}
