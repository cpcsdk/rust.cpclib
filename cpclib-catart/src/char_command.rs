// Allow iteration over &CharCommandList as &CharCommand
impl<'a> IntoIterator for &'a CharCommandList {
    type Item = &'a CharCommand;
    type IntoIter = std::slice::Iter<'a, CharCommand>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
/// A list of CharCommands with builder pattern for ergonomic construction
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CharCommandList(Vec<CharCommand>);

impl CharCommandList {
    /// Create a new empty CharCommandList
    pub fn new() -> Self {
        CharCommandList(Vec::new())
    }

    /// Add a command to the list (builder style)
    pub fn push(mut self, cmd: CharCommand) -> Self {
        self.0.push(cmd);
        self
    }

    /// Add multiple commands
    pub fn extend<I: IntoIterator<Item=CharCommand>>(mut self, iter: I) -> Self {
        self.0.extend(iter);
        self
    }

    /// Get the inner Vec
    pub fn into_vec(self) -> Vec<CharCommand> {
        self.0
    }

    /// Borrow as slice
    pub fn as_slice(&self) -> &[CharCommand] {
        &self.0
    }

    pub fn iter(&self) -> std::slice::Iter<'_, CharCommand> {
        self.0.iter()
    }
}

impl From<Vec<CharCommand>> for CharCommandList {
    fn from(v: Vec<CharCommand>) -> Self {
        CharCommandList(v)
    }
}

impl Into<Vec<CharCommand>> for CharCommandList {
    fn into(self) -> Vec<CharCommand> {
        self.0
    }
}

// This file encodes the CatArt chars commands as they could be handled in a stream of chars.
// They are mainly produced from a Basic list of command. But it is still possible to create them

use crate::basic_chars::*;

/// Represents the possible commands encoded in a stream of characters on the Amstrad CPC
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharCommand {
    /// No operation (0x00)
    Nop,
    /// Print a specific character symbol (0x01)
    PrintSymbol(u8),
    /// Disable cursor (0x02)
    CursorOff,
    /// Enable cursor (0x03)
    CursorOn,
    /// Set Mode (0x04)
    SetMode(u8), // 0, 1, 2
    /// Send Graphics (0x05). however we cannot position the graphics cursor, so usage is very limite
    SendGraphics(u8),
    /// Enable VDU (0x06)
    EnableVdu,
    /// Beep (0x07)
    Beep,
    /// Cursor Left (0x08)
    CursorLeft,
    /// Cursor Right (0x09)
    CursorRight,
    /// Cursor Down (0x0A)
    CursorDown,
    /// Cursor Up (0x0B)
    CursorUp,
    /// Clear Screen (0x0C)
    Cls,
    Esc,
    /// Carriage Return (0x0D)
    CarriageReturn,
    /// Set Paper (0x0E)
    Paper(u8),
    /// Set Pen (0x0F)
    Pen(u8),
    /// Delete character (0x10)
    Delete,
    /// Clear to end of line (0x11)
    ClearLineEnd,
    /// Clear to start of line (0x12)
    ClearLineStart,
    /// Clear to end of screen (0x13)
    ClearScreenEnd,
    /// Clear to start of screen (0x14)
    ClearScreenStart,
    /// Disable VDU (0x15)
    DisableVdu,
    /// Transparency
    Transparency(u8),
    /// Graphics Ink Mode (0x17)/ Not really usable it is for graphic mode
    GraphicsInkMode(u8),
    /// Exchange Pen/Paper (0x18)
    ExchangePenAndPaper,
    /// Define Symbol (0x19). However we cannot print the sybols that can be redifined
    Symbol(u8, u8, u8, u8, u8, u8, u8, u8, u8),
    /// Define Window (0x1A)
    Window(u8, u8, u8, u8),
    /// Set Ink (0x1C). Even if 2 inks are setup there is no flashing
    Ink(u8, u8, u8),
    /// Set Border (0x1D)
    Border(u8, u8),
    /// Home (0x1E)
    Home,
    /// Locate (0x1F)
    Locate(u8, u8),
    /// Standard character
    Char(u8)
}

impl CharCommand {
    /// Convert the command to a sequence of control codes and bytes for the CPC
    pub fn bytes(&self) -> Vec<u8>{
        match self {
            CharCommand::Nop => vec![NUL],
            CharCommand::PrintSymbol(c) => vec![SOH, *c],
            CharCommand::CursorOff => vec![STX],
            CharCommand::CursorOn => vec![ETX],
            CharCommand::SendGraphics(m) => vec![ENQ, *m],
            CharCommand::SetMode(m) => vec![EOT, *m],
            CharCommand::EnableVdu => vec![ACK],
            CharCommand::Beep => vec![BEL],
            CharCommand::CursorLeft => vec![BS],
            CharCommand::CursorRight => vec![TAB],
            CharCommand::CursorDown => vec![LF],
            CharCommand::CursorUp => vec![VT],
            CharCommand::Cls => vec![FF],
            CharCommand::CarriageReturn => vec![CR],
            CharCommand::Paper(p) => vec![SO, *p],
            CharCommand::Pen(p) => vec![SI, *p],
            CharCommand::Delete => vec![DLE],
            CharCommand::ClearLineEnd => vec![DC1],
            CharCommand::ClearLineStart => vec![DC2],
            CharCommand::ClearScreenEnd => vec![DC3],
            CharCommand::ClearScreenStart => vec![DC4],
            CharCommand::DisableVdu => vec![NAK],
            CharCommand::Transparency(p) => vec![SYN, *p],
            CharCommand::GraphicsInkMode(p) => vec![ETB, *p],
            CharCommand::ExchangePenAndPaper => vec![CAN],
            CharCommand::Symbol(c, r1, r2, r3, r4, r5, r6, r7, r8) => {
                vec![EM, *c, *r1, *r2, *r3, *r4, *r5, *r6, *r7, *r8]
            },
            CharCommand::Window(l, r, t, b) => vec![SUB, *l, *r, *t, *b],
            CharCommand::Esc => vec![ESC],
            CharCommand::Ink(p, i1, i2) => vec![FS, *p, *i1, *i2],
            CharCommand::Border(i1, i2) => vec![GS, *i1, *i2],
            CharCommand::Home => vec![RS],
            CharCommand::Locate(c, l) => vec![US, *c, *l],
            CharCommand::Char(c) => vec![*c],
        }
    }

    /// Return the given char command OR the number of missing chars required to build the appropriate command
    pub fn char_to_command_or_count(c: u8) -> Result<Self, usize> {
        match c {
            NUL => Ok(CharCommand::Nop),
            STX => Ok(CharCommand::CursorOff),
            ETX => Ok(CharCommand::CursorOn),
            ACK => Ok(CharCommand::EnableVdu),
            BEL => Ok(CharCommand::Beep),
            BS => Ok(CharCommand::CursorLeft),
            TAB => Ok(CharCommand::CursorRight),
            LF => Ok(CharCommand::CursorDown),
            VT => Ok(CharCommand::CursorUp),
            FF => Ok(CharCommand::Cls),
            CR => Ok(CharCommand::CarriageReturn),
            DLE => Ok(CharCommand::Delete),
            DC1 => Ok(CharCommand::ClearLineStart),
            DC2 => Ok(CharCommand::ClearLineEnd),
            DC3 => Ok(CharCommand::ClearScreenStart),
            DC4 => Ok(CharCommand::ClearScreenEnd),
            NAK => Ok(CharCommand::DisableVdu),
            CAN => Ok(CharCommand::ExchangePenAndPaper),
            ESC => Ok(CharCommand::Esc),
            RS => Ok(CharCommand::Home),
            
            0x20..=0x7F | 0x80..=0xFF => Ok(CharCommand::Char(c)),
            _ => Err(NB_PARAMS_FOR_CODE[c as usize] as usize), 
        }
    }

    /// Parse a stream of bytes into a sequence of CharCommands.
    /// Returns a CharCommandList or an error string if parsing fails.
    pub fn from_string(data: &[u8]) -> Result<CharCommandList, String> {
        let mut res = Vec::new();
        let mut iter = data.iter();
        while let Some(c) = iter.next() {
            let cmd_or_missing = CharCommand::char_to_command_or_count(*c);
            match cmd_or_missing {
                Ok(cmd) => res.push(cmd),
                Err(mut missing) => {
                    let mut params = Vec::new();
                    while missing > 0 {
                        if let Some(pc) = iter.next() {
                            params.push(*pc);
                            missing -= 1;
                        } else {
                            return Err(format!("Not enough parameters to build CharCommand for char {}", c));
                        }
                    }
                    let cmd = match *c {
                        SOH => CharCommand::PrintSymbol(params[0]),
                        EOT => CharCommand::SetMode(params[0]),
                        ENQ => CharCommand::SendGraphics(params[0]),
                        SO => CharCommand::Paper(params[0]),
                        SI => CharCommand::Pen(params[0]),
                        SYN => CharCommand::Transparency(params[0]),
                        ETB => CharCommand::GraphicsInkMode(params[0]),
                        EM => CharCommand::Symbol(params[0], params[1], params[2], params[3], params[4], params[5], params[6], params[7], params[8]),
                        SUB => CharCommand::Window(params[0], params[1], params[2], params[3]),
                        FS => CharCommand::Ink(params[0], params[1], params[2]),
                        GS => CharCommand::Border(params[0], params[1]),
                        US => CharCommand::Locate(params[0], params[1]),
                        _ => return Err(format!("Logic error in from_string for char {}", c)),
                    };
                    res.push(cmd);
                }
            }
        }
        Ok(CharCommandList::from(res))
    }
}