use nom::multispace;
use nom::types::CompleteStr;
use nom::{
    alphanumeric, alphanumeric1, eol, line_ending, space, space0, space1, Err, ErrorKind, IResult,
};
use nom::{InputIter, InputLength};

use std::path::{PathBuf};

use crate::assembler::tokens::*;
use crate::assembler::AssemblerError;
use either::*;
use std::iter;

pub mod error_code {
    pub const ASSERT_MUST_BE_FOLLOWED_BY_AN_EXPRESSION: u32 = 128;
    pub const INVALID_ARGUMENT: u32 = 129;
    pub const UNABLE_TO_PARSE_INNER_CONTENT: u32 = 130;
}

/// Context information that can guide the parser
#[derive(Default, Clone, Debug)]
pub struct ParserContext {
    /// Filename that is currently parsed
    current_filename: Option<PathBuf>,
    /// Search path to find files
    search_path: Vec<PathBuf>,
}

impl ParserContext {
    pub fn set_current_filename<P: Into<PathBuf>>(&mut self, file: P) {
        self.current_filename = Some(file.into().canonicalize().unwrap())
    }

    /// Add a search path and ensure it is ABSOLUTE
    pub fn add_search_path<P: Into<PathBuf>>(&mut self, path: P) {
        self.search_path.push(path.into().canonicalize().unwrap())
    }

    /// Add the folder that contains the given file. Panic if there are issues with the filename
    pub fn add_search_path_from_file<P: Into<PathBuf>>(&mut self, file: P) {
        let path = file
            .into()
            .canonicalize()
            .unwrap()
            .parent()
            .unwrap()
            .to_owned();
        self.add_search_path(path);
    }

    /// Return the real path name that correspond to the requested file
    pub fn get_path_for<P: Into<PathBuf>>(&self, fname: P) -> Option<PathBuf> {
        let fname = fname.into();

        // We expect the file to exists if no search_path is provided
        if self.search_path.is_empty() {
            if fname.is_file() {
                return Some(fname);
            } else {
                return None;
            }
        } else {
            // loop over all possibilities
            for search in self.search_path.iter() {
                let current_path = dbg!(search.join(fname.clone()));

                if current_path.is_file() {
                    return Some(current_path);
                }
            }
        }

        // No file found
        None
    }
}

/// Produce the stream of tokens. In case of error, return an explanatory string.
/// In case of success loop over all the tokens in order to expand those that read files
pub fn parse_str_with_context(code: &str, ctx: &ParserContext) -> Result<Listing, AssemblerError> {
    match parse_z80_code(code.into()) {
        Err(e) => Err(AssemblerError::SyntaxError {
            error: format!("Error while parsing: {:?}", e),
        }),
        Ok((remaining, mut parsed)) => {
            if remaining.len() > 0 {
                eprintln!("{:?}", parsed);
                Err(AssemblerError::BugInParser {
                    error: format!(
                        "Bug in the parser. The remaining source has not been assembled:\n{}",
                        remaining
                    ),
                })
            } else {
                for token in parsed.listing_mut().iter_mut() {
                    token.read_referenced_file(ctx)?;
                }
                Ok(parsed)
            }
        }
    }
}

pub fn parse_str(code: &str) -> Result<Listing, AssemblerError> {
    parse_str_with_context(code, &Default::default())
}

named! (
    pub parse_z80_code <CompleteStr<'_>, Listing>,
    do_parse!(
//        // Skip empty beginning
//       many0!(parse_empty_line) >>
//        opt!(line_ending) >>

        // Get optional code
        tmp: many0!(
                parse_z80_line
        ) >>

//        // Skip empty end
//        many0!(parse_empty_line) >>

        ({
            let mut res: Vec<Token> = Vec::new();
            for list in tmp {

         //       println!("Current list: {:?}", &list);
                res.append(&mut (list.clone()) );
            }
         //   println!("Opcodes: {:?}", &res);
            res.into()
        })
        )
    );

/// For an unknwon reason, the parse_z80_code function fails when there is no comment...
/// This one is a workaround as still as the other is not fixed
pub fn parse_z80_str(code: &str) -> Result<(CompleteStr<'_>, Listing), Err<CompleteStr<'_>>> {
    let mut tokens = Vec::new();
    let mut rest = None;
    let src = "<str>";

    for (line_number, line) in code.split("\n").enumerate() {
        let res = parse_z80_line(CompleteStr(line));
        match res {
            Ok((res, local_tokens)) => {
                tokens.extend_from_slice(&local_tokens);
                rest = Some(res);
            }
            Err(e) => {
                let error_string = format!("Error at line {}: {}", line_number, line);
                eprintln!("{}:{} ({}) {}", src, line_number, line, error_string);
                return Err(e);
            }
        }
    }
    Ok((rest.unwrap(), tokens.into()))
}

// TODO find a way to not use the alternative stuff
named!(
    pub parse_z80_line<CompleteStr<'_>, Vec<Token>>,
        alt_complete!(
            many1!(eol) => {|_|{Vec::new()}} |
            parse_empty_line |
            parse_repeat => {|repeat| vec![repeat]} |
            parse_macro => {|m| vec![m]} |
            parse_basic => {|basic| vec![basic]}|
            parse_rorg => {|rorg| vec![rorg]}|
            preceded!(space1, parse_conditional) => {|cond| vec![cond]}|
            parse_z80_line_label_only |
            parse_z80_line_complete
        )
    );

named!(
    pub parse_rorg <CompleteStr<'_>, Token>, do_parse!(
        opt!(multispace) >>
        alt!(
            tag_no_case!("PHASE") |
            tag_no_case!("RORG")
        ) >>
        space1 >>
        exp: expr >>
        space0 >>
        eol >>
        inner: opt!(add_return_error!(
            ErrorKind::Custom(error_code::UNABLE_TO_PARSE_INNER_CONTENT),
            parse_z80_code
        )) >> 
        multispace >>
        alt!(
            tag_no_case!("DEPHASE") |
            tag_no_case!("REND")
        ) >>
        (
            Token::Rorg(exp,
                if inner.is_some() {
                    inner.unwrap().into()
                }
                else {
                    Vec::new().into()
                }
            )
        )
    )
);

named!(
    pub parse_macro<CompleteStr<'_>, Token>, do_parse!(
        opt!(multispace) >>
        tag_no_case!("MACRO") >>
        space1 >>
        name: parse_label >> // TODO use a specific function for that
        // TODO treat args
        multispace >>
        content: many_till!(
            take!(1),
            tag_no_case!("ENDM")
        ) >>
        (
            Token::Macro(
                name,
                Vec::new(),
                content.0.iter().map(|s|->String{s.to_string()}).collect::<String>()
            )
        )
    )
);

named!(
    pub parse_repeat<CompleteStr<'_>, Token>, do_parse!(
        opt!(multispace) >>
        alt!(
            tag_no_case!("REPEAT") | 
            tag_no_case!("REPT") | 
            tag_no_case!("REP")
         ) >>
        space1 >>
        count: expr >>
        inner: opt!(parse_z80_code) >> 
        multispace >>
        alt!(
            tag_no_case!("ENDREPEAT") 
            | tag_no_case!("ENDREPT") 
            | tag_no_case!("ENDREP")
            | tag_no_case!("ENDR")
         ) >>
        space0 >>

        (
            Token::Repeat(
                count, 
                if inner.is_some() {
                    inner.unwrap().into()
                }
                else {
                    Vec::new().into()
                },
                None
            )
        )
    )
);

/// Parse a Basic bloc.
named!(
    pub parse_basic<CompleteStr<'_>, Token>, do_parse!(
        opt!(multispace) >>
        tag_no_case!("LOCOMOTIVE") >>
        space0 >>
        args: opt!(
            separated_nonempty_list!(
                preceded!(
                    space0, 
                    char!(',')
                ),
                preceded!(
                    space0,
                    map!(
                        parse_label,
                        |s|{s.to_string()}
                    )
                )    
            )
        ) >>
        space0 >>
        opt!(tag!("\r")) >>
        tag!("\n") >>
        hidden_lines: opt!(
                    terminated!(
                        preceded!(
                            opt!(multispace),
                            parse_basic_hide_lines
                        ),
                        multispace
                    )
        ) >>
        basic: take_until_and_consume!("ENDLOCOMOTIVE") >>
        space0 >>
        (
            Token::Basic(
                args,
                hidden_lines,
                basic.to_string()
            )
        )
    )
);

named!(
    pub parse_basic_hide_lines<CompleteStr<'_>, Vec<u16>>, do_parse!(
        tag_no_case!("HIDE_LINES") >>
        space1 >>
        lines: separated_nonempty_list!(
            preceded!(
                space0,
                char!(',')
            ),
            preceded!(
                space0,
                dec_u16
            )
        ) >>
        (
            lines
        )
    )
);

named!(
    pub parse_empty_line<CompleteStr<'_>, Vec<Token>>, do_parse!(
        opt!(line_ending) >>
            space0 >>
            comment: opt!(comment) >>
            alt!(
                line_ending |
                eof!()
            )>>
        ({
            let mut res = Vec::new();
            if comment.is_some() {
                res.push(comment.unwrap());
            }
            res
        })
        )
    );

// TODO add an argument o manage cases like '... : ENDIF'
named!(
    pub parse_z80_line_complete <CompleteStr<'_>, Vec<Token>>, do_parse!(
        opt!(line_ending) >>
        label: opt!(parse_label) >>
       // opt!(char!(':')) >> // XXX need to move that
        space1 >>
        opcode: alt!(parse_token | parse_directive) >>
        additional_opcodes: fold_many0!(
                do_parse!(
                    space0 >>
 //                   alt_complete!(
                        tag!(":") >>
   //                     delimited!(many0!(space), tag!("\n"), many1!(space))
     //               ) >>
                    space0 >>
                    inner_opcode:return_error!(
                                        ErrorKind::Custom(error_code::INVALID_ARGUMENT),
                                        alt_complete!(parse_token | parse_directive)
                                        )>>
                    (inner_opcode)
                ),
                Vec::new(),
                |mut acc: Vec<_>, item| {
                    acc.push(item);
                    acc
                }

        ) >>
        space0 >>
        comment: opt!(comment) >>
        alt_complete!(line_ending | eof!()) >>
        ({
            let mut tokens = Vec::new();
            if label.is_some() {
                tokens.push(Token::Label(label.unwrap()));
            }
            tokens.push(opcode);
            for opcode in additional_opcodes {
                tokens.push(opcode);
            }
            if comment.is_some() {
                tokens.push(comment.unwrap());
            }

            tokens
        })
        )
    );

/**
 * No opcodes are expected there.
 * Initially it was supposed to manage lines with only labels, however it has been extended
 * to labels fallowed by specific commands.
 */
named!(
       pub parse_z80_line_label_only <CompleteStr<'_>, Vec<Token>>, do_parse!(
        opt!(line_ending) >>
            label: parse_label >>

            // TODO make these stuff alternatives ...
            // Manage Equ
            equ: opt!(
                preceded!(
                    preceded!(
                        many1!(space),
                        tag_no_case!("EQU")
                    ),
                    preceded!(
                            many1!(space),
                            expr
                            )
                        )
            )
            >>

           // opt!(char!(':')) >>
            space0 >>
            comment: opt!(comment) >>
            alt_complete!(line_ending | eof!()) >>
            ({
                let mut tokens = Vec::new();

                if equ.is_some() {
                    tokens.push(Token::Equ(label, equ.unwrap()));
                }
                else {
                    tokens.push(Token::Label(label));
                }
                if comment.is_some() {
                    tokens.push(comment.unwrap());
                }

                tokens
            })
            )
        );

named!(
    parse_include<CompleteStr<'_>, Token>,
    do_parse!(
        tag_no_case!("INCLUDE")
            >> space1
            >> fname:
                alt!(
                    preceded!(tag!("\""), take_until_and_consume1!("\""))
                        | preceded!(tag!("'"), take_until_and_consume1!("'"))
                )
            >> (Token::Include(fname.to_string(), None))
    )
);

/// TODO add the missing optional parameters
named!(
    parse_incbin<CompleteStr<'_>, Token>,
    do_parse!(
        tag_no_case!("INCBIN")
            >> space1
            >> fname:
                alt!(
                    preceded!(tag!("\""), take_until_and_consume1!("\""))
                        | preceded!(tag!("'"), take_until_and_consume1!("'"))
                )
            >> (Token::Incbin(fname.to_string(), None, None, None, None, None))
    )
);

named!(
    parse_undef<CompleteStr<'_>, Token>,
    do_parse!(tag_no_case!("UNDEF") >> space1 >> label: parse_label >> (Token::Undef(label)))
);

named!(
    parse_token<CompleteStr<'_>, Token>,
    alt_complete!(
        parse_ex_af
            | parse_logical_operator
            | parse_add_or_adc
            | parse_cp
            | parse_djnz
            | parse_ld
            | parse_inc_dec
            | parse_out
            | parse_in
            | parse_call_jp_or_jr
            | parse_opcode_no_arg
            | parse_push_n_pop
            | parse_res_set_bit
            | parse_shifts
            | parse_ret
    )
);

named!(
    parse_ex_af<CompleteStr<'_>, Token>,
    do_parse!(
        tag_no_case!("EX")
            >> space1
            >> tag_no_case!("AF")
            >> space0
            >> char!(',')
            >> space0
            >> tag_no_case!("AF'")
            >> (Token::OpCode(Mnemonic::ExAf, None, None))
    )
);
named!(
    parse_directive<CompleteStr<'_>, Token>,
    alt_complete!(
        parse_assert
            | parse_align
            | parse_breakpoint
            | parse_org
            | parse_defs
            | parse_include
            | parse_incbin
            | parse_db_or_dw
            | parse_print
            | parse_protect
            | parse_stable_ticker
            | parse_undef
            | parse_noarg_directive
            | parse_macro_call
    )
);

named!(
    pub parse_noarg_directive <CompleteStr<'_>, Token>,
    alt_complete!(
        value!(Token::List, tag_no_case!("list")) |
        value!(Token::NoList, tag_no_case!("nolist"))
    )
);


const IF_CODE: u8 = 0;
const IFNOT_CODE: u8 = 1;
const IFDEF_CODE: u8 = 2;
const IFNDEF_CODE: u8 = 4;

/// Parse if expression.
/// TODO finish the implementation in order to have ELSEIF and ELSE branches
named!(
    pub parse_conditional<CompleteStr<'_>, Token>, do_parse!(

        // Gest the kind of test to do
        test_kind: alt!(
            value!(IFNOT_CODE, tag_no_case!("IFNOT")) |
            value!(IFDEF_CODE, tag_no_case!("IFDEF")) |
            value!(IFNDEF_CODE, tag_no_case!("IFNDEF")) |
            value!(IF_CODE, tag_no_case!("IF")) 
         ) >>

        // Get the corresponding test
        cond: delimited!(
            space1, 
            alt!(
                cond_reduce!(
                    test_kind == IF_CODE,
                    map!(expr, |e|{TestKind::True(e)})
                ) |
                cond_reduce!(
                    test_kind == IFNOT_CODE,
                    map!(expr, |e|{TestKind::False(e)})
                ) |
                cond_reduce!(
                    test_kind == IFDEF_CODE,
                    map!(parse_label, |l|{TestKind::LabelExists(l)})
                ) |
                cond_reduce!(
                    test_kind == IFNDEF_CODE,
                   map!(parse_label, |l|{TestKind::LabelDoesNotExist(l)})
                )
            ), 
            space0
        ) >>
        
        alt!(eol | tag!(":")) >>

        code: return_error!(
            ErrorKind::Custom(error_code::UNABLE_TO_PARSE_INNER_CONTENT),
            parse_z80_code
        ) >>


        r#else: opt!(
            preceded!(
                delimited!(
                    space0, 
                    tag_no_case!("ELSE"), 
                    alt!( 
                        terminated!(space0, eol) | 
                        tag!(":")
                    )
                ),
                parse_z80_code
            )
        ) >>

        alt!( space1 | delimited!(space0, tag!(":"), space0)) >>


        tag_no_case!("ENDIF") >>

        (
            Token::If(
                vec![(cond, code)],
                r#else
            )
        )
    )    
);

named!(
    parse_breakpoint<CompleteStr<'_>, Token>,
    do_parse!(
        tag_no_case!("BREAKPOINT")
            >> exp: opt!(preceded!(space1, expr))
            >> (Token::Breakpoint(exp))
    )
);

named!(
    parse_stable_ticker<CompleteStr<'_>, Token>,
    alt!(parse_stable_ticker_start | parse_stable_ticker_stop)
);

named!(
    parse_stable_ticker_start<CompleteStr<'_>, Token>,
    do_parse!(
        opt!(tag_no_case!("stable"))
            >> tag_no_case!("ticker")
            >> space1
            >> tag_no_case!("start")
            >> space1
            >> name: parse_label
            >> (Token::StableTicker(StableTickerAction::Start(name)))
    )
);

named!(
    parse_stable_ticker_stop<CompleteStr<'_>, Token>,
    do_parse!(
        opt!(tag_no_case!("stable"))
            >> tag_no_case!("ticker")
            >> space1
            >> tag_no_case!("stop")
            >> (Token::StableTicker(StableTickerAction::Stop))
    )
);

named!(
    pub parse_ld <CompleteStr<'_>, Token>,
        alt!(
            parse_ld_fake  |
            parse_ld_normal
    )
);

named!(
    pub parse_ld_fake <CompleteStr<'_>, Token>, do_parse!(
        tag_no_case!("LD") >>
        space1 >>
        dst: parse_register16 >>
        opt!(space1) >>
        tag!(",") >>
        opt!(space1) >>
        src: parse_register16 >>
        not!(alphanumeric1)>>
         (
             Token::OpCode(Mnemonic::Ld, Some(dst), Some(src))
         )
    )
);

named!(
    pub parse_ld_normal <CompleteStr<'_>, Token>, do_parse!(
        opt!(multispace) >>
        tag_no_case!("LD") >>
        space1 >>
        dst: return_error!(
            ErrorKind::Custom(error_code::INVALID_ARGUMENT),
            alt_complete!( parse_reg_address |
                           parse_register_sp |
                           parse_register16 |
                           parse_register8 |
                           parse_indexregister16 |
                           parse_indexregister8 |
                           parse_register_i |
                           parse_address)
        ) >>
        space0 >>
        tag!(",") >>
        space0 >>
        // src possibilities depend on dst
        src: return_error!(
            ErrorKind::Custom(error_code::INVALID_ARGUMENT),
            alt_complete!(
                cond_reduce!(
                    dst.is_register16() | dst.is_indexregister16(),
                    alt_complete!(parse_address | parse_expr)
                ) |
                cond_reduce!(
                    dst.is_register8(), 
                    alt_complete!(
                        parse_indexregister_with_index | parse_hl_address | parse_address | parse_expr | parse_register8)
                ) |
                cond_reduce!(
                    dst.is_memory(), 
                    alt_complete!(
                        parse_register16 | parse_register8 | parse_register_sp)
                ) |
                cond_reduce!(
                    dst.is_address_in_register16(), 
                    parse_register8
                ) |
                cond_reduce!(
                    dst.is_register_i(),
                    parse_register_a
                )
            )
        )
         >>
        (Token::OpCode(Mnemonic::Ld, Some(dst), Some(src)))
        )
    );

named!(
    pub parse_res_set_bit <CompleteStr<'_>, Token>, do_parse!(
        res_or_set: alt!(
            tag_no_case!("RES") => { |_|Mnemonic::Res} |
            tag_no_case!("BIT") => { |_|Mnemonic::Bit} |
            tag_no_case!("SET") => { |_|Mnemonic::Set}
        ) >>

        space1 >>

        bit: parse_expr >>

        delimited!(space0, tag!(","), space0) >>

        operand: alt!(
            parse_register8 |
            parse_hl_address |
            parse_indexregister_with_index
        )>> 
        (
            Token::OpCode(res_or_set, Some(bit), Some(operand))
        )
    )
);

named!(
    pub parse_cp <CompleteStr<'_>, Token>, do_parse!(
        tag_no_case!("CP") >>
        space1 >>
        operand:  alt!(
            parse_register8 |
            parse_hl_address |
            parse_indexregister_with_index |
            parse_expr 
        ) >>
        (
            Token::OpCode(
                Mnemonic::Cp,
                Some(operand),
                None
            )
        )
    )
);

named!(
  pub parse_db_or_dw <CompleteStr<'_>, Token>, do_parse!(
    is_db: alt!(
        alt!(
            tag_no_case!("DB") |
            tag_no_case!("DEFB") 
         )  => {|_| {true}} |
        alt!(
            tag_no_case!("DW") |
            tag_no_case!("DEFW")
         ) => {|_| {false}}
    ) >>
    many1!(space) >>
    expr: expr_list >>
    ({
        if is_db {
            Token::Defb(expr)
        }
        else {
            Token::Defw(expr)
        }
    })
  )
);


named!(
    pub parse_macro_call <CompleteStr<'_>, Token>, do_parse!(
        name: parse_label >>
        args: opt!(
            alt!(
                expr_list |
                tag_no_case!("(void)") => {|_| Vec::new()}
            )
        ) >>
        (
            Token::MacroCall(name, args.unwrap_or(Vec::new()))
        )
    )
);

named!(
    pub parse_djnz<CompleteStr<'_>, Token>, do_parse!(
        tag_no_case!("DJNZ") >>
        many1!(space) >>
        expr: parse_expr >>
        (Token::OpCode(Mnemonic::Djnz, Some(expr), None))
    )
);

named!(
    pub expr_list <CompleteStr<'_>, Vec<Expr>>,
        separated_nonempty_list!(
            do_parse!(tag!(",") >> many0!(space) >> ()),
            alt!(expr | string_expr)
            )
    );

named!(
    pub parse_assert <CompleteStr<'_>, Token>, do_parse!(
        tag_no_case!("ASSERT") >>
        space1 >>
        expr: return_error!(
            ErrorKind::Custom(error_code::ASSERT_MUST_BE_FOLLOWED_BY_AN_EXPRESSION),
            expr
        ) >>
        comment: opt!(
            preceded!(
                delimited!(space0, tag!(","), space0),
                delimited!(tag!("\""), take_until!("\""), tag!("\""))
            )
        ) >>
        (
            Token::Assert(expr, comment.map(|s|{s.to_string()}))
        )
    )
);

named!(
    pub parse_align <CompleteStr<'_>, Token>, do_parse!(
        tag_no_case!("ALIGN") >>
        many1!(space) >>
        expr: return_error!(
            ErrorKind::Custom(error_code::ASSERT_MUST_BE_FOLLOWED_BY_AN_EXPRESSION),
            expr
        ) >>
        (
            Token::Align(expr, None)
        )
    )
);

named!(
    pub parse_print <CompleteStr<'_>, Token>, do_parse!(
        tag_no_case!("PRINT") >>
        space1 >>
        exp: alt!(
            expr => {|e|{Left(e)}} |
            string_between_quotes => {|s:CompleteStr<'_>|{Right(s.to_string())}}
        ) >>
        space0 >>
        (
            Token::Print(exp)
        )
    )
);

named!(
    pub parse_protect <CompleteStr<'_>, Token>, do_parse!(
        tag_no_case!("PROTECT") >>
        space1 >>
        start: expr >>
        space0 >>
        tag!(",") >>
        space0 >>
        end: expr >>
        (
            Token::Protect(start, end)
        )
    )
);

/// TODO treat all the cases
named!(
    pub parse_logical_operator<CompleteStr<'_>, Token>, do_parse!(
        operator: alt_complete!(
            value!(Mnemonic::And, tag_no_case!("AND")) |
            value!(Mnemonic::Or, tag_no_case!("Or")) |
            value!(Mnemonic::Xor, tag_no_case!("Xor")) 
        ) >>

        space1 >>


         operand: alt!(
            parse_register8 |
            parse_hl_address |
            parse_indexregister_with_index |
            parse_expr
        ) >>


        (
            Token::OpCode(
                operator,
                Some(operand),
                None
            )
        )          
    )
);

named!(
    pub parse_shifts <CompleteStr<'_>, Token>, do_parse! (
        operator: alt!(
            value!(Mnemonic::Sla, tag_no_case!("SLA")) |
            value!(Mnemonic::Sra, tag_no_case!("SRA")) |
            value!(Mnemonic::Srl, tag_no_case!("SRL")) 
        ) >>

        space1 >>

        operand: alt!(
            parse_register8 |
            parse_hl_address |
            parse_indexregister_with_index
        ) >>

        (
            Token::OpCode(
                operator,
                Some(operand),
                None
            )
        )


    )
);

named!(
    pub parse_add_or_adc <CompleteStr<'_>, Token>, alt_complete!(
        parse_add_or_adc_complete |
        parse_add_or_adc_shorten
    )
);

named!(
    pub parse_add_or_adc_complete <CompleteStr<'_>, Token>, do_parse!(
        add_or_adc: alt_complete!(
            value!(Mnemonic::Adc, tag_no_case!("ADC")) |
            value!(Mnemonic::Add, tag_no_case!("ADD"))
        ) >>

        many1!(space) >>

        first: alt_complete!(
            value!(
                DataAccess::Register8(Register8::A),
                parse_register_a ) |
            value!(
                DataAccess::Register16(Register16::Hl), 
                parse_register_hl) |
            parse_indexregister16
        ) >>

        opt!(space) >>
        tag!(",") >>
        opt!(space) >>

        second: alt_complete!(
            cond_reduce!(
                first.is_register8(), 
                alt_complete!(
                    parse_register8 | 
                    parse_indexregister8 | 
                    parse_hl_address | 
                    parse_indexregister_with_index | 
                    parse_expr)) | // Case for A
            cond_reduce!(
                first.is_register16(), 
                alt_complete!(
                    parse_register16 | 
                    parse_register_sp)) | // Case for HL XXX AF is accepted whereas it is not the case in real life
            cond_reduce!(
                first.is_indexregister16(), 
                alt_complete!(
                    value!(
                        DataAccess::Register16(Register16::De), 
                        tag_no_case!("DE")) |
                    value!(
                        DataAccess::Register16(Register16::Bc), 
                        tag_no_case!("BC")) |
                    value!(
                        DataAccess::Register16(Register16::Sp), 
                        tag_no_case!("SP")) |
                    value!(
                        DataAccess::IndexRegister16(IndexRegister16::Ix),
                         tag_no_case!("IX")) |
                    value!(
                        DataAccess::IndexRegister16(IndexRegister16::Iy),
                        tag_no_case!("IY"))
                    )
            )
        ) >>

        (Token::OpCode(add_or_adc, Some(first), Some(second)))
    )
);

// TODO Find a way to not duplicate code with complete version
named!(
    pub parse_add_or_adc_shorten <CompleteStr<'_>, Token>, do_parse!(
        add_or_adc: alt_complete!(
            value!(Mnemonic::Adc, tag_no_case!("ADC")) |
            value!(Mnemonic::Add, tag_no_case!("ADD"))
        ) >>

        many1!(space) >>

        second: alt_complete!(parse_register8 | parse_hl_address | parse_indexregister_with_index | parse_expr)
        >>
        (Token::OpCode(add_or_adc, Some(DataAccess::Register8(Register8::A)), Some(second)))
    )
);

named!(
    pub parse_push_n_pop <CompleteStr<'_>, Token>, do_parse!(
        push_or_pop: alt_complete!(
            value!(Mnemonic::Push, tag_no_case!("PUSH")) |
            value!(Mnemonic::Pop, tag_no_case!("POP"))) >>
        space >>
        register: alt!(parse_register16 | parse_indexregister16) >>
        (
            Token::OpCode(push_or_pop, Some(register), None)
        )
        )
    );

named!(
    pub parse_ret <CompleteStr<'_>, Token>, do_parse!(
        tag_no_case!("RET") >>
        cond: opt!(preceded!(many1!(space), parse_flag_test)) >>
        (
            Token::OpCode(Mnemonic::Ret, if cond.is_some() {Some(DataAccess::FlagTest(cond.unwrap()))} else {None}, None)
        )
    )
);

named!(
    pub parse_inc_dec<CompleteStr<'_>, Token>, do_parse!(
        inc_or_dec: alt_complete!(
            value!(Mnemonic::Inc, tag_no_case!("INC") ) |
            value!(Mnemonic::Dec, tag_no_case!("DEC"))) >>
        space >>
        register: alt_complete!(
            parse_register16 | parse_register8 | parse_register_sp
            ) >>
        (
            Token::OpCode(inc_or_dec, Some(register), None)
        )
        )
    );

named!(// TODO manage other out formats
    pub parse_out<CompleteStr<'_>, Token>, do_parse!(
        tag_no_case!("OUT") >>

        many1!(space) >>

        tag!("(") >>
        many0!(space) >>
        tag_no_case!("C") >>
        many0!(space) >>
        tag!(")") >>

        many0!(space) >>
        tag!(",") >>
        many0!(space) >>

        reg : parse_register8 >>
        (
            Token::OpCode(
                Mnemonic::Out, 
                Some(DataAccess::Register8(Register8::C)),
                Some(reg)
            )
        )

    )
);

named!(// TODO manage other out formats
    pub parse_in<CompleteStr<'_>, Token>, do_parse!(
        tag_no_case!("IN") >>

        many1!(space) >>
        reg : parse_register8 >>

        many0!(space) >>
        tag!(",") >>
        many0!(space) >>


        tag!("(") >>
        many0!(space) >>
        tag_no_case!("C") >>
        many0!(space) >>
        tag!(")") >>
        (
            Token::OpCode(
                Mnemonic::In, 
                Some(DataAccess::Register8(Register8::C)),
                Some(reg)
            )
        )

    )
);

/// TODO remove multispace
/// TODO reduce the flag space for jr
named!(
    parse_call_jp_or_jr<CompleteStr<'_>, Token>,
    do_parse!(
        call_jp_or_jr:
            alt_complete!(
                value!(Mnemonic::Jp, tag_no_case!("JP"))
                    | value!(Mnemonic::Jr, tag_no_case!("JR"))
                    | value!(Mnemonic::Call, tag_no_case!("CALL"))
            )
            >> space
            >> flag_test:
                opt!(terminated!(
                    parse_flag_test,
                    delimited!(opt!(multispace), tag!(","), opt!(multispace))
                ))
            >> dst: expr
            >> ({
                let flag_test = if flag_test.is_some() {
                    Some(DataAccess::FlagTest(flag_test.unwrap()))
                } else {
                    None
                };
                Token::OpCode(call_jp_or_jr, flag_test, Some(DataAccess::Expression(dst)))
            })
    )
);

named!(
    parse_flag_test<CompleteStr<'_>, FlagTest>,
    alt_complete!(
        value!(FlagTest::NZ, tag_no_case!("NZ"))
            | value!(FlagTest::Z, tag_no_case!("Z"))
            | value!(FlagTest::NC, tag_no_case!("NC"))
            | value!(FlagTest::C, tag_no_case!("C"))
            | value!(FlagTest::PO, tag_no_case!("PO"))
            | value!(FlagTest::PE, tag_no_case!("PE"))
            | value!(FlagTest::P, tag_no_case!("P"))
            | value!(FlagTest::M, tag_no_case!("M"))
    )
);

/*
/// XXX to remove as soon as possible
named!(
    parse_dollar <CompleteStr, Expr>, do_parse!(
        tag!("$") >>
        (Expr::Label(String::from("$")))
        )
    );
*/

named!(
    pub parse_register16 <CompleteStr<'_>, DataAccess>,
        alt_complete!(
            parse_register_hl |
            parse_register_bc |
            parse_register_de |
            parse_register_af
        ) 
);

named!(
    pub parse_register8 <CompleteStr<'_>, DataAccess>, 
        alt_complete!(
            parse_register_a |
            parse_register_b |
            parse_register_c |
            parse_register_d |
            parse_register_e |
            parse_register_h |
            parse_register_l
        )
);

named!(
    pub parse_register_i <CompleteStr<'_>, DataAccess> , 
    value!(DataAccess::SpecialRegisterI, preceded!(tag_no_case!("I"), not!(alphanumeric)))
);

named!(
    pub parse_register_r <CompleteStr<'_>, DataAccess> , 
    value!(DataAccess::SpecialRegisterR, preceded!(tag_no_case!("R"), not!(alphanumeric)))
);

macro_rules! parse_any_register8 {
    ($name: ident, $char:expr, $reg:expr) => {
        named!(
            pub $name <CompleteStr<'_>, DataAccess> ,
            value!(DataAccess::Register8($reg), preceded!(tag_no_case!($char), not!(alphanumeric)))
        );
};
}

parse_any_register8!(parse_register_a, "A", Register8::A);
parse_any_register8!(parse_register_b, "B", Register8::B);
parse_any_register8!(parse_register_c, "C", Register8::C);
parse_any_register8!(parse_register_d, "d", Register8::D);
parse_any_register8!(parse_register_e, "e", Register8::E);
parse_any_register8!(parse_register_h, "h", Register8::H);
parse_any_register8!(parse_register_l, "l", Register8::L);

named!(
    parse_register_sp<CompleteStr<'_>, DataAccess>,
    do_parse!(
        preceded!(tag_no_case!("SP"), not!(alphanumeric))
            >> (DataAccess::Register16(Register16::Sp))
    )
);

named!(
    parse_register_hl<CompleteStr<'_>, DataAccess>,
    do_parse!(
        preceded!(tag_no_case!("HL"), not!(alphanumeric))
            >> (DataAccess::Register16(Register16::Hl))
    )
);

named!(
    parse_register_bc<CompleteStr<'_>, DataAccess>,
    do_parse!(
        preceded!(tag_no_case!("BC"), not!(alphanumeric))
            >> (DataAccess::Register16(Register16::Bc))
    )
);

named!(
    parse_register_de<CompleteStr<'_>, DataAccess>,
    do_parse!(
        preceded!(tag_no_case!("DE"), not!(alphanumeric))
            >> (DataAccess::Register16(Register16::De))
    )
);

named!(
    parse_register_af<CompleteStr<'_>, DataAccess>,
    do_parse!(
        preceded!(tag_no_case!("AF"), not!(alphanumeric))
            >> (DataAccess::Register16(Register16::Af))
    )
);

named!(
    pub parse_indexregister8 <CompleteStr<'_>, DataAccess>, do_parse!(
        reg: alt_complete!(
            tag_no_case!("IXH") => { |_| IndexRegister8::Ixh} |
            tag_no_case!("IXL") => { |_| IndexRegister8::Ixl} |
            tag_no_case!("IYH") => { |_| IndexRegister8::Iyh} |
            tag_no_case!("IYL") => { |_| IndexRegister8::Iyl}
            )
        >>
        (DataAccess::IndexRegister8(reg))
    )
);

named!(
    parse_indexregister16<CompleteStr<'_>, DataAccess>,
    do_parse!(
        reg: alt_complete!(
        tag_no_case!("IX") => { |_| IndexRegister16::Ix} |
        tag_no_case!("IY") => { |_| IndexRegister16::Iy}
        ) >> (DataAccess::IndexRegister16(reg))
    )
);

/// Parse the use of an indexed register as (IX + 5)
named!(
    parse_indexregister_with_index<CompleteStr<'_>, DataAccess>,
    do_parse!(
        tag!("(")
            >> space0
            >> reg: parse_indexregister16
            >> space0
            >> _op: alt_complete!(value!(Oper::Add, tag!("+")) | value!(Oper::Sub, tag!("-")))
            >> space0
            >> expr: expr
            >> space0
            >> tag!(")")
            >> ({
                let target = reg.get_indexregister16().unwrap();
                DataAccess::IndexRegister16WithIndex(target, expr)
            })
    )
);

/// Parse an address access `(expression)`
named!(
    pub parse_address <CompleteStr<'_>, DataAccess>,
    do_parse!(
        tag!("(") >>
        address: expr >>
        tag!(")") >>
        (
            DataAccess::Memory(address)
        )
    )

);

/// Parse (R16)
named!(
    pub parse_reg_address <CompleteStr<'_>, DataAccess>,
    do_parse!(
        tag!("(") >>
        many0!(space) >>
        reg: parse_register16 >>
        many0!(space) >>
        tag!(")") >>
        (
            DataAccess::MemoryRegister16(reg.get_register16().unwrap())
        )
    )

);

/// Parse (HL)
named!(
    pub parse_hl_address<CompleteStr<'_>, DataAccess>,
    do_parse!(
        tag!("(") >>
        many0!(space) >>
        tag_no_case!("HL") >>
        many0!(space) >>
        tag!(")") >>
        (
            DataAccess::MemoryRegister16(Register16::Hl)
        )
    )
);

/// Parse and expression and returns it inside a DataAccession::Expression
named!(
    pub parse_expr <CompleteStr<'_>, DataAccess>,
    do_parse!(
        expr: expr >>
        (
            DataAccess::Expression(expr)
        )
        )
    );

named!(
    pub parse_org <CompleteStr<'_>, Token>, do_parse!(
        tag_no_case!("ORG") >>
        space >>
        val: expr >>
        (Token::Org(val, None))
    )
);

named!(
    pub parse_defs <CompleteStr<'_>, Token>, do_parse!(
        tag_no_case!("DEFS") >>
        space >>
        val: expr >>
        (Token::Defs(val, None))
        )
    );

named!(
  pub parse_opcode_no_arg <CompleteStr<'_>, Token>,
    do_parse!(
    res:alt_complete!(
        tag_no_case!("DI") => { |_| Mnemonic::Di } |
        tag_no_case!("EI") => { |_| Mnemonic::Ei } |
        tag_no_case!("EXX") => { |_| Mnemonic::Exx } |
        tag_no_case!("HALT") => {|_| Mnemonic::Halt } |
        tag_no_case!("LDIR") => { |_| Mnemonic::Ldir } |
        tag_no_case!("LDDR") => { |_| Mnemonic::Lddr } |
        tag_no_case!("LDI") => { |_| Mnemonic::Ldi } |
        tag_no_case!("LDD") => { |_| Mnemonic::Ldd } |
        tag_no_case!("NOPS2") => { |_| Mnemonic::Nops2 } |
        tag_no_case!("NOP") => { |_| Mnemonic::Nop } |
        tag_no_case!("OUTD") => { |_| Mnemonic::Outd } |
        tag_no_case!("OUTI") => { |_| Mnemonic::Outi } |
        tag_no_case!("RRA") => {|_| Mnemonic::Rra } |
        value!(Mnemonic::Scf, tag_no_case!("SCF")) |
        value!(Mnemonic::Ind, tag_no_case!("IND")) |
        value!(Mnemonic::Indr, tag_no_case!("INDR")) |
        value!(Mnemonic::Ini, tag_no_case!("INI")) |
        value!(Mnemonic::Inir, tag_no_case!("INIR"))
      )
    >> (Token::OpCode(res, None, None) )
    )
);

named!(
    pub parse_value <CompleteStr<'_>, Expr>, do_parse!(
        val: alt_complete!(hex_u16 | dec_u16 | bin_u16) >>
        (Expr::Value(val as i32))
        )
    );

named!(
    pub hex_u16 <CompleteStr<'_>, u16>, do_parse!(
        alt!(tag_no_case!("0x") | tag!("#") | tag!("&")) >>
        val: hex_u16_inner >>
        (val)
        )
    );

/// Parse a comment that start by `;` and ends at the end of the line.
named!(
    comment<CompleteStr<'_>, Token>,
    map!(
        preceded!(tag!(";"), take_till!(|ch| ch == '\n')),
        |string| Token::Comment(string.iter_elements().collect::<String>())
    )
);

// Usefull later for db
named!(pub string_between_quotes<CompleteStr<'_>, CompleteStr<'_>>, delimited!(char!('\"'), is_not!("\""), char!('\"')));


named!(
    pub string_expr<CompleteStr<'_>, Expr>,
    map!(
        string_between_quotes,
        |string| Expr::String(string.to_string())
    )
);

pub fn parse_label(input: CompleteStr<'_>) -> IResult<CompleteStr<'_>, String> {
    // Get the label
    match do_parse!(
        input,
        first: one_of!("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.") >> // XXX The inclusion of . is probably problematic
        middle: is_a!("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_.") >>
        (
            format!("{}{}",
                    first as char,
                    middle.iter_elements().collect::<String>()
            )

        )
    ) {
        Err(e) => Err(e),
        Ok((remaining, label)) => {
            let impossible = ["af", "hl", "de", "bc", "ix", "iy", "ixl", "ixh"];
            if impossible.iter().any(|val| val == &label.to_lowercase()) {
                Err(::nom::Err::Error(error_position!(input, ErrorKind::OneOf)))
            } else {
                Ok((remaining, label))
            }
        }
    }
}

#[inline]
pub fn dec_u16(input: CompleteStr<'_>) -> IResult<CompleteStr<'_>, u16> {
    match is_a!(input, &b"0123456789"[..]) {
        Err(e) => Err(e),
        Ok((remaining, parsed)) => {
            // Do not parse more than 5 characters for a u16
            if parsed.input_len() > 5 {
                Err(::nom::Err::Error(error_position!(input, ErrorKind::OneOf)))
            } else {
                let mut res = 0u32;
                for e in parsed.iter_elements() {
                    let digit = e as char;
                    let value = digit.to_digit(10).unwrap_or(0) as u32;
                    res = value + (res * 10);
                }
                if res > u16::max_value() as u32 {
                    Err(::nom::Err::Error(error_position!(
                        input,
                        ErrorKind::Custom(0)
                    )))
                } else {
                    Ok((remaining, res as u16))
                }
            }
        }
    }
}

named!(
pub bin_u16<CompleteStr<'_>, u16>, do_parse!(
    alt!( tag_no_case!("0b") | tag_no_case!("%")) >>
    value: fold_many1!( 
        alt!(
            tag!("0") => {|_|{0 as u16}} |
            tag!("1") => {|_|{1 as u16}}
        ),
         0, 
        |mut acc: u16, item: u16| {
            acc *= 2;
            acc += item;
            acc
        }
    )

    >> (value)
));

#[inline]
pub fn hex_u16_inner(input: CompleteStr<'_>) -> IResult<CompleteStr<'_>, u16> {
    match is_a!(input, &b"0123456789abcdefABCDEF"[..]) {
        Err(e) => Err(e),
        Ok((remaining, parsed)) => {
            // Do not parse more than  characters for a u16
            if parsed.input_len() > 4 {
                Err(::nom::Err::Error(error_position!(input, ErrorKind::OneOf)))
            } else {
                let mut res = 0u32;
                for e in parsed.iter_elements() {
                    let digit = e as char;
                    let value = digit.to_digit(16).unwrap_or(0) as u32;
                    res = value + (res * 16);
                }
                if res > u16::max_value() as u32 {
                    Err(::nom::Err::Error(error_position!(input, ErrorKind::OneOf)))
                } else {
                    Ok((remaining, res as u16))
                }
            }
        }
    }
}

/*
/// Parse an ASM file an returns the stream of tokens.
pub fn parse_file(fname: String) -> Vec<Token> {
    let mut f = File::open(fnmae).expect(format!("{} not found", fname));
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect(format!("Something went wrong reading {}", fname));


    parse_binary_stream(fname.to_bytes())
}
*/

// XXX Code greatly inspired from https://github.com/Geal/nom/blob/master/tests/arithmetic_ast.rs

named!(
    parens<CompleteStr<'_>, Expr>,
    delimited!(
        delimited!(space0, tag!("("), space0),
        map!(map!(expr, Box::new), Expr::Paren),
        delimited!(space0, tag!(")"), space0)
    )
);

//TODO add stuff to manipulate any kind of data (value/label)
named!(pub factor< CompleteStr<'_>, Expr >, alt_complete!(
    // Manage functions
      delimited!(space0, parse_hi_or_lo, space0)
    | delimited!(space0, parse_duration, space0)
    | delimited!(space0, parse_assemble, space0)
    // manage values
    | map!(
        delimited!(
            space0, 
            alt_complete!(hex_u16 | bin_u16 | dec_u16), 
            space0
        ),
        |d:u16| {Expr::Value(d as i32)}
        )
    // manage $
   | map!(
        delimited!(
            space0, 
            tag!("$"), 
            space0
        ),
        |_x|{Expr::Label(String::from("$"))}
    )
    // manage labels
   | map!(
        delimited!(
            space0, 
            parse_label , 
            space0),
        Expr::Label
    )
  | parens
  
  )
);

fn fold_exprs(initial: Expr, remainder: Vec<(Oper, Expr)>) -> Expr {
    remainder.into_iter().fold(initial, |acc, pair| {
        let (oper, expr) = pair;
        match oper {
            Oper::Add => Expr::Add(Box::new(acc), Box::new(expr)),
            Oper::Sub => Expr::Sub(Box::new(acc), Box::new(expr)),
            Oper::Mul => Expr::Mul(Box::new(acc), Box::new(expr)),
            Oper::Div => Expr::Div(Box::new(acc), Box::new(expr)),
            Oper::Mod => Expr::Mod(Box::new(acc), Box::new(expr)),

            Oper::BinaryAnd => Expr::BinaryAnd(Box::new(acc), Box::new(expr)),
            Oper::BinaryOr => Expr::BinaryOr(Box::new(acc), Box::new(expr)),
            Oper::BinaryXor => Expr::BinaryXor(Box::new(acc), Box::new(expr)),

            Oper::Equal => Expr::Equal(Box::new(acc), Box::new(expr)),
            Oper::StrictlyGreater => Expr::StrictlyGreater(Box::new(acc), Box::new(expr)),
            Oper::StrictlyLower => Expr::StrictlyLower(Box::new(acc), Box::new(expr)),
            Oper::LowerOrEqual => Expr::LowerOrEqual(Box::new(acc), Box::new(expr)),
            Oper::GreaterOrEqual => Expr::GreaterOrEqual(Box::new(acc), Box::new(expr)),
        }
    })
}

named!(pub term< CompleteStr<'_>, Expr >, do_parse!(
    initial: factor >>
    remainder: many0!(
           alt_complete!(
             do_parse!(tag!("*") >> mul: factor >> (Oper::Mul, mul)) |
             do_parse!(tag!("%") >> _mod: factor >> (Oper::Mod, _mod)) |
             do_parse!(tag!("/") >> div: factor >> (Oper::Div, div))
           )
         ) >>
    (fold_exprs(initial, remainder))
));

named!(pub expr< CompleteStr<'_>, Expr >, do_parse!(
    initial: comp >>
    remainder: many0!(
        alt_complete!(
            do_parse!(tag!("<=") >> le: comp >> (Oper::LowerOrEqual, le) ) |
            do_parse!(tag!(">=") >> ge: comp >> (Oper::GreaterOrEqual, ge) ) |
            do_parse!(tag!("<") >> lt: comp >> (Oper::StrictlyLower, lt) ) |
            do_parse!(tag!(">") >> gt: comp >> (Oper::StrictlyGreater, gt) ) |
            do_parse!(tag!("==") >> eq: comp >> (Oper::Equal , eq) )  // TODO should be done even one step before
        )
    ) >>
    (fold_exprs(initial, remainder))
 )
);

named!(pub   parse_hi_or_lo <CompleteStr<'_>, Expr>, do_parse!(
        hi_or_lo: alt_complete!(
            value!(Function::Hi, tag_no_case!("HI")) |
            value!(Function::Lo, tag_no_case!("LO")) 
        ) >>
        many0!(space) >>
        tag!("(") >>
        many0!(space) >>
        exp: expr >>
        many0!(space) >>
        tag!(")") >>
        (
            match hi_or_lo {
                Function::Hi => Expr::High(Box::new(exp)),
                Function::Lo => Expr::Low(Box::new(exp)),
            }
        )
    )
);

named!(pub parse_duration <CompleteStr<'_>, Expr>, do_parse!(
    tag_no_case!("duration(") >>
    space0 >>
    token: parse_token >>
    space0 >>
    tag!(")") >>
    (
        Expr::Duration(Box::new(token))
    )
));

named!(pub parse_assemble <CompleteStr<'_>, Expr>, do_parse!(
    tag_no_case!("opcode(") >>
    space0 >>
    token: parse_token >>
    space0 >>
    tag!(")") >>
    (
        Expr::OpCode(Box::new(token))
    )
));

named!(pub comp<CompleteStr<'_>, Expr>, do_parse!(
    initial: term >>
    remainder: many0!(
           alt_complete!(
             do_parse!(tag!("+") >> add: term >> (Oper::Add, add)) |
             do_parse!(tag!("-") >> sub: term >> (Oper::Sub, sub)) |

             do_parse!(
                 alt!( 
                     terminated!(tag!("&"), not!(tag!("&"))) |
                     tag_no_case!("AND")
                 ) >> 
                 and: term >> 
                 (Oper::BinaryAnd, and)
            ) |

            do_parse!(
                 alt!( 
                     terminated!(tag!("|"), not!(tag!("|"))) |
                     tag_no_case!("OR")
                 ) >> 
                 and: term >> 
                 (Oper::BinaryAnd, and)
            ) |

             do_parse!(
                 alt!( 
                     terminated!(tag!("^"), not!(tag!("^"))) |
                     tag_no_case!("XOR")
                 ) >> 
                 and: term >> 
                 (Oper::BinaryAnd, and)
            ) 
           )
         ) >>
    (fold_exprs(initial, remainder))
    )
);

pub fn decode_parsing_error(orig: &str, e: ::nom::Err<CompleteStr<'_>>) -> String {
    use nom::InputLength;

    let mut error_string;

    if let ::nom::Err::Failure(::nom::simple_errors::Context::Code(
        remaining,
        ErrorKind::Custom(_),
    )) = e
    {
        let bytes = orig.as_bytes();
        let complete_size = orig.len();
        let remaining_size = remaining.input_len();
        let error_position = complete_size - remaining_size;
        let line_end = {
            let mut idx = error_position;
            while idx < complete_size && bytes[idx] != b'\n' {
                idx += 1;
            }
            idx
        };
        let line_start = {
            let mut idx = error_position;
            while idx > 0 && bytes[idx - 1] != b'\n' {
                idx -= 1;
            }
            idx
        };

        let line = &orig[line_start..line_end];
        let line_idx = orig[..(error_position)]
            .bytes()
            .filter(|b| *b == b'\n')
            .count(); // way too slow I guess
        let column_idx = error_position - line_start;
        let error_description = "Error because";
        let empty = iter::repeat(" ").take(column_idx).collect::<String>();
        error_string = format!(
            "{}:{}:{} {}\n{}\n{}^",
            "fname", line_idx, column_idx, error_description, line, empty
        );
    } else {
        error_string = String::from("Unknown error");
    }

    error_string
}
