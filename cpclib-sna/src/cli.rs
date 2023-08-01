#![allow(dead_code)]

use std::fmt::Write;

use cpclib_common::itertools::Itertools;
use cpclib_common::nom::branch::*;
use cpclib_common::nom::bytes::complete::*;
use cpclib_common::nom::character::complete::*;
use cpclib_common::nom::combinator::*;
use cpclib_common::nom::error::*;
use cpclib_common::nom::sequence::*;
use cpclib_common::nom::*;
use cpclib_common::{bin_number, dec_number, hex_number, LocatedSpan};
use minus::{ExitStrategy, Pager};
use rustyline::error::ReadlineError;
use rustyline::Editor;

use crate::*;

type Source<'src> = LocatedSpan<&'src str>;

enum Command {
    Disassemble(Option<u32>, Option<u32>),
    Load2(String),
    Memory(Option<u32>, Option<u32>),
    Help
}

const DATA_WIDTH: usize = 16;

fn mem_to_string(sna: &Snapshot, from: Option<u32>, amount: Option<u32>) -> String {
    let from = from.unwrap_or(0);
    let amount = amount.unwrap_or_else(|| sna.memory.len() as u32 - from);

    (from..(from + amount))
        .map(move |addr| sna.get_byte(addr))
        .chunks(DATA_WIDTH)
        .into_iter()
        .enumerate()
        .map(|(i, bytes)| {
            let bytes = bytes.collect_vec();
            let hex = bytes.iter().map(|byte| format!("{:02X}", byte)).join(" ");

            let addr = DATA_WIDTH * i + (from) as usize;

            let chars = bytes
                .iter()
                .map(|byte| {
                    char::from_u32(*byte as u32)
                        .map(|c| {
                            if !(' '..='~').contains(&c) {
                                '.'
                            }
                            else {
                                c
                            }
                        })
                        .unwrap_or('.')
                })
                .collect::<String>();

            format!("{:04X}: {:48}|{:16}|", addr, hex, chars)
        })
        .join("\n")
}

fn diff_lines(first: &str, second: &str) -> String {
    first
        .lines()
        .zip(second.lines())
        .map(|(line1, line2)| {
            if line1 != line2 {
                format!("{}\t{}", line1, line2)
            }
            else {
                "...".to_string()
            }
        })
        .unique()
        .join("\n")
}

impl Command {
    fn handle(self, sna: &mut Snapshot, sna2: &mut Option<(String, Snapshot)>) {
        match self {
            Command::Load2(fname) => {
                use cpclib_common::resolve_path::*;
                let path = fname.resolve();
                Snapshot::load(&path)
                    .map(|s| sna2.replace((fname.clone(), s)))
                    .map_err(|e| {
                        eprintln!("Error while loading {}. {}", path.display(), e);
                    });
            }
            Command::Memory(from, amount) => {
                let mut output = Pager::new();
                output.set_exit_strategy(ExitStrategy::PagerQuit).unwrap();
                output.set_prompt("MEM").unwrap();

                sna.unwrap_memory_chunks();
                let mem = mem_to_string(sna, from, amount);

                if let Some((_, sna2)) = sna2 {
                    sna2.unwrap_memory_chunks();
                    let mem2 = mem_to_string(sna2, from, amount);
                    let diff = diff_lines(&mem, &mem2);
                    write!(output, "{}", diff);
                }
                else {
                    write!(output, "{}", mem);
                }

                minus::page_all(output).unwrap();
                dbg!("exit pager");
            }

            Command::Disassemble(..) => todo!(),

            Command::Help => {
                println!("DISASSEMBLE [start [amount]]: Display memory from physical address start for amount bytes");
                println!("MEMORY [start [amount]]: Display memory from physical address start for amount bytes");
                println!("LOAD2 \"fname\": Load the second snapshot fname");
            }
        }
    }
}

fn parse_number(input: Source<'_>) -> IResult<Source<'_>, u32, VerboseError<Source<'_>>> {
    alt((hex_number, bin_number, dec_number))(input)
}

fn parse_line(input: Source<'_>) -> IResult<Source<'_>, Command, VerboseError<Source<'_>>> {
    alt((parse_memory, parse_disassemble, parse_help, parse_load2))(input)
}

fn parse_memory(input: Source<'_>) -> IResult<Source<'_>, Command, VerboseError<Source<'_>>> {
    map(
        tuple((
            alt((tag_no_case("MEMORY"), tag_no_case("MEM"))),
            opt(preceded(space1, parse_number)),
            opt(preceded(space1, parse_number))
        )),
        |v| Command::Memory(v.1, v.2)
    )(input)
}

fn parse_disassemble(input: Source<'_>) -> IResult<Source<'_>, Command, VerboseError<Source<'_>>> {
    map(
        tuple((
            alt((tag_no_case("DISASSEMBLE"), tag_no_case("DISASS"))),
            opt(preceded(space1, parse_number)),
            opt(preceded(space1, parse_number))
        )),
        |v| Command::Disassemble(v.1, v.2)
    )(input)
}

fn parse_help(input: Source<'_>) -> IResult<Source<'_>, Command, VerboseError<Source<'_>>> {
    map(tag_no_case("HELP"), |_| Command::Help)(input)
}

fn parse_load2(input: Source<'_>) -> IResult<Source<'_>, Command, VerboseError<Source<'_>>> {
    map(
        preceded(
            tuple((tag_no_case("LOAD2"), space1)),
            cut(context(
                "Filename needs to be in a string",
                recognize(delimited(char('"'), take_until("\""), char('"')))
            ))
        ),
        |fname: Source| Command::Load2(fname[1..(fname.len() - 1)].to_string())
    )(input)
}

pub fn cli(fname: &str, mut sna: Snapshot) {
    let mut sna2: Option<(String, Snapshot)> = None;

    // `()` can be used when no completer is required
    let mut rl = Editor::<(), _>::new().unwrap();
    if rl.load_history("snapshot.txt").is_err() {}
    loop {
        let fname1 = std::path::Path::new(fname)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        let prompt = if let Some((fname2, _)) = &sna2 {
            let fname2 = std::path::Path::new(fname2)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();
            format!("{} vs {} > ", fname1, fname2)
        }
        else {
            format!("{} > ", fname1)
        };

        let readline = rl.readline(&prompt);
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());

                let src = Source::new(line.as_str());
                match parse_line(src) {
                    Ok((_input, cmd)) => cmd.handle(&mut sna, &mut sna2),
                    Err(e) => eprintln!("Wrong command. {}", e)
                }
            }
            Err(ReadlineError::Interrupted) => break,
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    rl.save_history("snapshot.txt").unwrap();
}
