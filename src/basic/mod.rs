pub mod parser;
pub mod tokens;

use parser::parse_basic_program;
use std::collections::BTreeMap;
use std::fmt;
use tokens::BasicToken;

#[derive(Debug, Clone, PartialEq)]
pub enum BasicProgramLineIdx {
    /// The basic line is indexed by its position in the listing
    Index(usize),
    /// The basic line is indexed by its real number
    Number(u16),
}

#[derive(Debug, Fail, PartialEq)]
pub enum BasicError {
    #[fail(display = "Line does not exist: {:?}", idx)]
    UnknownLine { idx: BasicProgramLineIdx },
}

/// Basic line of code representation
#[derive(Debug, Clone)]
pub struct BasicLine {
    /// Basic number of the line
    line_number: u16,
    /// Tokens of the basic line
    tokens: Vec<tokens::BasicToken>,
    /// Length of the line when we do not have to use the real lenght (ie, we play to hide lines)
    forced_length: Option<u16>,
}

impl fmt::Display for BasicLine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ", self.line_number)?;
        for token in self.tokens().iter() {
            write!(f, "{}", token)?;
        }
        Ok(())
    }
}

impl BasicLine {
    pub fn line_number(&self) -> u16 {
        self.line_number
    }

    /// Create a line with its content
    pub fn new(line_number: u16, tokens: &[tokens::BasicToken]) -> BasicLine {
        BasicLine {
            line_number,
            tokens: tokens.to_vec(),
            forced_length: None,
        }
    }

    pub fn add_length(&mut self, length: u16) {
        let current = self.forced_length();
        self.set_length(current + length);
    }

    pub fn set_length(&mut self, length: u16) {
        self.forced_length = Some(length);
    }
    /// Return the forced line length or the real line length if not specified
    pub fn forced_length(&self) -> u16 {
        match self.forced_length {
            Some(val) => val,
            None => (self.real_length() + 2 + 2 + 1) as _,
        }
    }

    /// Return the byte size taken by the tokens
    pub fn real_length(&self) -> u16 {
        self.tokens_as_bytes().len() as _
    }

    /// Returns the number of tokens
    pub fn len(&self) -> usize {
        self.tokens().len()
    }

    fn tokens_as_bytes(&self) -> Vec<u8> {
        self.tokens
            .iter()
            .flat_map(|t| t.as_bytes())
            .collect::<Vec<u8>>()
    }

    /// Returns the encoded line.
    /// - 2 bytes for data length -- Could be different than reallity when playing with hidden lines
    /// - 2 bytes for line number
    /// - n bytes for tokens
    /// - 1 bytes for end of line marker
    pub fn as_bytes(&self) -> Vec<u8> {
        let size = self.forced_length();

        let mut content = vec![
            (size % 256) as u8,
            (size / 256) as u8,
            (self.line_number % 256) as u8,
            (self.line_number / 256) as u8,
        ];
        content.extend_from_slice(&self.tokens_as_bytes());
        content.push(0);

        content
    }

    pub fn tokens(&self) -> &[BasicToken] {
        &self.tokens
    }
}

/// Encode a complete basic program
#[derive(Debug, Clone)]
pub struct BasicProgram {
    lines: Vec<BasicLine>,
}

impl fmt::Display for BasicProgram {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for line in self.lines.iter() {
            write!(f, "{}\n", line)?;
        }
        Ok(())
    }
}

impl BasicProgram {
    /// Create the program from a list of lines
    pub fn new(mut lines: Vec<BasicLine>) -> BasicProgram {
        BasicProgram { lines }
    }

    /// Create the program from a code to parse
    pub fn parse<S: AsRef<str>>(code: S) -> Result<BasicProgram, String> {
        match parse_basic_program(code.as_ref().into()) {
            Ok((res, prog)) => {
                if res.trim().len() != 0 {
                    Err(format!(
                        "Basic content has not been totally parsed: `{}`",
                        res
                    ))
                } else {
                    Ok(prog)
                }
            }
            Err(e) => Err(format!("Error while parsing the Basic content: {}", e)),
        }
    }

    /// Add a line to the program. If a line with the same number already exists, it is replaced
    pub fn add_line(&mut self, line: BasicLine) {
        self.lines.push(line);
    }

    /// Returns a mutable reference on the requested line if exists
    pub fn get_line_mut(&mut self, idx: BasicProgramLineIdx) -> Option<&mut BasicLine> {
        match self.line_idx_as_valid_index(idx) {
            Ok(BasicProgramLineIdx::Index(index)) => self.lines.get_mut(index),
            _ => None,
        }
    }

    /// Returns a reference on the requested line if exists
    pub fn get_line(&mut self, idx: BasicProgramLineIdx) -> Option<&BasicLine> {
        match self.line_idx_as_valid_index(idx) {
            Ok(BasicProgramLineIdx::Index(index)) => self.lines.get(index),
            _ => None,
        }
    }

    /// Returns true if the index corresponds to the very first line. False if it does not correspond or does not exists
    pub fn is_first_line(&self, idx: BasicProgramLineIdx) -> bool {
        match self.line_idx_as_valid_index(idx) {
            Ok(BasicProgramLineIdx::Index(0)) => true,
            _ => false,
        }
    }

    /// Returns the previous index of idx if it exists
    pub fn previous_idx(&self, idx: BasicProgramLineIdx) -> Option<BasicProgramLineIdx> {
        match self.line_idx_as_valid_index(idx) {
            Ok(BasicProgramLineIdx::Index(index)) => {
                if index != 0 {
                    Some(BasicProgramLineIdx::Index(index - 1))
                } else {
                    None
                }
            }
            Err(e) => None,
            _ => unreachable!(),
        }
    }

    /// Check if the program contains the requested line
    pub fn has_line(&self, idx: BasicProgramLineIdx) -> bool {
        self.line_idx_as_valid_index(idx).is_ok()
    }

    /// Return the line index in the index format if the line exists
    fn line_idx_as_valid_index(
        &self,
        idx: BasicProgramLineIdx,
    ) -> Result<BasicProgramLineIdx, BasicError> {
        match &idx {
            BasicProgramLineIdx::Index(index) => {
                if self.lines.len() <= *index {
                    Err(BasicError::UnknownLine { idx })
                } else {
                    Ok(idx)
                }
            }

            BasicProgramLineIdx::Number(number) => match self.get_index_of_line_number(*number) {
                Some(index) => Ok(BasicProgramLineIdx::Index(index)),
                None => Err(BasicError::UnknownLine { idx }),
            },
        }
    }

    /// For a given line number, returns the index in the list of lines
    fn get_index_of_line_number(&self, number: u16) -> Option<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter(move |(_index, line)| line.line_number == number)
            .map(|(index, _line)| index)
            .collect::<Vec<_>>()
            .first()
            .map(|&v| v)
    }

    /// https://cpcrulez.fr/applications_protect-protection_logiciel_n42_ACPC.htm
    pub fn hide_line(&mut self, idx: BasicProgramLineIdx) -> Result<(), BasicError> {
        if !self.has_line(idx.clone()) {
            Err(BasicError::UnknownLine { idx })
        } else if self.is_first_line(idx.clone()) {
            // Locomotive basic stat to list lines from 1
            self.lines[0].line_number = 0;
            Ok(())
        } else {
            match self.previous_idx(idx.clone()) {
                Some(previous_idx) => {
                    let current_length = self.get_line(idx.clone()).unwrap().real_length();
                    self.get_line_mut(previous_idx)
                        .unwrap()
                        .add_length(current_length + 1 + 2 + 2);
                    self.get_line_mut(idx).unwrap().set_length(0);
                    Ok(())
                }
                None => Err(BasicError::UnknownLine { idx }),
            }
        }
    }

    pub fn hide_lines(&mut self, lines: &Vec<u16>) -> Result<(), BasicError> {
        match lines.len() {
			0 => Ok(()),
			1 => self.hide_line(BasicProgramLineIdx::Number(lines[0])),
			_ => unimplemented!("The current version is only able to hide one line. I can still implement multiline version if needed")
		}
    }

    /// Generate the byte stream for the gien program
    pub fn as_bytes(&self) -> Vec<u8> {
        eprintln!("{:?}", self);
        dbg!(self);
        let mut bytes = self
            .lines
            .iter()
            .flat_map(|v| v.as_bytes())
            .collect::<Vec<u8>>();
        bytes.resize(bytes.len() + 3, 0);
        bytes
    }
}

#[cfg(test)]
mod test {
    use crate::basic::*;

    #[test]
    fn parse_complete() {
        let code = "10 call &0: call &0\n";
        BasicProgram::parse(code).expect("Unable to produce basic tokens");

        let code = "10 call &0: call &0";
        BasicProgram::parse(code).expect("Unable to produce basic tokens");

        let code = "10 ' blabla bla\n20 ' blab bla bal\n30 call &180";
        BasicProgram::parse(code).expect("Unable to produce basic tokens");
    }

    #[test]
    fn print_basic() {
        let code1 = "10 call &0: abs &0\n20 call 12\n30 print\n";
        let tokens = BasicProgram::parse(code1).unwrap();
        println!("{:?}", tokens.lines);
        let code2 = tokens.to_string();
        assert_eq!(code1.to_uppercase(), code2)
    }

    #[test]
    fn parse_correct() {
        let code = "10 CALL &1234";
        let prog = BasicProgram::parse(code).unwrap();
        let bytes = prog.as_bytes();
        let expected = vec![10, 0, 10, 0, 131, 32, 28, 0x34, 0x12, 0, 0, 0, 0];

        assert_eq!(bytes, expected);

        let code = "10 CALL &1234\n20 CALL &1234";
        let prog = BasicProgram::parse(code).unwrap();
        let bytes = prog.as_bytes();
        let expected = vec![
            10, 0, 10, 0, 131, 32, 28, 0x34, 0x12, 0, 10, 0, 20, 0, 131, 32, 28, 0x34, 0x12, 0, 0,
            0, 0,
        ];

        assert_eq!(bytes, expected);
    }

    #[test]
    fn hide1() {
        let code = "10 CALL &1234";
        let mut prog = BasicProgram::parse(code).unwrap();
        prog.hide_line(BasicProgramLineIdx::Number(10));
        let bytes = prog.as_bytes();
        let expected = vec![10, 0, 0, 0, 131, 32, 28, 0x34, 0x12, 0, 0, 0, 0];

        assert_eq!(bytes, expected);
    }

    #[test]
    fn indices() {
        let code = "10 CALL &1234\n20 CALL &1234";
        let mut prog = BasicProgram::parse(code).unwrap();

        assert_eq!(
            Ok(BasicProgramLineIdx::Index(0)),
            prog.line_idx_as_valid_index(BasicProgramLineIdx::Index(0))
        );
        assert_eq!(
            Ok(BasicProgramLineIdx::Index(1)),
            prog.line_idx_as_valid_index(BasicProgramLineIdx::Index(1))
        );
        assert_eq!(
            Err(BasicError::UnknownLine {
                idx: BasicProgramLineIdx::Index(2)
            }),
            prog.line_idx_as_valid_index(BasicProgramLineIdx::Index(2))
        );

        assert_eq!(
            Some(BasicProgramLineIdx::Index(0)),
            prog.previous_idx(BasicProgramLineIdx::Index(1))
        );
        assert_eq!(None, prog.previous_idx(BasicProgramLineIdx::Index(0)));

        assert!(prog.has_line(BasicProgramLineIdx::Number(10)));
        assert!(prog.has_line(BasicProgramLineIdx::Number(20)));
        assert!(!prog.has_line(BasicProgramLineIdx::Number(30)));
        assert!(prog.has_line(BasicProgramLineIdx::Index(0)));
        assert!(prog.has_line(BasicProgramLineIdx::Index(1)));
        assert!(!prog.has_line(BasicProgramLineIdx::Index(2)));
    }

    #[test]
    fn hide2() {
        let code = "10 CALL &1234\n20 CALL &1234";
        let mut prog = BasicProgram::parse(code).unwrap();
        assert!(prog.has_line(BasicProgramLineIdx::Number(20)));
        prog.hide_line(BasicProgramLineIdx::Number(20));
        let bytes = prog.as_bytes();
        let expected = vec![
            20, 0, 10, 0, 131, 32, 28, 0x34, 0x12, 0, 00, 0, 20, 0, 131, 32, 28, 0x34, 0x12, 0, 0,
            0, 0,
        ];

        assert_eq!(bytes, expected);
    }

}
