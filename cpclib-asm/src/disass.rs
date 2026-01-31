use crate::preamble::*;
use cpclib_tokens::opcode_table::{
    TABINSTRED, TABINSTR, TABINSTRCB, TABINSTRDD, TABINSTRDDCB, TABINSTRFD, TABINSTRFDCB,
};

/// Generate a listing from the list of bytes. An error is generated if it is impossible to disassemble the flux
/// TODO really implement it
pub fn disassemble<'a>(mut bytes: &'a [u8]) -> Listing {
    let mut reverse_tokens = Vec::new();

    // Generate a listing that contains the current token followed by tokens obtained from remaining bytes
    let mut continue_disassembling = |token: Token, bytes: &'a [u8]| {
        reverse_tokens.push(token);
        bytes
    };

    while !bytes.is_empty() {
        bytes = match bytes {
            [] => unreachable!(),

            // Current mnemonic is nop
            [0, rest @ ..] => continue_disassembling(nop(), rest),

            [prefix, 0xCB, param, opcode, rest @ ..] if *prefix == 0xFD || *prefix == 0xDD => {
                let token = disassemble_with_one_argument(
                    *opcode,
                    *param,
                    if *prefix == 0xFD {
                        &TABINSTRFDCB
                    }
                    else {
                        &TABINSTRDDCB
                    }
                )
                .unwrap_or_else(|_| defb_elements(&[*prefix, 0xCB, *param, *opcode]));
                continue_disassembling(token, rest)
            },

            [prefix, opcode, rest @ ..]
                if *prefix == 0xCB || *prefix == 0xED || *prefix == 0xDD || *prefix == 0xFD =>
            {
                let (token, rest) = disassemble_with_potential_argument(
                    *opcode,
                    match prefix {
                        0xCB => &TABINSTRCB,
                        0xED => &TABINSTRED,
                        0xDD => &TABINSTRDD,
                        0xFD => &TABINSTRFD,
                        _ => unreachable!()
                    },
                    rest
                )
                .unwrap_or_else(|_| (defb_elements(&[*prefix, *opcode]), rest));
                continue_disassembling(token, rest)
            },

            [opcode, rest @ ..] => {
                let (token, rest) = disassemble_with_potential_argument(*opcode, &TABINSTR, rest)
                    .unwrap_or_else(|_| (defb(*opcode), rest));
                continue_disassembling(token, rest)
            }
        }
    }

    // reverse_tokens.reverse();
    reverse_tokens.into()
}

/// Manage the disassembling of the current instraction. However this instruction may need an argument.
/// For this reason the byte stream is provided to collect this argument if needed
pub fn disassemble_with_potential_argument<'stream>(
    opcode: u8,
    lut: &[&'static str; 256],
    bytes: &'stream [u8]
) -> Result<(Token, &'stream [u8]), String> {
    let representation: &'static str = lut[opcode as usize];

    // get the first argument if any
    let (representation, bytes) = if representation.contains("nnnn") {
        let word = bytes[0] as u16 + 256 * (bytes[1] as u16);
        let representation = representation.replacen("nnnn", &format!("{word:#03x}"), 1);
        (representation, &bytes[2..])
    }
    else if representation.contains("nn") {
        let byte = bytes[0] as i8;
        let representation =
            if representation.starts_with("DJNZ") || representation.starts_with("JR") {
                let byte = byte as i16 + 2;
                if byte == 0 {
                    representation.replacen("nn", "$", 1)
                }
                else if byte < 0 {
                    representation.replacen("nn", &format!("-{}", byte.abs()), 1)
                }
                else {
                    representation.replacen("nn", &format!("+{}", byte.abs()), 1)
                }
            }
            else {
                representation.replacen("nn", &format!("{byte:#01x}"), 1)
            };

        (representation.to_owned(), &bytes[1..])
    }
    else {
        (representation.to_owned(), bytes)
    };

    // get the second argument if any
    let (representation, bytes) = if representation.contains("nn") {
        let byte = bytes[0];
        let representation = representation.replacen("nn", &format!("{byte:#01x}"), 1);
        (representation, &bytes[1..])
    }
    else {
        (representation, bytes)
    };

    Ok((string_to_token(&representation)?, bytes))
}

/// The 8bits argument has already been read
pub fn disassemble_with_one_argument(
    opcode: u8,
    argument: u8,
    lut: &[&'static str; 256]
) -> Result<Token, String> {
    let representation: &'static str = lut[opcode as usize];
    let representation = representation.replacen("nn", &format!("{argument:#01x}"), 1);
    string_to_token(&representation)
}

/// No argument is expected
pub fn disassemble_without_argument(
    opcode: u8,
    lut: &[&'static str; 256]
) -> Result<Token, String> {
    let representation: &'static str = lut[opcode as usize];
    string_to_token(representation)
}

/// Thje method never fails now => it generate a db opcode
pub fn string_to_token(representation: &str) -> Result<Token, String> {
    if representation.is_empty() {
        Err("Empty opcode".to_string())
    }
    else {
        Token::parse_token(representation)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn disass_from_bytes() {
        assert_eq!("PUSH HL", disassemble(&[0xE5]).to_string().trim());
        assert_eq!("RES 0x3, E", disassemble(&[0xCB, 0x9B]).to_string().trim());
        assert_eq!(
            "SBC HL, DE",
            disassemble(&[0xED, 0b01010010]).to_string().trim()
        );

        assert_eq!(
            "RLC (IX + 0x1)",
            disassemble(&[0xDD, 0xCB, 01, 06]).to_string().trim()
        );
        assert_eq!(
            "RLC (IX + 0x1), B",
            disassemble(&[0xDD, 0xCB, 01, 00]).to_string().trim()
        );
        assert_eq!(
            "RLC (IY + 0x2), C",
            disassemble(&[0xFD, 0xCB, 02, 01]).to_string().trim()
        );
    }

    #[test]
    fn disass_instruction_with_arg() {
        assert_eq!(
            "CALL NZ, 0x123",
            disassemble(&[0xC4, 0x23, 0x01]).to_string().trim()
        );
        assert_eq!(
            "LD IX, (0x4321)",
            disassemble(&[0xDD, 0x2A, 0x21, 0x43]).to_string().trim()
        );
        assert_eq!(
            "LD (IX + 0x21), 0x43",
            disassemble(&[0xDD, 0x36, 0x21, 0x43]).to_string().trim()
        );
        assert_eq!(
            "BIT 0x6, (IX + 0x1)",
            disassemble(&[0xDD, 0xCB, 0x01, 0x76]).to_string().trim()
        );
    }
    // #[test]
    // fn disass_unknwon_opcode(){
    // assert!(disassemble(&[0xfd, 0x00]).is_err());
    // }
    //   #[test] // disable because incorrect test due to the several possible views of instructions
    fn disass_check_representation_equality() {
        disass_for_table_and_prefix(&TABINSTR, &[]);
        disass_for_table_and_prefix(&TABINSTRCB, &[0xCB]);
        disass_for_table_and_prefix(&TABINSTRDD, &[0xDD]);
        disass_for_table_and_prefix(&TABINSTRED, &[0xED]);
        disass_for_table_and_prefix(&TABINSTRFD, &[0xFD]);

        disass_for_double_prefix(&TABINSTRFDCB, 0xFD, 0xCB);
        disass_for_double_prefix(&TABINSTRDDCB, 0xDD, 0xCB);
    }

    fn disass_for_double_prefix(tab: &[&'static str; 256], first: u8, second: u8) {
        for code in 0..=255 {
            let repr = tab[code as usize];

            if repr.is_empty() {
                continue;
            }

            let repr = repr.replacen("nn", "0x12", 1);
            let expected_bytes = [first, second, 0x12, code];

            let obtained = disassemble(&expected_bytes);

            println!("{:?},{:?}, {:?}", repr, expected_bytes, obtained);
            assert_eq!(
                repr.replace(" ", "")
                    .replace("0x", "")
                    .replace("00", "0")
                    .replace("08", "8")
                    .to_uppercase(),
                obtained
                    .to_string()
                    .trim()
                    .replace(" ", "")
                    .replace("0x", "")
                    .to_uppercase()
            );

            let mut env = Env::default();
            if let Token::OpCode(mnemonic, arg1, arg2, arg3) = &obtained.listing()[0] {
                let obtained_bytes = env
                    .assemble_opcode_impl(*mnemonic, arg1, arg2, arg3)
                    .unwrap();
                assert_eq!(&expected_bytes[..], &obtained_bytes[..]);
            }
            else {
                println!("ERROR, this is not a Token {:?}", obtained);
                assert!(false);
            }
        }
    }
    fn disass_for_table_and_prefix(tab: &[&'static str; 256], prefix: &[u8]) {
        // Concatenate list of list of bytes
        let merge = |list: &[&[u8]]| -> Vec<u8> {
            list.iter()
                .flat_map(|&bytes| bytes.iter())
                .copied()
                .collect()
        };

        for code in 0..=255 {
            let repr = tab[code as usize];

            if repr.is_empty() {
                continue;
            }

            println!("0x{:x} : {}", code, repr);

            // TODO add test for opcodes with operandes
            let (expected, bytes) = if repr.contains("nnnn") {
                (
                    repr.replace("nnnn", "0x3412"),
                    merge(&[prefix, &[code], &[0x12, 0x34]])
                )
            }
            else if repr.contains("nn") {
                let repr = repr.replacen("nn", "0x12", 1);
                let (repr, bytes) = if repr.contains("nn") {
                    (repr.replace("nn", "0x34"), [0x12, 0x34].to_vec())
                }
                else {
                    (repr, [0x12].to_vec())
                };
                (repr, merge(&[prefix, &[code], &bytes]))
            }
            else {
                (repr.to_owned(), merge(&[prefix, &[code]]))
            };

            let obtained = disassemble(&bytes);

            // check if disassembling provides the right value
            // alter strings in order to be able to compare them
            if !expected.contains("RST") && !expected.contains("DJNZ") && !expected.contains("JR") {
                assert_eq!(
                    expected
                        .replace(" ", "")
                        .replace("0x", "")
                        .replace("00", "0")
                        .replace("08", "8")
                        .to_uppercase(),
                    obtained
                        .to_string()
                        .trim()
                        .replace(" ", "")
                        .replace("0x", "")
                        .to_uppercase()
                );
            }

            return; // the following code is deactivated as several instructions can be assembled with several bytecodes
                    // check if it is possible to assemble it
        /*            
            let mut env = Env::default();
            if let Token::OpCode(mnemonic, arg1, arg2, arg3) = &obtained.listing()[0] {
                // relative addresses are not properly managed
                if !(mnemonic.is_djnz() || mnemonic.is_jr()) {
                    let obtained_bytes =
                        env.assemble_opcode_impl(*mnemonic, arg1, arg2, arg3).unwrap();
                    assert_eq!(
                        &bytes[..],
                        &obtained_bytes[..],
                        "{:?}",
                        &obtained.listing()[0]
                    );
                }
            }
            else {
                println!("ERROR, this is not a Token {:?}", obtained);
                assert!(false);
            }
            */
        }
    }
}
