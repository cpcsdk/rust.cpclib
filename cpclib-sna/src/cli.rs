#![allow(dead_code)]

use std::fmt::Write;

use cpclib_common::itertools::Itertools;
use cpclib_common::winnow::Parser;
use cpclib_common::{parse_value, winnow};
use line_span::LineSpanExt;
use minus::{ExitStrategy, Pager};
use rustyline::error::ReadlineError;
use rustyline::Editor;

use crate::cli::winnow::ascii::space1;
use crate::cli::winnow::combinator::{alt, cut_err, delimited, opt, preceded};
use crate::cli::winnow::error::{
    AddContext, ContextError, ErrMode, ParserError, StrContext, TreeError
};
use crate::cli::winnow::stream::{AsBytes, AsChar, Compare, FindSlice, Stream, StreamIsPartial};
use crate::cli::winnow::token::{tag_no_case, take_until1};
use crate::cli::winnow::{Located, PResult};
use crate::*;

type Source<'src> = winnow::Located<&'src [u8]>;

#[derive(Debug)]
enum Command {
    Disassemble(Option<u32>, Option<u32>),
    Load2(String),
    Memory(Option<u32>, Option<u32>),
    Symbols(Option<String>),
    Help
}

const DATA_WIDTH: usize = 16;

fn mem_to_string(sna: &Snapshot, from: Option<u32>, amount: Option<u32>) -> String {
    let from = from.unwrap_or(0);
    let amount = amount.unwrap_or_else(|| dbg!(sna.memory.len()) as u32 - dbg!(from));

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
                let path = &fname.resolve();
                Snapshot::load(&path)
                    .map(|s| sna2.replace((fname.clone(), s)))
                    .map_err(|e| {
                        eprintln!("Error while loading {}. {}", path.display(), e);
                    });
            },
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
            },

            Command::Disassemble(..) => todo!(),

            Command::Symbols(symbol) => {
                sna.get_chunk("SYMB")
                    .map(|chunk| chunk.ace_symbol_chunk().unwrap())
                    .map(|chunk| (chunk.get_symbols()))
                    //.flatten()
                    .map(|v| {
                        v.into_iter()
                            .for_each(|(sym, addr)| println!("{sym} {addr:X}"))
                    });
            },

            Command::Help => {
                println!("DISASSEMBLE [start [amount]]: Display memory from physical address start for amount bytes");
                println!("MEMORY [start [amount]]: Display memory from physical address start for amount bytes");
                println!("LOAD2 \"fname\": Load the second snapshot fname");
            }
        }
    }
}

fn parse_line<'i, I, Error: ParserError<I>>(input: &mut I) -> PResult<Command, Error>
where
    I: 'i
        + Stream<Slice = &'i [u8]>
        + StreamIsPartial
        + for<'a> Compare<&'a str>
        + for<'s> FindSlice<&'s str>
        + AsBytes,
    <I as Stream>::Slice: AsBytes,
    <I as Stream>::Token: AsChar,
    <I as Stream>::Token: Clone,
    I: for<'a> Compare<&'a [u8; 2]>,
    I: for<'a> Compare<&'a [u8; 1]>,
    Error: AddContext<I, winnow::error::StrContext>
{
    cut_err(alt((
        parse_memory,
        parse_disassemble,
        parse_help,
        parse_load2,
        parse_symbols
    )))
    .context(StrContext::Label("Wrong command"))
    .parse_next(input)
}

fn parse_memory<'i, I, Error: ParserError<I>>(input: &mut I) -> PResult<Command, Error>
where
    I: 'i
        + Stream<Slice = &'i [u8]>
        + StreamIsPartial
        + for<'a> Compare<&'a str>
        + for<'s> FindSlice<&'s str>
        + AsBytes,
    <I as Stream>::Slice: AsBytes,
    <I as Stream>::Token: AsChar,
    <I as Stream>::Token: Clone,
    I: for<'a> Compare<&'a [u8; 2]>,
    I: for<'a> Compare<&'a [u8; 1]>,
    Error: AddContext<I, winnow::error::StrContext>
{
    (
        alt((tag_no_case("MEMORY"), tag_no_case("MEM"))),
        opt(preceded(space1, parse_value)),
        opt(preceded(space1, parse_value))
    )
        .map(|v| Command::Memory(v.1, v.2))
        .parse_next(input)
}

fn parse_disassemble<'i, I, Error: ParserError<I>>(input: &mut I) -> PResult<Command, Error>
where
    I: 'i
        + Stream<Slice = &'i [u8]>
        + StreamIsPartial
        + for<'a> Compare<&'a str>
        + for<'s> FindSlice<&'s str>
        + AsBytes,
    <I as Stream>::Slice: AsBytes,
    <I as Stream>::Token: AsChar,
    <I as Stream>::Token: Clone,
    I: for<'a> Compare<&'a [u8; 2]>,
    I: for<'a> Compare<&'a [u8; 1]>,
    Error: AddContext<I, winnow::error::StrContext>
{
    (
        alt((
            tag_no_case("DISASSEMBLE"),
            tag_no_case("DISASS"),
            tag_no_case("DIS")
        )),
        opt(preceded(space1, parse_value)),
        opt(preceded(space1, parse_value))
    )
        .map(|v| Command::Disassemble(v.1, v.2))
        .parse_next(input)
}

fn parse_symbols<'i, I, Error: ParserError<I>>(input: &mut I) -> PResult<Command, Error>
where
    I: 'i
        + Stream<Slice = &'i [u8]>
        + StreamIsPartial
        + for<'a> Compare<&'a str>
        + for<'s> FindSlice<&'s str>
        + AsBytes,
    <I as Stream>::Slice: AsBytes,
    <I as Stream>::Token: AsChar,
    <I as Stream>::Token: Clone,
    I: for<'a> Compare<&'a [u8; 2]>,
    I: for<'a> Compare<&'a [u8; 1]>,
    Error: AddContext<I, winnow::error::StrContext>
{
    (alt((
        tag_no_case("SYMBOLS"),
        tag_no_case("SYMB"),
        tag_no_case("S")
    )))
    .map(|v| Command::Symbols(None))
    .parse_next(input)
}

fn parse_help<'i, I, Error: ParserError<I>>(input: &mut I) -> PResult<Command, Error>
where
    I: 'i
        + Stream<Slice = &'i [u8]>
        + StreamIsPartial
        + for<'a> Compare<&'a str>
        + for<'s> FindSlice<&'s str>
        + AsBytes,
    <I as Stream>::Slice: AsBytes,
    <I as Stream>::Token: AsChar,
    <I as Stream>::Token: Clone,
    I: for<'a> Compare<&'a [u8; 2]>,
    I: for<'a> Compare<&'a [u8; 1]>,
    Error: AddContext<I, winnow::error::StrContext>
{
    tag_no_case("HELP").map(|_| Command::Help).parse_next(input)
}

fn parse_load2<'i, I, Error: ParserError<I>>(input: &mut I) -> PResult<Command, Error>
where
    I: 'i
        + Stream<Slice = &'i [u8]>
        + StreamIsPartial
        + for<'a> Compare<&'a str>
        + for<'s> FindSlice<&'s str>
        + AsBytes,
    <I as Stream>::Slice: AsBytes,
    <I as Stream>::Token: AsChar,
    <I as Stream>::Token: Clone,
    I: for<'a> Compare<&'a [u8; 2]>,
    I: for<'a> Compare<&'a [u8; 1]>,
    Error: AddContext<I, winnow::error::StrContext>
{
    preceded(
        (
            tag_no_case("LOAD2"),
            cut_err(space1).context(StrContext::Label("LOAD2 expects a filename"))
        ),
        cut_err(
            delimited('"', take_until1("\""), '"')
                .context(StrContext::Label("Filename needs to be in a string"))
        )
    )
    .map(|fname: &[u8]| Command::Load2(String::from_utf8_lossy(fname).into_owned()))
    .parse_next(input)
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

                let line = line.as_bytes();

                let mut src = Source::new(line);
                match parse_line::<Source, ContextError>.parse(src) {
                    Ok(cmd) => cmd.handle(&mut sna, &mut sna2),
                    Err(e) => {
                        // Coded as if there ere several lines
                        let input = e.input().as_bytes();
                        let input = unsafe { std::str::from_utf8_unchecked(input) };
                        let offset = e.offset();

                        let range = input.find_line_range(offset);
                        assert_eq!(range.start, 0);
                        let pos_in_line = offset - range.start;

                        let line = &input[range];
                        eprintln!("{line}");
                        for _ in 0..offset {
                            eprint!(" ");
                        }
                        eprintln!("^");
                        eprintln!("{}", e.inner());
                    },
                    _ => todo!()
                }
            },
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
