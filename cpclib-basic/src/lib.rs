pub mod binary_parser;
/// Paring related functions for basic.
pub mod string_parser;
/// Basic token encoding.
pub mod tokens;

use std::fmt::{self};

use cpclib_common::winnow::ascii::space0;
use cpclib_common::winnow::Parser;
use cpclib_sna::Snapshot;
use string_parser::parse_basic_program;
use thiserror::Error;
use tokens::BasicToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Basic line index represtation. Can be by line number of position in the list
pub enum BasicProgramLineIdx {
    /// The basic line is indexed by its position in the listing
    Index(usize),
    /// The basic line is indexed by its real number
    Number(u16)
}

#[derive(Debug, Error, PartialEq, Eq, Clone)]
#[allow(missing_docs)]
pub enum BasicError {
    #[error("Line does not exist: {:?}", idx)]
    UnknownLine { idx: BasicProgramLineIdx },
    #[error("{}", msg)]
    ParseError { msg: String },
    #[error("Exponent Overflow")]
    ExponentOverflow
}

/// Basic line of code representation
#[derive(Debug, Clone)]
pub struct BasicLine {
    /// Basic number of the line
    line_number: u16,
    /// Tokens of the basic line
    tokens: Vec<BasicToken>,
    /// Length of the line when we do not have to use the real length (ie, we play to hide lines)
    forced_length: Option<u16>
}

impl fmt::Display for BasicLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ", self.line_number)?;
        for token in self.tokens().iter() {
            write!(f, "{token}")?;
        }
        Ok(())
    }
}

#[allow(missing_docs)]
impl BasicLine {
    pub fn line_number(&self) -> u16 {
        self.line_number
    }

    /// Create a line with its content
    pub fn new(line_number: u16, tokens: &[BasicToken]) -> Self {
        Self {
            line_number,
            tokens: tokens.to_vec(),
            forced_length: None
        }
    }

    pub fn add_length(&mut self, length: u16) {
        let current = self.expected_length();
        self.force_length(current + length);
    }

    pub fn force_length(&mut self, length: u16) {
        self.forced_length = Some(length);
    }

    /// Return the forced line length or the real line length if not specified
    pub fn expected_length(&self) -> u16 {
        match self.forced_length {
            Some(val) => val,
            None => self.real_complete_length()
        }
    }

    /// Return the byte size taken by the tokens
    pub fn real_length(&self) -> u16 {
        self.tokens_as_bytes().len() as _
    }

    pub fn real_complete_length(&self) -> u16 {
        self.real_length() + 2 + 2 + 1
    }

    /// Returns the number of tokens
    pub fn len(&self) -> usize {
        self.tokens().len()
    }

    /// Verify if there are tokens
    pub fn is_empty(&self) -> bool {
        self.tokens().is_empty()
    }

    pub fn tokens_as_bytes(&self) -> Vec<u8> {
        self.tokens
            .iter()
            .flat_map(BasicToken::as_bytes)
            .collect::<Vec<u8>>()
    }

    /// Returns the encoded line.
    /// - 2 bytes for data length -- Could be different than reallity when playing with hidden lines
    /// - 2 bytes for line number
    /// - n bytes for tokens
    /// - 1 bytes for end of line marker
    pub fn as_bytes(&self) -> Vec<u8> {
        let size = self.expected_length();

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
    /// The ensemble of lines of the basic program
    lines: Vec<BasicLine>
}

impl fmt::Display for BasicProgram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in &self.lines {
            writeln!(f, "{line}")?;
        }
        Ok(())
    }
}

#[allow(missing_docs)]
impl BasicProgram {
    /// Create the program from a list of lines
    pub fn new(lines: Vec<BasicLine>) -> Self {
        Self { lines }
    }

    /// Create the program from a code to parse
    pub fn parse<S: AsRef<str>>(code: S) -> Result<Self, BasicError> {
        let input = code.as_ref();
        match (parse_basic_program, space0).parse(input) {
            Ok((prog, _)) => Ok(prog),
            Err(e) => {
                Err(BasicError::ParseError {
                    msg: format!("Error while parsing the Basic content: {e}")
                })
            },

            _ => unreachable!()
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
            _ => None
        }
    }

    /// Returns a reference on the requested line if exists
    pub fn get_line(&mut self, idx: BasicProgramLineIdx) -> Option<&BasicLine> {
        match self.line_idx_as_valid_index(idx) {
            Ok(BasicProgramLineIdx::Index(index)) => self.lines.get(index),
            _ => None
        }
    }

    /// Returns true if the index corresponds to the very first line. False if it does not correspond or does not exists
    pub fn is_first_line(&self, idx: BasicProgramLineIdx) -> bool {
        match self.line_idx_as_valid_index(idx) {
            Ok(BasicProgramLineIdx::Index(0)) => true,
            _ => false
        }
    }

    /// Returns the previous index of idx if it exists
    pub fn previous_idx(&self, idx: BasicProgramLineIdx) -> Option<BasicProgramLineIdx> {
        match self.line_idx_as_valid_index(idx) {
            Ok(BasicProgramLineIdx::Index(index)) => {
                if index == 0 {
                    None
                }
                else {
                    Some(BasicProgramLineIdx::Index(index - 1))
                }
            },
            Err(_e) => None,
            _ => unreachable!()
        }
    }

    /// Check if the program contains the requested line
    pub fn has_line(&self, idx: BasicProgramLineIdx) -> bool {
        self.line_idx_as_valid_index(idx).is_ok()
    }

    /// Return the line index in the index format if the line exists
    fn line_idx_as_valid_index(
        &self,
        idx: BasicProgramLineIdx
    ) -> Result<BasicProgramLineIdx, BasicError> {
        match &idx {
            BasicProgramLineIdx::Index(index) => {
                if self.lines.len() <= *index {
                    Err(BasicError::UnknownLine { idx })
                }
                else {
                    Ok(idx)
                }
            },

            BasicProgramLineIdx::Number(number) => {
                match self.get_index_of_line_number(*number) {
                    Some(index) => Ok(BasicProgramLineIdx::Index(index)),
                    None => Err(BasicError::UnknownLine { idx })
                }
            },
        }
    }

    /// For a given line number, returns the index in the list of lines
    fn get_index_of_line_number(&self, number: u16) -> Option<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(move |(index, line)| {
                if line.line_number == number {
                    Some(index)
                }
                else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .first()
            .cloned()
    }

    /// https://cpcrulez.fr/applications_protect-protection_logiciel_n42_ACPC.htm
    /// 64nops2
    pub fn hide_line(&mut self, current_idx: BasicProgramLineIdx) -> Result<(), BasicError> {
        if !self.has_line(current_idx) {
            Err(BasicError::UnknownLine { idx: current_idx })
        }
        else if self.is_first_line(current_idx) {
            // Locomotive basic stat to list lines from 1
            self.lines[0].line_number = 0;
            Ok(())
        }
        else {
            match self.previous_idx(current_idx) {
                Some(previous_idx) => {
                    let current_length = self.get_line(current_idx).unwrap().real_complete_length(); // TODO handle the case where they are multiple hidden
                    self.get_line_mut(previous_idx)
                        .unwrap()
                        .add_length(current_length);
                    self.get_line_mut(current_idx).unwrap().force_length(0);
                    Ok(())
                },
                None => Err(BasicError::UnknownLine { idx: current_idx })
            }
        }
    }

    pub fn hide_lines(&mut self, lines: &[u16]) -> Result<(), BasicError> {
        match lines.len() {
			0 => Ok(()),
			1 => self.hide_line(BasicProgramLineIdx::Number(lines[0])),
			_ => unimplemented!("The current version is only able to hide one line. I can still implement multiline version if needed")
		}
    }

    /// Generate the byte stream for the given program
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = self
            .lines
            .iter()
            .flat_map(BasicLine::as_bytes)
            .collect::<Vec<u8>>();
        bytes.resize(bytes.len() + 3, 0);
        bytes
    }

    pub fn as_sna(&self) -> Result<Snapshot, String> {
        let bytes = self.as_bytes();
        let mut sna = Snapshot::new_6128()?;
        sna.unwrap_memory_chunks();
        sna.add_data(&bytes, 0x170)
            .map_err(|e| format!("{e:?}"))?;
        Ok(sna)
    }
}

#[allow(clippy::let_unit_value)]
#[allow(clippy::shadow_unrelated)]
#[cfg(test)]
pub mod test {

    use super::*;

    #[test]
    fn parse_complete() {
        let code = "10 call &0: call &0\n";
        BasicProgram::parse(code).expect("Unable to produce basic tokens");

        let code1 = "10 call &0: call &0";
        BasicProgram::parse(code1).expect("Unable to produce basic tokens");

        let code2 = "10 ' blabla bla\n20 ' blab bla bal\n30 call &180";
        BasicProgram::parse(code2).expect("Unable to produce basic tokens");
    }

    #[test]
    fn parse_correct() {
        let code = "10 CALL &1234";
        let prog = BasicProgram::parse(code).unwrap();
        let bytes = prog.as_bytes();
        let expected = [10, 0, 10, 0, 131, 32, 28, 0x34, 0x12, 0, 0, 0, 0];

        assert_eq!(&bytes, &expected);

        let code = "10 CALL &1234\n20 CALL &1234";
        let prog = BasicProgram::parse(code).unwrap();
        let bytes = prog.as_bytes();
        let expected = [
            10, 0, 10, 0, 131, 32, 28, 0x34, 0x12, 0, 10, 0, 20, 0, 131, 32, 28, 0x34, 0x12, 0, 0,
            0, 0
        ];

        assert_eq!(&bytes, &expected);
    }

    #[test]
    fn hide1() {
        let code = "10 CALL &1234";
        let mut prog = BasicProgram::parse(code).unwrap();
        prog.hide_line(BasicProgramLineIdx::Number(10)).unwrap();
        let bytes = prog.as_bytes();
        let expected = vec![10, 0, 0, 0, 131, 32, 28, 0x34, 0x12, 0, 0, 0, 0];

        assert_eq!(bytes, expected);
    }

    #[test]
    fn indices() {
        let code = "10 CALL &1234\n20 CALL &1234";
        let prog = BasicProgram::parse(code).unwrap();

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
        prog.hide_line(BasicProgramLineIdx::Number(20)).unwrap();
        let bytes = prog.as_bytes();
        let expected = vec![
            20, 0, 10, 0, 131, 32, 28, 0x34, 0x12, 0, 00, 0, 20, 0, 131, 32, 28, 0x34, 0x12, 0, 0,
            0, 0,
        ];

        assert_eq!(bytes, expected);
    }
}
