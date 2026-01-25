/// This file encodes the CatArt basic commands as the could be handled in a basic program

use std::{fmt::Display};

use crate::char_command::{CharCommand, CharCommandList};

/// Represents an argument to the PRINT command.
#[derive(Clone, Debug, PartialEq)]
pub enum PrintArgument {
    /// A string of bytes to print.
    String(Vec<u8>),
    /// A single character code (CHR$).
    ChrDollar(u8),
    /// A composite argument, containing multiple PrintArguments.
    Composite(Vec<PrintArgument>),
}

impl PrintArgument {
    /// Constructs a PrintArgument::String from a byte vector.
    pub fn string(data: Vec<u8>) -> Self {
        PrintArgument::String(data)
    }

    /// Constructs a PrintArgument::ChrDollar from a character code.
    pub fn chr_dollar(char_code: u8) -> Self {
        PrintArgument::ChrDollar(char_code)
    }

    /// Constructs a PrintArgument::Composite from a vector of PrintArguments.
    pub fn composite(args: Vec<PrintArgument>) -> Self {
        PrintArgument::Composite(args)
    }
}

impl From<&[u8]> for PrintArgument {
    fn from(data: &[u8]) -> Self {
        PrintArgument::String(data.to_vec())
    }
}

impl<const N: usize> From<&[u8; N]> for PrintArgument {
    fn from(data: &[u8; N]) -> Self {
        PrintArgument::String(data.to_vec())
    }
}

impl From <Vec<u8>> for PrintArgument {
    fn from(data: Vec<u8>) -> Self {
        PrintArgument::String(data)
    }
}

impl From<Vec<PrintArgument>> for PrintArgument {
    fn from(data: Vec<PrintArgument>) -> Self {
        PrintArgument::Composite(data)
    }
}




impl PrintArgument {
    /// Convert the argument to a sequence of bytes (control codes or text)
    pub fn bytes(&self) -> Vec<u8> {
        match self {
            PrintArgument::String(data) => data.clone(),
            PrintArgument::ChrDollar(char_code) => vec![*char_code],
            PrintArgument::Composite(args) => args.iter().flat_map(|arg| arg.bytes()).collect(),
        }
    }
}

impl Display for PrintArgument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrintArgument::String(data) => {
                let s = String::from_utf8_lossy(data);
                write!(f, "\"{}\"", s)
            },
            PrintArgument::ChrDollar(char_code) => {
                write!(f, "CHR$({})", char_code)
            },
            PrintArgument::Composite(args) => {
                let mut first = true;
                for arg in args {
                    if !first {
                        write!(f, ";")?;
                    }
                    write!(f, "{}", arg)?;
                    first = false;
                }
                Ok(())
            },
        }
    }
}

/// Defines how a PRINT command ends (semicolon or CRLF)
#[derive(Clone, Debug, PartialEq)]
pub enum PrintTerminator {
    /// Ends with semicolon (no new line)
    None,
    /// Ends with CRLF (new line)
    CrLf,
}

impl Display for PrintTerminator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrintTerminator::None => write!(f, ";"),
            PrintTerminator::CrLf => write!(f, ""),
        }
    }
}

/// Represents a CatArt command
#[derive(Clone, Debug, PartialEq)]
pub enum BasicCommand {
    /// BORDER ink1, ink2
    Border(u8, Option<u8>), // 0-31, 0-31
    /// CLS
    Cls,
    /// CURSOR OFF
    CursorOff,
    /// CURSOR ON
    CursorOn,
    /// INK pen, ink1, [ink2]
    Ink(u8, u8, Option<u8>), // pen (0-15), ink1(0-31), ink2(0-31) 
    /// LOCATE col, row
    Locate(u8, u8), // column (1-80), row (1-25)
    /// MODE m
    Mode(u8), // 0-2
    /// PAPER p
    Paper(u8), // 0-15
    /// PEN p
    Pen(u8), // 0-15
    /// PRINT string, terminator
    PrintString(PrintArgument, PrintTerminator),
    /// SYMBOL char, matrix...
    Symbol(u8, u8, u8, u8, u8, u8, u8, u8, u8), // char, row1..row8
    /// WINDOW left, right, top, bottom
    Window(u8, u8, u8, u8), // left (1-80), right (1-80), top (1-25), bottom (1-25)

}

/// A list of BasicCommands with builder pattern for ergonomic construction
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BasicCommandList(Vec<BasicCommand>);

impl BasicCommandList {
    /// Create a new empty BasicCommandList
    pub fn new() -> Self {
        BasicCommandList(Vec::new())
    }

    /// Add a command to the list (builder style)
    pub fn push(mut self, cmd: BasicCommand) -> Self {
        self.0.push(cmd);
        self
    }

    /// Add multiple commands
    pub fn extend<I: IntoIterator<Item=BasicCommand>>(mut self, iter: I) -> Self {
        self.0.extend(iter);
        self
    }

    /// Get the inner Vec
    pub fn into_vec(self) -> Vec<BasicCommand> {
        self.0
    }

    /// Borrow as slice
    pub fn as_slice(&self) -> &[BasicCommand] {
        &self.0
    }
}

impl From<Vec<BasicCommand>> for BasicCommandList {
    fn from(v: Vec<BasicCommand>) -> Self {
        BasicCommandList(v)
    }
}

impl Into<Vec<BasicCommand>> for BasicCommandList {
    fn into(self) -> Vec<BasicCommand> {
        self.0
    }
}

impl BasicCommand {
    /// Create a BORDER command
    pub fn border(ink1: u8, ink2: Option<u8>) -> Self {
        BasicCommand::Border(ink1, ink2)
    }
    /// Create a CLS command
    pub fn cls() -> Self {
        BasicCommand::Cls
    }
    /// Create a CURSOR OFF command
    pub fn cursor_off() -> Self {
        BasicCommand::CursorOff
    }
    /// Create a CURSOR ON command
    pub fn cursor_on() -> Self {
        BasicCommand::CursorOn
    }
    /// Create an INK command
    pub fn ink(pen: u8, ink1: u8, ink2: Option<u8>) -> Self {
        BasicCommand::Ink(pen, ink1, ink2)
    }
    /// Create a LOCATE command
    pub fn locate(col: u8, row: u8) -> Self {
        BasicCommand::Locate(col, row)
    }
    /// Create a MODE command
    pub fn mode(mode: u8) -> Self {
        BasicCommand::Mode(mode)
    }
    /// Create a PAPER command
    pub fn paper(pen: u8) -> Self {
        BasicCommand::Paper(pen)
    }
    /// Create a PEN command
    pub fn pen(pen: u8) -> Self {
        BasicCommand::Pen(pen)
    }
    /// Create a PRINT command without newline (terminated by ;)
    pub fn print_string<S: Into<PrintArgument>>(data: S) -> Self {
        BasicCommand::PrintString(
            data.into(),
            PrintTerminator::None
        )
    }
    /// Create a PRINT command with newline (terminated by nothing)
    pub fn print_string_crlf<S: Into<PrintArgument>>(data: S) -> Self {
        BasicCommand::PrintString(
            data.into(),
            PrintTerminator::CrLf
        )
    }
    /// Create a SYMBOL command
    pub fn symbol(char_code: u8, r1: u8, r2: u8, r3: u8, r4: u8, r5: u8, r6: u8, r7: u8, r8: u8) -> Self {
        BasicCommand::Symbol(char_code, r1, r2, r3, r4, r5, r6, r7, r8)
    }
    /// Create a WINDOW command
    pub fn window(left: u8, right: u8, top: u8, bottom: u8) -> Self {
        BasicCommand::Window(left, right, top, bottom)
    }
}



impl BasicCommandList {
    pub fn to_char_commands(&self) -> Result<CharCommandList, String> {
        let mut result = CharCommandList::new();
        for cmd in &self.0 {
            let cmds = cmd.to_char_commands()?;
            result = result.extend(cmds.as_slice().iter().cloned());
        }
        Ok(result)
    }
}

impl BasicCommand {
    /// Convert the command to a sequence of control codes and bytes for the CPC

    pub fn bytes(&self) -> Vec<u8> {
        match self.to_char_commands() {
            Ok(cmds) => cmds.as_slice().iter().flat_map(|cmd: &CharCommand| cmd.bytes()).collect(),
            Err(_) => vec![],
        }
    }

    /// Convert this BasicCommand into a list of CharCommands (control codes and parameters)
    pub fn to_char_commands(&self) -> Result<CharCommandList, String> {
        match self {
            BasicCommand::Border(ink1, ink2) => Ok(CharCommandList::from(vec![CharCommand::Border(*ink1, ink2.unwrap_or(*ink1))])),
            BasicCommand::Cls => Ok(CharCommandList::from(vec![CharCommand::Cls])),
            BasicCommand::CursorOff => Ok(CharCommandList::from(vec![CharCommand::CursorOff])),
            BasicCommand::CursorOn => Ok(CharCommandList::from(vec![CharCommand::CursorOn])),
            BasicCommand::Ink(pen, ink1, ink2) => Ok(CharCommandList::from(vec![CharCommand::Ink(*pen, *ink1, ink2.unwrap_or(*ink1))])),
            BasicCommand::Locate(col, row) => Ok(CharCommandList::from(vec![CharCommand::Locate(*col, *row)])),
            BasicCommand::Mode(mode) => Ok(CharCommandList::from(vec![CharCommand::Mode(*mode)])),
            BasicCommand::Paper(pen) => Ok(CharCommandList::from(vec![CharCommand::Paper(*pen)])),
            BasicCommand::Pen(pen) => Ok(CharCommandList::from(vec![CharCommand::Pen(*pen)])),
            BasicCommand::PrintString(data, terminator) => {
                let mut cmds = CharCommand::from_string(&data.bytes())?;
                let mut cmds_vec = cmds.into_vec();
                match terminator {
                    PrintTerminator::CrLf => {
                        cmds_vec.push(CharCommand::CarriageReturn);
                        cmds_vec.push(CharCommand::CursorDown);
                    },
                    PrintTerminator::None => {}
                }
                Ok(CharCommandList::from(cmds_vec))
            },
            BasicCommand::Symbol(char_code, r1, r2, r3, r4, r5, r6, r7, r8) => {
                Ok(CharCommandList::from(vec![CharCommand::Symbol(*char_code, *r1, *r2, *r3, *r4, *r5, *r6, *r7, *r8)]))
            },
            BasicCommand::Window(left, right, top, bottom) => {
                Ok(CharCommandList::from(vec![CharCommand::Window(*left, *right, *top, *bottom)]))
            },
        }
    }


}

impl Display for BasicCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BasicCommand::Border(ink1, ink2) => if let Some(ink2) = ink2 {
                write!(f, "BORDER {},{}", ink1, ink2)} else {
                write!(f, "BORDER {}", ink1)
            },
            BasicCommand::Cls => write!(f, "CLS"),
            BasicCommand::CursorOff => write!(f, "CURSOR OFF"),
            BasicCommand::CursorOn => write!(f, "CURSOR ON"),
            BasicCommand::Ink(pen, ink1, ink2) => if let Some(ink2) = ink2 {
                write!(f, "INK {},{},{}", pen, ink1, ink2)} else {
                write!(f, "INK {},{}", pen, ink1)
            },
            BasicCommand::Locate(col, row) => write!(f, "LOCATE {},{}", col, row),
            BasicCommand::Mode(mode) => write!(f, "MODE {}", mode),
            BasicCommand::Paper(pen) => write!(f, "PAPER {}", pen),
            BasicCommand::Pen(pen) => write!(f, "PEN {}", pen),
            BasicCommand::PrintString(data, termin) => {
                write!(f, "PRINT {}{}", data, termin)
            },
            BasicCommand::Symbol(char_code, r1, r2, r3, r4, r5, r6, r7, r8) => {
                write!(f, "SYMBOL {},{},{},{},{},{},{},{},{}", char_code, r1, r2, r3, r4, r5, r6, r7, r8)
            },
            BasicCommand::Window(left, right, top, bottom) => write!(f, "WINDOW {},{},{},{}", left, right, top, bottom),
        }
    }
}

impl BasicCommandList {
    /// Convert all commands in the list to a sequence of bytes

    pub fn bytes(&self) -> Vec<u8> {
        self.0.iter().flat_map(|cmd| cmd.bytes()).collect()
    }

    /// Iterate over the commands
    pub fn iter(&self) -> impl Iterator<Item = &BasicCommand> {
        self.0.iter()
    }

}

impl Display for BasicCommandList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut line = 10;
        for command in &self.0 {
            writeln!(f, "{line} {command}")?;
            line += 10;
        }
        Ok(())
    }
}