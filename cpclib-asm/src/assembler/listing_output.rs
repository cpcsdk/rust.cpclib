use std::fmt::{Debug, Display};
use std::io::Write;
use std::sync::{Arc, RwLock};

use cpclib_common::itertools::Itertools;
use cpclib_common::smallvec::SmallVec;
use cpclib_tokens::symbols::PhysicalAddress;
use cpclib_tokens::{ExprResult, Token};

use crate::preamble::{LocatedToken, MayHaveSpan};
/// Generate an output listing.
/// Can be useful to detect issues
pub struct ListingOutput {
    /// Writer that will contains the listing/
    /// The listing is produced line by line and not token per token
    writer: Box<dyn Write + Send + Sync>,
    /// Filename of the current line
    current_fname: Option<String>,
    activated: bool,

    /// Bytes collected at the current line
    current_line_bytes: SmallVec<[u8; 4]>,
    /// Complete source
    current_source: Option<&'static str>,
    /// Line number and line content.
    current_line_group: Option<(u32, String)>, // clone view of the line XXX avoid this clone

    current_first_address: u32,
    current_address_kind: AddressKind,
    current_physical_address: PhysicalAddress,
    crunched_section_counter: usize
}
#[derive(PartialEq)]
pub enum AddressKind {
    Address,
    CrunchedArea,
    Mixed,
    None
}

impl Display for AddressKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AddressKind::Address => ' ',
                AddressKind::CrunchedArea => 'C',
                AddressKind::Mixed => 'M',
                AddressKind::None => 'N'
            }
        )
    }
}

impl Debug for ListingOutput {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl ListingOutput {
    /// Build a new ListingOutput that will write everyting in writter
    pub fn new<W: 'static + Write + Send + Sync>(writer: W) -> Self {
        Self {
            writer: Box::new(writer),
            current_fname: None,
            activated: false,
            current_line_bytes: Default::default(),
            current_line_group: None,
            current_source: None,
            current_first_address: 0,
            current_address_kind: AddressKind::None,
            crunched_section_counter: 0,
            current_physical_address: PhysicalAddress::new(0, 0)
        }
    }

    fn bytes_per_line(&self) -> usize {
        8
    }

    /// Check if the token is for the same source
    fn token_is_on_same_source(&self, token: &LocatedToken) -> bool {
        match &self.current_source {
            Some(current_source) => {
                std::ptr::eq(token.context().source.as_ptr(), current_source.as_ptr())
            }
            None => false
        }
    }

    /// Check if the token is for the same line than the previous token
    fn token_is_on_same_line(&self, token: &LocatedToken) -> bool {
        match &self.current_line_group {
            Some((current_location, _current_line)) => {
                self.token_is_on_same_source(token)
                    && *current_location == token.span().location_line()
            }
            None => false
        }
    }

    fn extract_code(token: &LocatedToken) -> String {
        match token {
            LocatedToken::Standard {
                token: Token::Macro(..),
                span
            } => {
                // 		self.need_to_cut = true;
                span.fragment().to_string()
            }

            _ => {
                // 			self.need_to_cut = false;
                unsafe { std::str::from_utf8_unchecked(token.span().get_line_beginning()) }
                    .to_owned()
            }
        }
    }

    /// Add a token for the current line
    fn add_token(
        &mut self,
        token: &LocatedToken,
        bytes: &[u8],
        address: u32,
        address_kind: AddressKind,
        physical_address: PhysicalAddress
    ) {
        if !self.activated {
            return;
        }

        let fname_handling = self.manage_fname(token);

        // Handle specific tokens with a specific behavior
        let specific_content = match token {
            LocatedToken::Label(l) => {
                self.current_line_group = Some((token.span().location_line(), Self::extract_code(token)));

                Some(format!("{:04X} {:05X} {l}", self.current_first_address, self.current_physical_address.offset_in_cpc()))
            },

            LocatedToken::Equ{label, ..} => {
                self.current_line_group = Some((token.span().location_line(), Self::extract_code(token)));
                Some(format!("{:04X} {} {label}", self.current_first_address, "-----"))
            }

            _ => None
        };

        if specific_content.is_some() || !self.token_is_on_same_line(token) {
            // handle previous line
            self.process_current_line(specific_content); // request a display

            // handle the new line

            // replace the objects of interest
            self.current_source = Some(token.context().source);

            // TODO manage differently for macros and so on
            // let current_line = current_line.split("\n").next().unwrap_or(current_line);
            self.current_line_group =
                Some((token.span().location_line(), Self::extract_code(token)));
            self.current_first_address = address;
            self.current_physical_address = physical_address;
            self.current_address_kind = AddressKind::None;
            self.manage_fname(token);
        } else {
            // update the line
            self.current_line_group = Some((token.span().location_line(), Self::extract_code(token)));
        }




        self.current_line_bytes.extend_from_slice(bytes);
        self.current_address_kind = if self.current_address_kind == AddressKind::None {
            address_kind
        }
        else if self.current_address_kind != address_kind {
            AddressKind::Mixed
        }
        else {
            address_kind
        };

        if let Some(line) = fname_handling {
            writeln!(self.writer, "{}", line).unwrap();
        }
    }

    pub fn process_current_line(&mut self, specific_content: Option<String>) {
        // retrieve the line
        let (line_number, line) = match &self.current_line_group {
            Some((idx, line)) => (idx, line),
            None => return
        };

        // build the iterators over the line representation of source code and data
        let mut line_representation = line.split("\n");
        let data_representation = &self
            .current_line_bytes
            .iter()
            .chunks(self.bytes_per_line())
            .into_iter()
            .map(|c| c.map(|b| format!("{:02X}", b)).join(" "))
            .collect_vec();
        let mut data_representation = data_representation.iter();

        // TODO manage missing end of files/blocks if needed

        // draw all lines that correspond to the instructions to output
        let mut idx = 0;
        loop {
            let current_inner_line = line_representation.next();
            let current_inner_data = data_representation.next();

            if current_inner_data.is_none() && current_inner_line.is_none() {
                break;
            }

            let loc_representation = if current_inner_line.is_none() {
                "    ".to_owned()
            }
            else {
                format!("{:04X}", self.current_first_address)
            };

            // Physical address is only printed if it differs from the code address
            let offset = self.current_physical_address.offset_in_cpc();
            let phys_addr_representation =
                if current_inner_line.is_none() || offset == self.current_first_address {
                    "      ".to_owned()
                }
                else {
                    format!("{:05X}{}", offset, self.current_address_kind)
                };

            let line_nb_representation = if current_inner_line.is_none() {
                "    ".to_owned()
            }
            else {
                format!("{:4}", line_number + idx)
            };


            if specific_content.is_none() || !self.current_line_bytes.is_empty() {
                writeln!(
                    self.writer,
                    "{loc_representation} {phys_addr_representation} {:bytes_width$} {line_nb_representation} {}",
                    current_inner_data.unwrap_or(&"".to_owned()),
                    current_inner_line.map(|line| line.trim_end()).unwrap_or(""),
                    bytes_width = self.bytes_per_line() * 3
                )
                .unwrap();
            }

            idx += 1;
        }

        // TODO add the line representation ?
        if let Some(specific_content) = specific_content {
            let lines = line.split("\n");
            let delta = line_representation.count();
            let line = lines.last().unwrap();
            writeln!(
                self.writer,
                "{:37}{:4} {}",
                specific_content,
                line_number + delta as u32,
                line

            )
            .unwrap();
        }

        // cleanup all the fields of the current line
        self.current_line_group = None;
        self.current_source = None;
        self.current_line_bytes.clear();
    }

    pub fn finish(&mut self) {
        self.process_current_line(None)
    }

    /// Print filename if needed
    pub fn manage_fname(&mut self, token: &LocatedToken) -> Option<String> {
        // 	dbg!(token);

        let ctx = &token.span().extra;
        let fname = ctx
            .filename()
            .map(|p| p.as_os_str().to_str().unwrap().to_string())
            .or_else(|| ctx.context_name().map(|s| s.to_owned()));

        match fname {
            Some(fname) => {
                let print = match self.current_fname.as_ref() {
                    Some(current_fname) => *current_fname != fname,
                    None => true
                };

                if print {
                    self.current_fname = Some(fname.clone());
                    Some(format!("Context: {}", fname))
                }
                else {
                    None
                }
            }
            None => None
        }
    }

    pub fn on(&mut self) {
        self.activated = true;
    }

    pub fn off(&mut self) {
        self.finish();
        self.activated = false;
    }

    pub fn enter_crunched_section(&mut self) {
        self.crunched_section_counter += 1;
    }

    pub fn leave_crunched_section(&mut self) {
        self.crunched_section_counter -= 1;
    }
}

/// This structure collects the necessary information to feed the output
#[derive(Clone)]
pub struct ListingOutputTrigger {
    /// the token read before collecting the bytes
    /// Because each token can have a different lifespan, we store them using a pointer
    pub(crate) token: Option<*const LocatedToken>,
    /// the bytes progressively collected
    pub(crate) bytes: Vec<u8>,
    pub(crate) start: u32,
    pub(crate) physical_address: PhysicalAddress,
    pub(crate) builder: Arc<RwLock<ListingOutput>>
}

unsafe impl Sync for ListingOutputTrigger {}

impl ListingOutputTrigger {
    pub fn write_byte(&mut self, b: u8) {
        self.bytes.push(b);
    }

    pub fn new_token(
        &mut self,
        new: *const LocatedToken,
        code: u32,
        kind: AddressKind,
        physical_address: PhysicalAddress
    ) {

        if let Some(token) = &self.token {
            self.builder.write().unwrap().add_token(
                unsafe { &**token },
                &self.bytes,
                self.start,
                kind,
                self.physical_address
            );
        }

        self.token.replace(new.clone()); // TODO remove that clone that is memory/time eager

        // TODO double check if these lines are current. I doubt it is the case when having severl instructions per line
        self.bytes.clear();
        self.start = code;
        self.physical_address = physical_address;
    }

    /// Override the address value by the expression result
    /// BUGGY when it is not a number ...
    pub fn replace_code_address(&mut self, address: ExprResult) {
        match address {
            ExprResult::Float(_f) => {}
            ExprResult::Value(v) => self.start = v as _,
            ExprResult::Char(v) => self.start = v as _,
            ExprResult::Bool(b) => self.start = if b { 1 } else { 0 },
            ExprResult::String(s) => self.start = s.len() as _,
            ExprResult::List(l) => self.start = l.len() as _,
            ExprResult::Matrix {
                width,
                height,
                content: _
            } => self.start = (width + height) as _
        };

    }

    pub fn replace_physical_address(&mut self, address: PhysicalAddress) {
        self.physical_address = address;
    }

    pub fn finish(&mut self) {
        if let Some(token) = &self.token {
            self.builder.write().unwrap().add_token(
                unsafe { &**token },
                &self.bytes,
                self.start,
                AddressKind::Address,
                self.physical_address
            );
        }
        self.builder.write().unwrap().finish();
    }

    pub fn on(&mut self) {
        self.builder.write().unwrap().on();
    }

    pub fn off(&mut self) {
        self.builder.write().unwrap().off();
    }

    pub fn enter_crunched_section(&mut self) {
        self.builder.write().unwrap().enter_crunched_section();
    }

    pub fn leave_crunched_section(&mut self) {
        self.builder.write().unwrap().leave_crunched_section();
    }
}
