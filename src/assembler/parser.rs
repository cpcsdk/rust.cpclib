use nom::{Err, ErrorKind, IResult, space, space1, space0, line_ending};
use nom::types::{CompleteStr};
use nom::{multispace};
use nom::{InputLength, InputIter};
use crate::assembler::tokens::*;
    use std::iter;


    pub mod error_code {
        pub const ASSERT_MUST_BE_FOLLOWED_BY_AN_EXPRESSION:u32 = 128;
        pub const INVALID_ARGUMENT:u32 = 129;
    }

named! (
    pub parse_z80_code <CompleteStr<'_>, Vec<Token>>,
    do_parse!(
//        // Skip empty beginning
//       many0!(parse_empty_line) >>
//        opt!(line_ending) >>

        // Get optional code
        tmp:many0!(
                parse_z80_line
        ) >>

//        // Skip empty end
//        many0!(parse_empty_line) >>

        eof!() >>

        ({
            let mut res: Vec<Token> = Vec::new();
            for list in tmp {

         //       println!("Current list: {:?}", &list);
                res.append(&mut (list.clone()) );
            }
         //   println!("Opcodes: {:?}", &res);
            res
        })
        )
    );


/// For an unknwon reason, the parse_z80_code function fails when there is no comment...
/// This one is a workaround as still as the other is not fixed
pub fn parse_z80_str(code: &str) -> Result< (CompleteStr<'_>, Vec<Token>), Err<CompleteStr<'_>>> {
    let mut tokens = Vec::new();
    let mut rest = None;
    let src = "<str>";

    for (line_number, line) in code.split("\n").enumerate() {
        let res = parse_z80_line(CompleteStr(line));
        match res {
            Ok((res, local_tokens)) => {
                tokens.extend_from_slice(&local_tokens);
                rest = Some(res);
            },
            Err(e) => {
                let error_string = format!("Error at line {}: {}", line_number, line);
                eprintln!("{}:{} ({}) {}", src, line_number, line, error_string);
                return Err(e);
            }
        }


    }
    //TODO vérifier que c'est totallement assemblé
    Ok((rest.unwrap(), tokens))
}



// TODO find a way to not use the alternative stuff
named!(
    pub parse_z80_line<CompleteStr<'_>, Vec<Token>>,
        alt_complete!(
            parse_empty_line |
            parse_z80_line_label_only |
            parse_z80_line_complete
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
    parse_include <CompleteStr<'_>, Token>, do_parse!(
        tag_no_case!("INCLUDE") >>
        many1!(space) >>
        fname: delimited!(
            tag!("\""),
            parse_label, // TODO really write file stuff
            tag!("\"")
        ) >>
        (
            Token::Include(fname)
        )
    )
);


named!(
    parse_token <CompleteStr<'_>, Token>,
        alt_complete!(
            parse_add_or_adc |
            parse_djnz |
            parse_ld |
            parse_inc_dec |
            parse_out | parse_in |
            parse_jp_or_jr |
            parse_opcode_no_arg |
            parse_push_n_pop |
            parse_res_set |
            parse_ret
        )

    );


named!(
    parse_directive <CompleteStr<'_>, Token>,
        alt_complete!(
            parse_assert |
            parse_align |
            parse_org |
            parse_defs |
            parse_include |
            parse_db_or_dw |
            parse_protect
        )

    );


named!(
    pub parse_ld <CompleteStr<'_>, Token>, do_parse!(
        tag_no_case!("LD") >>
        space >>
        dst: return_error!(
            ErrorKind::Custom(error_code::INVALID_ARGUMENT),
            alt_complete!( parse_reg_address |
                           parse_register_sp |
                           parse_register16 |
                           parse_register8 |
                           parse_indexregister16 |
                           parse_indexregister8 |
                           parse_address)
        ) >>
        opt!(space) >>
        tag!(",") >>
        opt!(space) >>
        // src possibilities depend on dst
        src: return_error!(
            ErrorKind::Custom(error_code::INVALID_ARGUMENT),
            alt_complete!(
                cond_reduce!(dst.is_register16() | dst.is_indexregister16(), alt_complete!(parse_address | parse_expr)) |
                cond_reduce!(dst.is_register8(), alt_complete!(parse_indexregister_with_index | parse_hl_address | parse_address | parse_expr | parse_register8)) |
                cond_reduce!(dst.is_memory(), alt_complete!(parse_register16 | parse_register8 | parse_register_sp)) |
                cond_reduce!(dst.is_address_in_register16(), parse_register8)
            )
        )
         >>

        (Token::OpCode(Mnemonic::Ld, Some(dst), Some(src)))
        )
    );

named!(
    pub parse_res_set <CompleteStr<'_>, Token>, do_parse!(
        res_or_set: alt!(
            tag_no_case!("RES") => { |_|Mnemonic::Res} |
            tag_no_case!("SET") => { |_|Mnemonic::Set}
        ) >>
        many1!(space) >>
        bit: parse_expr >>
        many0!(space) >>
        tag!(",") >>
        many0!(space) >>
        dst: parse_register8 >> // TODO add other kinds
        (
            Token::OpCode(res_or_set, Some(bit), Some(dst))
        )
    )
);


named!(
  pub parse_db_or_dw <CompleteStr<'_>, Token>, do_parse!(
    is_db: alt!(
        tag_no_case!("DB") => {|_| {true}} |
        tag_no_case!("DW") => {|_| {false}}
    ) >>
    many1!(space) >>
    expr: expr_list >>
    ({
        if is_db {
            Token::Db(expr)
        }
        else {
            Token::Dw(expr)
        }
    })
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
            expr
            )
    );

named!(
    pub parse_assert <CompleteStr<'_>, Token>, do_parse!(
        tag_no_case!("ASSERT") >>
        many1!(space) >>
        expr: return_error!(
            ErrorKind::Custom(error_code::ASSERT_MUST_BE_FOLLOWED_BY_AN_EXPRESSION),
            expr
        ) >>
        (
            Token::Assert(expr)
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
            Token::Align(expr)
        )
    )
);


named!(
    pub parse_protect <CompleteStr<'_>, Token>, do_parse!(
        tag_no_case!("PROTECT") >>
        many1!(space) >>
        start: expr >>
        many0!(space) >>
        tag!(",") >>
        many0!(space) >>
        end: expr >>
        (
            Token::Protect(start, end)
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
            value!( DataAccess::Register8(Register8::A), tag_no_case!("A")) |
            value!( DataAccess::Register16(Register16::Hl), tag_no_case!("HL")) |
            value!( DataAccess::IndexRegister16(IndexRegister16::Ix), tag_no_case!("IX")) |
            value!( DataAccess::IndexRegister16(IndexRegister16::Iy), tag_no_case!("IY"))
        ) >>

        opt!(space) >>
        tag!(",") >>
        opt!(space) >>

        second: alt_complete!(
            cond_reduce!(first.is_register8(), alt_complete!(parse_register8 | parse_hl_address | parse_indexregister_with_index | parse_expr)) | // Case for A
            cond_reduce!(first.is_register16(), alt_complete!(parse_register16 | parse_register_sp)) | // Case for HL XXX AF is accepted whereas it is not the case in real life
            cond_reduce!(first.is_indexregister16(), alt_complete!(
                    value!(DataAccess::Register16(Register16::De), tag_no_case!("DE")) |
                    value!(DataAccess::Register16(Register16::Bc), tag_no_case!("BC")) |
                    value!(DataAccess::Register16(Register16::Sp), tag_no_case!("SP")) |
                    value!(DataAccess::IndexRegister16(IndexRegister16::Ix), tag_no_case!("IX")) |
                    value!(DataAccess::IndexRegister16(IndexRegister16::Iy), tag_no_case!("IY"))
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


named!(
    parse_jp_or_jr <CompleteStr<'_>, Token>, do_parse!(
        jp_or_jr: alt_complete!(
            value!(Mnemonic::Jp, tag_no_case!("JP")) |
            value!(Mnemonic::Jr, tag_no_case!("JR"))
            ) >>
        space >>
        flag_test:opt!(terminated!(parse_flag_test, delimited!(opt!(multispace), tag!(","), opt!(multispace)) )) >>
        dst: expr  >>
        ({

            let flag_test = if flag_test.is_some() {
                Some(DataAccess::FlagTest(flag_test.unwrap()))
            }
            else {
                None
            };
            Token::OpCode(jp_or_jr, flag_test, Some(DataAccess::Expression(dst)))
        })
        )
    );

named!(
    parse_flag_test <CompleteStr<'_>, FlagTest>,
        alt_complete!(
            value!(FlagTest::NZ, tag_no_case!("NZ")) |
            value!(FlagTest::Z, tag_no_case!("Z"))|
            value!(FlagTest::NC, tag_no_case!("NC"))|
            value!(FlagTest::C, tag_no_case!("C"))|
            value!(FlagTest::PO, tag_no_case!("PO")) |
            value!(FlagTest::PE, tag_no_case!("PE")) |
            value!(FlagTest::P, tag_no_case!("P")) |
            value!(FlagTest::M, tag_no_case!("M"))
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
    parse_register16 <CompleteStr<'_>, DataAccess>, do_parse!(
        reg: alt_complete!(
            tag_no_case!("AF") => { |_| Register16::Af} |
            tag_no_case!("HL") => { |_| Register16::Hl} |
            tag_no_case!("DE") => { |_| Register16::De} |
            tag_no_case!("BC") => { |_| Register16::Bc}
            )
        >>
        (DataAccess::Register16(reg))
    )
);


named!(
    parse_register8 <CompleteStr<'_>, DataAccess>, do_parse!(
        reg: alt_complete!(
            tag_no_case!("A") => { |_| Register8::A} |
            tag_no_case!("H") => { |_| Register8::H} |
            tag_no_case!("D") => { |_| Register8::D} |
            tag_no_case!("B") => { |_| Register8::B} |

            tag_no_case!("L") => { |_| Register8::L} |
            tag_no_case!("E") => { |_| Register8::E} |
            tag_no_case!("C") => { |_| Register8::C}
            )
        >>
        (DataAccess::Register8(reg))
    )
);

named!(
    parse_indexregister8 <CompleteStr<'_>, DataAccess>, do_parse!(
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
    parse_indexregister16 <CompleteStr<'_>, DataAccess>, do_parse!(
        reg: alt_complete!(
            tag_no_case!("IX") => { |_| IndexRegister16::Ix} |
            tag_no_case!("IY") => { |_| IndexRegister16::Iy}
            )
        >>
        (DataAccess::IndexRegister16(reg))
    )
);




/// Parse the use of an indexed register as (IX + 5)
named!(
    parse_indexregister_with_index <CompleteStr<'_>, DataAccess>, do_parse!(
            tag!("(") >>
            many0!(space) >>

            reg: parse_indexregister16 >>

            many0!(space) >>

            op: alt_complete!(
                value!(Oper::Add, tag!("+")) |
                value!(Oper::Sub, tag!("-"))
            ) >>

            many0!(space) >>

            expr: expr >>

            many0!(space) >>
            tag!(")")
        >>
        (DataAccess::IndexRegister16WithIndex(reg.get_indexregister16().unwrap(), op, expr))
    )
);


named!(
    parse_register_sp <CompleteStr<'_>, DataAccess>, do_parse!(
        tag_no_case!("SP") >>
        (DataAccess::Register16(Register16::Sp))
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
        (Token::Org(val))
    )
);


named!(
    pub parse_defs <CompleteStr<'_>, Token>, do_parse!(
        tag_no_case!("DEFS") >>
        space >>
        val: expr >>
        (Token::Defs(val))
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
        tag_no_case!("RRA") => {|_| Mnemonic::Rra }
      )
    >> (Token::OpCode(res, None, None) )
    )
);


named!(
    pub parse_value <CompleteStr<'_>, Expr>, do_parse!(
        val: alt_complete!(hex_u16 | dec_u16) >>
        (Expr::Value(val as i32))
        )
    );


    // TODO foribd label with the same name as a register or mnemonic
named!(
    pub parse_label <CompleteStr<'_>, String>, do_parse!(
        first: one_of!("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.") >> // XXX The inclusion of . is probably problematic
        middle: is_a!("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_.") >>
        (
            format!("{}{}",
                    first as char,
                    middle.iter_elements().collect::<String>()
                   )
        )
        )
    );

named!(
    pub hex_u16 <CompleteStr<'_>, u16>, do_parse!(
        tag_no_case!( "0x") >>
        val: hex_u16_inner >>
        (val)
        )
    );


/// Parse a comment that start by `;` and ends at the end of the line.
named!( comment<CompleteStr<'_>, Token>,
        map!(
            preceded!(
                tag!( ";" ),
                take_till!( |ch| ch == '\n' )
            ),
            |string| {Token::Comment(string.iter_elements().collect::<String>())}
        )
);

// Usefull later for db
named!(string_between_quotes<CompleteStr<'_>, CompleteStr<'_>>, delimited!(char!('\"'), is_not!("\""), char!('\"')));

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
                    Err(::nom::Err::Error(error_position!(input, ErrorKind::Custom(0))))
                } else {
                    Ok((remaining, res as u16))
                }
            }
        }
    }
}

named!(
pub bin_u16<CompleteStr<'_>, u16>, do_parse!(
    tag_no_case!("0b") >>
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

/// Produce the stream of tokens. In case of error, return an explanatory string
pub fn parse_str(code: &str) -> Result<Vec<Token>, String> {
    match parse_z80_str(code) {
        Err(e) => Err(format!("Error while assembling: {:?}", e)),
        Ok((_, parsed)) => {
            Ok(parsed)
        }
    }
}










// XXX Code greatly inspired from https://github.com/Geal/nom/blob/master/tests/arithmetic_ast.rs

named!(parens< CompleteStr<'_>, Expr >, delimited!(
    delimited!(opt!(multispace), tag!("("), opt!(multispace)),
    map!(map!(expr, Box::new), Expr::Paren),
    delimited!(opt!(multispace), tag!(")"), opt!(multispace))
  )
);

//TODO add stuff to manipulate any kind of data (value/label)
named!(pub factor< CompleteStr<'_>, Expr >, alt_complete!(
    // Manahge functions
    parse_hi_or_lo
    // manage values
    | map!(
        delimited!(opt!(multispace), alt_complete!(hex_u16 | dec_u16 | bin_u16), opt!(multispace)),
        |d:u16| {Expr::Value(d as i32)}
        )
    // manage $
   | map!(
        delimited!(opt!(multispace), tag!("$"), opt!(multispace)),
        |_x|{Expr::Label(String::from("$"))}
    )
    // manage labels
   | map!(
        delimited!(opt!(multispace), parse_label , opt!(multispace)),
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



named!(pub comp<CompleteStr<'_>, Expr>, do_parse!(
    initial: term >>
    remainder: many0!(
           alt_complete!(
             do_parse!(tag!("+") >> add: term >> (Oper::Add, add)) |
             do_parse!(tag!("-") >> sub: term >> (Oper::Sub, sub))
           )
         ) >>
    (fold_exprs(initial, remainder))
    )
);




pub fn decode_parsing_error(orig: &str, e: ::nom::Err<CompleteStr<'_>>) -> String {

    use nom::InputLength;

    let mut error_string;

    if let ::nom::Err::Failure(::nom::simple_errors::Context::Code(remaining, ErrorKind::Custom(_))) = e {

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
            while idx >0 &&  bytes[idx-1] != b'\n' {
                idx -= 1;
            }
            idx
        };


        let line = &orig[line_start..line_end];
        let line_idx = orig[..(error_position)].bytes().filter(|b| *b == b'\n').count(); // way too slow I guess
        let column_idx = error_position - line_start;
        let error_description = "Error because";
        let empty = iter::repeat(" ").take(column_idx).collect::<String>();
        error_string = format!("{}:{}:{} {}\n{}\n{}^", "fname", line_idx, column_idx, error_description, line, empty);
    }
    else {
        error_string = String::from("Unknown error");
    }

    error_string


}
