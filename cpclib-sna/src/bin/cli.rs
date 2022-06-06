
#![allow(dead_code)]


use rustyline::error::ReadlineError;
use rustyline::Editor;
use cpclib_sna::*;
use cpclib_common::nom::*;
use cpclib_common::{bin_number, dec_number, hex_number, LocatedSpan};
use cpclib_common::nom::{branch::alt, sequence::tuple, bytes::complete::tag_no_case, character::complete::space1, combinator::map};
use cpclib_common::itertools::Itertools;
use crate::cli::error::VerboseError;


type Source<'src> = LocatedSpan<&'src str>;

enum Command {
	Memory(u32, u32),
	Disassemble(u32, u32)
}

const DATA_WIDTH: usize = 16;

impl Command {
	fn handle(&self, sna: &mut Snapshot) {
		match self {
			Command::Memory(from, amount) => {
				sna.unwrap_memory_chunks();
				let mem = (*from..(*from+*amount)).into_iter()
							.map(move|addr| sna.get_byte(addr))
							.chunks(DATA_WIDTH).into_iter()
							.enumerate()
							.map(|(i,bytes)| {
								let bytes = bytes.collect_vec();
								let hex = bytes.iter().map(|byte| format!("{:02X}", byte)).join(" ");

								let addr = DATA_WIDTH * i + (*from) as usize ;
							
								let chars = bytes.iter().map(|byte| char::from_u32(*byte as u32).map(|c| if c < ' ' || c > '~' {'.'} else {c}).unwrap_or('.')).collect::<String>();

								format!("{:04X}: {:48}|{:16}|", addr, hex, chars)
							})
							.join("\n");

				println!("{}", mem);
			},

			Command::Disassemble(_, _) => todo!()
		}
	}
}

fn parse_number<'src>(input: Source<'src>) -> IResult<Source<'src>, u32, VerboseError<Source<'src>>> {
	alt((
		hex_number,
		bin_number,
		dec_number
	))(input)
}

fn parse_line<'src>(input: Source<'src>) -> IResult<Source<'src>, Command, VerboseError<Source<'src>>> {
	alt((
		parse_memory,
		parse_disassemble
	))(input)
}

fn parse_memory<'src>(input: Source<'src>) -> IResult<Source<'src>, Command, VerboseError<Source<'src>>> {
	map(
		tuple((
			alt((
				tag_no_case("MEMORY"),
				tag_no_case("MEM")
			)),
			space1,
			parse_number,
			space1,
			parse_number
		)),
		|v| Command::Memory(v.2, v.4)
	)(input)
}

fn parse_disassemble<'src>(input: Source<'src>) -> IResult<Source<'src>, Command, VerboseError<Source<'src>>> {
	map(
		tuple((
			alt((
				tag_no_case("DISASSEMBLE"),
				tag_no_case("DISASS")
			)),
			space1,
			parse_number,
			space1,
			parse_number
		)),
		|v| Command::Memory(v.2, v.4)
	)(input)
}


fn handle_line(sna: &mut Snapshot, line: String) {
	let src = Source::new(line.as_str());
	match parse_line(src) {
		Ok( (_input, cmd)) => cmd.handle(sna),
		Err(_) => eprintln!("Wrong command")
	}
}


pub fn cli(mut sna: Snapshot) {
    // `()` can be used when no completer is required
    let mut rl = Editor::<()>::new();
    if rl.load_history("snapshot.txt").is_err() {
    }
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
				handle_line(&mut sna, line);
            },
            Err(ReadlineError::Interrupted) => {
                break
            },
            Err(ReadlineError::Eof) => {
                break
            },
            Err(err) => {
                println!("Error: {:?}", err);
                break
            }
        }
    }
    rl.save_history("snapshot.txt").unwrap();
}
