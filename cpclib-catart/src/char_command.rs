use cpclib_common::itertools::Itertools;

// Allow iteration over &CharCommandList as &CharCommand
impl<'a> IntoIterator for &'a CharCommandList {
    type IntoIter = std::slice::Iter<'a, CharCommand>;
    type Item = &'a CharCommand;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
/// A list of CharCommands with builder pattern for ergonomic construction
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CharCommandList(Vec<CharCommand>);

impl From<&[u8]> for CharCommandList {
    fn from(data: &[u8]) -> Self {
        Self::from_bytes(data)
    }
}

impl<const C: usize> From<&[u8; C]> for CharCommandList {
    fn from(data: &[u8; C]) -> Self {
        Self::from(&data[..])
    }
}

impl IntoIterator for CharCommandList {
    type IntoIter = std::vec::IntoIter<CharCommand>;
    type Item = CharCommand;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Deref for CharCommandList {
    type Target = [CharCommand];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl CharCommandList {
    /// Create a new empty CharCommandList
    pub fn new() -> Self {
        CharCommandList(Vec::new())
    }

    /// Add a command to the list (builder style)
    pub fn push(&mut self, cmd: CharCommand) -> &mut Self {
        self.0.push(cmd);
        self
    }

    /// Add multiple commands
    pub fn extend<I: IntoIterator<Item = CharCommand>>(&mut self, iter: I) -> &mut Self {
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

    pub fn from_bytes(data: &[u8]) -> Self {
        let mut idx = 0;
        let mut list = Vec::new();
        while idx < data.len() {
            let cmd_or_missing = CharCommand::char_to_command_or_count(data[idx]);
            match cmd_or_missing {
                Ok(cmd) => {
                    list.push(cmd);
                    idx += 1;
                },
                Err(missing) => {
                    let mut params = Vec::new();
                    for i in 0..missing {
                        params.push(data.get(idx + 1 + i)
                            .cloned()
                            .unwrap_or_else(|| {eprintln!("missing byte"); 0xff}));
                    }
                    let cmd = match data[idx] {
                        SOH => CharCommand::PrintSymbol(params[0]),
                        EOT => CharCommand::Mode(params[0]),
                        ENQ => CharCommand::SendGraphics(params[0]),
                        SO => CharCommand::Paper(params[0]),
                        SI => CharCommand::Pen(params[0]),
                        SYN => CharCommand::Transparency(params[0]),
                        ETB => CharCommand::GraphicsInkMode(params[0]),
                        LF => CharCommand::CursorDown,
                        EM => {
                            CharCommand::Symbol(
                                params[0], params[1], params[2], params[3], params[4], params[5],
                                params[6], params[7], params[8]
                            )
                        },
                        SUB => CharCommand::Window(params[0], params[1], params[2], params[3]),
                        FS => CharCommand::Ink(params[0], params[1], params[2]),
                        GS => CharCommand::Border(params[0], params[1]),
                        US => {
                            CharCommand::Locate(
                                params[0],
                                params[1]
                            )
                        }, // catalog 1-based → CharCommand 0-based
                        _ => panic!("Logic error in from_bytes for char {}", data[idx])
                    };
                    list.push(cmd);
                    idx += 1 + missing;
                }
            }
        }
        list.into()
    }

    /// Helper to add n newlines (CR LF) to commands
    pub fn add_newlines(&mut self, count: usize) {
        for _ in 0..count {
            self.0.push(CharCommand::CarriageReturn);
            self.0.push(CharCommand::CursorDown);
        }
    }

    pub fn to_command_string(&self) -> String {
        self.0.iter().map(|cmd| cmd.to_command_string()).join(":")
    }

    pub fn to_basic_string(&self) -> String {
        self.0.iter().map(|cmd| cmd.to_basic_string()).join(":")
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.0.iter().flat_map(|cmd| cmd.bytes()).collect()
    }
}

impl Display for CharCommandList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut interpreter = Interpreter::new_6128();
        interpreter
            .interpret(&self.0, false)
            .map_err(|e| panic!("Failed to interpret commands: {:?}", e));

        let screen_output = interpreter.to_string();
        write!(f, "{}", screen_output)
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

use std::fmt::{Debug, Display};
use std::ops::Deref;

use cpclib_common::smallvec::{SmallVec, smallvec};

use crate::basic_chars::*;
use crate::basic_command::{BasicCommand, PrintArgument, PrintTerminator};
use crate::interpret::Interpreter;

/// Represents the possible commands encoded in a stream of characters on the Amstrad CPC
#[derive(Clone, PartialEq, Eq)]
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
    Mode(u8), // 0, 1, 2
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
    /// Cursor Down (0x0A) Line Feed LF
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
    /// Clear to end of line (0x12)
    ClearCursorToLineEnd,
    /// Clear to start of line (0x11)
    ClearLineStartToCursor,
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
    /// There is an offset of 1 between the BASIC command and the CPC char command.
    Window(u8, u8, u8, u8),
    /// Set Ink (0x1C). Even if 2 inks are setup there is no flashing
    Ink(u8, u8, u8),
    /// Set Border (0x1D)
    Border(u8, u8),
    /// Home (0x1E)
    Home,
    /// Locate (0x1F)
    /// Ofsset by -1 in comparison to basic
    Locate(u8, u8),
    /// Standard character
    Char(u8),
    String(Vec<u8>) // list of ascii chars
}

impl Debug for CharCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CharCommand::Nop => write!(f, "Nop"),
            CharCommand::PrintSymbol(c) => write!(f, "PrintSymbol({})", c),
            CharCommand::CursorOff => write!(f, "CursorOff"),
            CharCommand::CursorOn => write!(f, "CursorOn"),
            CharCommand::Mode(m) => write!(f, "Mode({})", m),
            CharCommand::SendGraphics(m) => write!(f, "SendGraphics({})", m),
            CharCommand::EnableVdu => write!(f, "EnableVdu"),
            CharCommand::Beep => write!(f, "Beep"),
            CharCommand::CursorLeft => write!(f, "CursorLeft"),
            CharCommand::CursorRight => write!(f, "CursorRight"),
            CharCommand::CursorDown => write!(f, "CursorDown"),
            CharCommand::CursorUp => write!(f, "CursorUp"),
            CharCommand::Cls => write!(f, "Cls"),
            CharCommand::CarriageReturn => write!(f, "CarriageReturn"),
            CharCommand::Paper(p) => write!(f, "Paper({})", p),
            CharCommand::Pen(p) => write!(f, "Pen({})", p),
            CharCommand::Delete => write!(f, "Delete"),
            CharCommand::ClearCursorToLineEnd => write!(f, "ClearLineEnd"),
            CharCommand::ClearLineStartToCursor => write!(f, "ClearLineStart"),
            CharCommand::ClearScreenEnd => write!(f, "ClearScreenEnd"),
            CharCommand::ClearScreenStart => write!(f, "ClearScreenStart"),
            CharCommand::DisableVdu => write!(f, "DisableVdu"),
            CharCommand::Transparency(p) => write!(f, "Transparency({})", p),
            CharCommand::GraphicsInkMode(p) => write!(f, "GraphicsInkMode({})", p),
            CharCommand::ExchangePenAndPaper => write!(f, "ExchangePenAndPaper"),
            CharCommand::Symbol(c, r1, r2, r3, r4, r5, r6, r7, r8) => {
                write!(
                    f,
                    "Symbol({}, {}, {}, {}, {}, {}, {}, {}, {})",
                    c, r1, r2, r3, r4, r5, r6, r7, r8
                )
            },
            CharCommand::Window(l, r, t, b) => write!(f, "Window({}, {}, {}, {})", l, r, t, b),
            CharCommand::Esc => write!(f, "Esc"),
            CharCommand::Ink(p, i1, i2) => write!(f, "Ink({}, {}, {})", p, i1, i2),
            CharCommand::Border(i1, i2) => write!(f, "Border({}, {})", i1, i2),
            CharCommand::Home => write!(f, "Home"),
            CharCommand::Locate(c, l) => write!(f, "Locate({}, {})", c, l),
            CharCommand::Char(c) => {
                if c.is_ascii_graphic() {
                    write!(f, "Char('{}')", *c as char)
                }
                else {
                    write!(f, "Char({})", c)
                }
            },
            CharCommand::String(s) => write!(f, "String({:?})", s)
        }
    }
}

impl CharCommand {
    pub fn to_command_string(&self) -> String {
        match self {
            CharCommand::Nop => "NOP".to_string(),
            CharCommand::PrintSymbol(c) => format!("PRINT CHR$({})", c),
            CharCommand::CursorOff => "CURSOR OFF".to_string(),
            CharCommand::CursorOn => "CURSOR ON".to_string(),
            CharCommand::Mode(m) => format!("MODE {}", m),
            CharCommand::SendGraphics(m) => format!("SendGraphics({})", m),
            CharCommand::EnableVdu => "ENABLE".to_string(),
            CharCommand::Beep => "BEEP".to_string(),
            CharCommand::CursorLeft => "LEFT".to_string(),
            CharCommand::CursorRight => "RIGHT".to_string(),
            CharCommand::CursorDown => "DOWN".to_string(),
            CharCommand::CursorUp => "UP".to_string(),
            CharCommand::Cls => "CLS".to_string(),
            CharCommand::CarriageReturn => "CR".to_string(),
            CharCommand::Paper(p) => format!("PAPER {}", p),
            CharCommand::Pen(p) => format!("PEN {}", p),
            CharCommand::Delete => "DELETE".to_string(),
            CharCommand::ClearCursorToLineEnd => "CLEAR LINE END".to_string(),
            CharCommand::ClearLineStartToCursor => "CLEAR LINE START".to_string(),
            CharCommand::ClearScreenEnd => "CLEAR SCREEN END".to_string(),
            CharCommand::ClearScreenStart => "CLEAR SCREEN START".to_string(),
            CharCommand::DisableVdu => "DISABLE".to_string(),
            CharCommand::Transparency(p) => format!("TRANSPARENCY {}", p),
            CharCommand::GraphicsInkMode(p) => format!("GFX INK MODE {}", p),
            CharCommand::ExchangePenAndPaper => "EXCHANGE PEN AND PAPER".to_string(),
            CharCommand::Symbol(c, r1, r2, r3, r4, r5, r6, r7, r8) => {
                format!(
                    "Symbol({}, {}, {}, {}, {}, {}, {}, {}, {})",
                    c, r1, r2, r3, r4, r5, r6, r7, r8
                )
            },
            CharCommand::Window(l, r, t, b) => {
                format!("WINDOW {}, {}, {}, {}", l, r, t, b)
            },
            CharCommand::Esc => "ESC".to_string(),
            CharCommand::Ink(p, i1, i2) => format!("INK {}, {}, {}", p, i1, i2),
            CharCommand::Border(i1, i2) => format!("BORDER {}, {}", i1, i2),
            CharCommand::Home => "HOME".to_string(),
            CharCommand::Locate(c, l) => format!("LOCATE {}, {}", c, l),
            CharCommand::Char(c) => {
                if c.is_ascii_graphic() || *c == b' ' {
                    format!("PRINT \"{}\"", *c as char)
                }
                else {
                    format!("PRINT CHR$({})", c)
                }
            },
            CharCommand::String(s) => format!("PRINT \"{}\"", String::from_utf8_lossy(&s))
        }
    }

    pub fn len(&self) -> usize {
        self.bytes().len()
    }

    #[inline]
    pub fn first_byte(&self) -> u8 {
        self.bytes()[0]
    }

    #[inline]
    pub fn second_byte(&self) -> u8 {
        self.bytes()[1]
    }

    #[inline]
    pub fn third_byte(&self) -> u8 {
        self.bytes()[2]
    }

    // Ensure some Char are translated to their command
    pub fn normalize(self) -> Self {
        match self {
            Self::Char(NAK) => Self::DisableVdu,
            Self::Char(ACK) => Self::EnableVdu,
            _ => self
        }
    }

    /// Convert the command to a sequence of control codes and bytes for the CPC
    pub fn bytes(&self) -> SmallVec<[u8; 3]> {
        match self {
            CharCommand::Nop => smallvec![NUL],
            CharCommand::PrintSymbol(c) => smallvec![SOH, *c],
            CharCommand::CursorOff => smallvec![STX],
            CharCommand::CursorOn => smallvec![ETX],
            CharCommand::SendGraphics(m) => smallvec![ENQ, *m],
            CharCommand::Mode(m) => smallvec![EOT, *m],
            CharCommand::EnableVdu => smallvec![ACK],
            CharCommand::Beep => smallvec![BEL],
            CharCommand::CursorLeft => smallvec![BS],
            CharCommand::CursorRight => smallvec![TAB],
            CharCommand::CursorDown => smallvec![LF],
            CharCommand::CursorUp => smallvec![VT],
            CharCommand::Cls => smallvec![FF],
            CharCommand::CarriageReturn => smallvec![CR],
            CharCommand::Paper(p) => smallvec![SO, *p],
            CharCommand::Pen(p) => smallvec![SI, *p],
            CharCommand::Delete => smallvec![DLE],
            CharCommand::ClearCursorToLineEnd => smallvec![DC2],
            CharCommand::ClearLineStartToCursor => smallvec![DC1],
            CharCommand::ClearScreenEnd => smallvec![DC3],
            CharCommand::ClearScreenStart => smallvec![DC4],
            CharCommand::DisableVdu => smallvec![NAK],
            CharCommand::Transparency(p) => smallvec![SYN, *p],
            CharCommand::GraphicsInkMode(p) => smallvec![ETB, *p],
            CharCommand::ExchangePenAndPaper => smallvec![CAN],
            CharCommand::Symbol(c, r1, r2, r3, r4, r5, r6, r7, r8) => {
                smallvec![EM, *c, *r1, *r2, *r3, *r4, *r5, *r6, *r7, *r8]
            },
            CharCommand::Window(l, r, t, b) => smallvec![SUB, *l, *r, *t, *b],
            CharCommand::Esc => smallvec![ESC],
            CharCommand::Ink(p, i1, i2) => smallvec![FS, *p, *i1, *i2],
            CharCommand::Border(i1, i2) => smallvec![GS, *i1, *i2],
            CharCommand::Home => smallvec![RS],
            CharCommand::Locate(c, l) => smallvec![US, *c, *l],
            CharCommand::Char(c) => smallvec![*c],
            CharCommand::String(s) => s.iter().cloned().collect()
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
            DC1 => Ok(CharCommand::ClearLineStartToCursor),
            DC2 => Ok(CharCommand::ClearCursorToLineEnd),
            DC3 => Ok(CharCommand::ClearScreenStart),
            DC4 => Ok(CharCommand::ClearScreenEnd),
            NAK => Ok(CharCommand::DisableVdu),
            CAN => Ok(CharCommand::ExchangePenAndPaper),
            ESC => Ok(CharCommand::Esc),
            RS => Ok(CharCommand::Home),

            0x20..=0x7F | 0x80..=0xFF => Ok(CharCommand::Char(c)),
            _ => Err(NB_PARAMS_FOR_CODE[c as usize] as usize)
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
                        }
                        else {
                            return Err(format!(
                                "Not enough parameters to build CharCommand for char {}",
                                c
                            ));
                        }
                    }
                    let cmd = match *c {
                        SOH => CharCommand::PrintSymbol(params[0]),
                        EOT => CharCommand::Mode(params[0]),
                        ENQ => CharCommand::SendGraphics(params[0]),
                        SO => CharCommand::Paper(params[0]),
                        SI => CharCommand::Pen(params[0]),
                        SYN => CharCommand::Transparency(params[0]),
                        ETB => CharCommand::GraphicsInkMode(params[0]),
                        EM => {
                            CharCommand::Symbol(
                                params[0], params[1], params[2], params[3], params[4], params[5],
                                params[6], params[7], params[8]
                            )
                        },
                        SUB => CharCommand::Window(params[0], params[1], params[2], params[3]),
                        FS => CharCommand::Ink(params[0], params[1], params[2]),
                        GS => CharCommand::Border(params[0], params[1]),
                        US => CharCommand::Locate(params[0], params[1]),
                        _ => return Err(format!("Logic error in from_string for char {}", c))
                    };
                    res.push(cmd);
                }
            }
        }
        Ok(CharCommandList::from(res))
    }

    pub fn is_mode(&self) -> bool {
        matches!(self, CharCommand::Mode(_))
    }

    pub fn is_pen(&self) -> bool {
        matches!(self, CharCommand::Pen(_))
    }

    pub fn is_paper(&self) -> bool {
        matches!(self, CharCommand::Paper(_))
    }

    pub fn is_ink(&self) -> bool {
        matches!(self, CharCommand::Ink(_, _, _))
    }

    pub fn is_border(&self) -> bool {
        matches!(self, CharCommand::Border(_, _))
    }

    pub fn is_locate(&self) -> bool {
        matches!(self, CharCommand::Locate(_, _))
    }

    pub fn is_print_symbol(&self) -> bool {
        matches!(self, CharCommand::PrintSymbol(_))
    }

    pub fn to_basic_command(&self) -> Option<BasicCommand> {
        match self {
            CharCommand::Mode(m) => Some(BasicCommand::Mode(*m)),
            CharCommand::Paper(p) => Some(BasicCommand::Paper(*p)),
            CharCommand::Pen(p) => Some(BasicCommand::Pen(*p)),
            CharCommand::Ink(p, i1, i2) => Some(BasicCommand::Ink(*p, *i1, Some(*i2))),
            CharCommand::Border(i1, i2) => Some(BasicCommand::Border(*i1, Some(*i2))),
            CharCommand::Locate(c, l) => Some(BasicCommand::Locate(c.wrapping_add(1), l.wrapping_add(1))), /* CharCommand 0-based → BASIC 1-based */
            CharCommand::Window(a, b, c, d) => {
                Some(BasicCommand::Window(a.wrapping_add(1), b.wrapping_add(1), c.wrapping_add(1), d.wrapping_add(1)))
            }, /* CharCommand 0-based → BASIC 1-based */
            CharCommand::PrintSymbol(c) => {
                Some(BasicCommand::PrintString(
                    PrintArgument::from(*c),
                    PrintTerminator::None
                ))
            },
            CharCommand::Char(c) => {
                Some(BasicCommand::PrintString(
                    PrintArgument::from(*c),
                    PrintTerminator::None
                ))
            },
            CharCommand::String(s) => {
                Some(BasicCommand::PrintString(
                    PrintArgument::from(s.clone()),
                    PrintTerminator::None
                ))
            },
            CharCommand::CarriageReturn => {
                Some(BasicCommand::PrintString(
                    PrintArgument::from(CR),
                    PrintTerminator::None
                ))
            },
            CharCommand::CursorDown => {
                Some(BasicCommand::PrintString(
                    PrintArgument::from(LF),
                    PrintTerminator::None
                ))
            },
            CharCommand::Beep => {
                Some(BasicCommand::PrintString(
                    PrintArgument::from(BEL),
                    PrintTerminator::None
                ))
            },
            CharCommand::CursorLeft => {
                Some(BasicCommand::PrintString(
                    PrintArgument::from(BS),
                    PrintTerminator::None
                ))
            },
            CharCommand::CursorRight => {
                Some(BasicCommand::PrintString(
                    PrintArgument::from(TAB),
                    PrintTerminator::None
                ))
            },
            CharCommand::CursorUp => {
                Some(BasicCommand::PrintString(
                    PrintArgument::from(VT),
                    PrintTerminator::None
                ))
            },
            CharCommand::Cls => Some(BasicCommand::Cls),
            CharCommand::EnableVdu => {
                Some(BasicCommand::PrintString(
                    PrintArgument::from(ACK),
                    PrintTerminator::None
                ))
            },
            CharCommand::DisableVdu => {
                Some(BasicCommand::PrintString(
                    PrintArgument::from(NAK),
                    PrintTerminator::None
                ))
            },
            CharCommand::GraphicsInkMode(mode) => {
                Some(BasicCommand::PrintString(
                    PrintArgument::from(vec![ETB, *mode]),
                    PrintTerminator::None
                ))
            },
            CharCommand::Nop => None, // no equivalent in BASIC
            CharCommand::Transparency(v) => {
                Some(BasicCommand::PrintString(
                    PrintArgument::Composite(vec![SYN.into(), (*v).into()]),
                    PrintTerminator::None
                ))
            },

            CharCommand::ClearCursorToLineEnd => {
                Some(BasicCommand::PrintString(
                    PrintArgument::from(DC1),
                    PrintTerminator::None
                ))
            }
            _ => unimplemented!("to_basic_command not implemented for command {:?}", self)
        }
    }

    pub fn to_basic_string(&self) -> String {
        self.to_basic_command()
            .map(|cmd: BasicCommand| cmd.to_string())
            .unwrap_or_else(|| {
                match self {
                    CharCommand::Nop => String::new(),
                    _ => panic!("to_basic_string not implemented for command {:?}", self)
                }
            })
    }
}
