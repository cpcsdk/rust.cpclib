use std::fmt::{Debug, Display};
use std::io::Write;
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use std::collections::{HashMap, HashSet};

use cpclib_common::itertools::Itertools;
use cpclib_common::smallvec::SmallVec;
use cpclib_tokens::ExprResult;
use cpclib_tokens::symbols::{MemoryPhysicalAddress, PhysicalAddress, SymbolsTable};

use crate::preamble::{LocatedToken, LocatedTokenInner, MayHaveSpan, SourceString};
/// Generate an output listing.
/// Can be useful to detect issues

#[derive(Clone, Debug)]
pub enum ListingAddressRadix {
    Hex,
    Dec
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListingSourceFileOutputMode {
    None,
    Header,
    FileMap
}

#[derive(Clone, Debug)]
pub struct ListingOutputFormat {
    pub bytes_per_line: usize,
    pub show_physical_address: bool,
    pub uppercase_hex: bool,
    pub address_radix: ListingAddressRadix,
    pub logical_address_width: usize,
    pub physical_address_width: usize,
    pub line_number_width: usize,
    pub show_line_numbers: bool,
    pub show_context_header: bool,
    pub listing_line_template: String,
    pub source_file_output_mode: ListingSourceFileOutputMode
}

impl Default for ListingOutputFormat {
    fn default() -> Self {
        Self {
            bytes_per_line: 8,
            show_physical_address: true,
            uppercase_hex: true,
            address_radix: ListingAddressRadix::Hex,
            logical_address_width: 4,
            physical_address_width: 5,
            line_number_width: 4,
            show_line_numbers: true,
            show_context_header: true,
            listing_line_template: "{A} {P} {C} {L4} {S}".to_string(),
            source_file_output_mode: ListingSourceFileOutputMode::Header
        }
    }
}

#[derive(PartialEq)]
pub enum TokenKind {
    Hidden,
    Label(String),
    Set(String),
    MacroCall,
    MacroDefine(String),
    Displayable
}

impl TokenKind {
    fn is_displayable(&self) -> bool {
        self == &TokenKind::Displayable
    }
}
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
    crunched_section_counter: usize,
    current_token_kind: TokenKind,
    deferred_for_line: Vec<String>,
    counter_update: Vec<String>,
    format: ListingOutputFormat,
    current_file_index: usize,
    file_indices: HashMap<String, usize>,
    file_order: Vec<String>,
    file_map_header_printed: bool
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
        Self::new_with_format(writer, ListingOutputFormat::default())
    }

    pub fn new_with_format<W: 'static + Write + Send + Sync>(
        writer: W,
        format: ListingOutputFormat
    ) -> Self {
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
            current_physical_address: MemoryPhysicalAddress::new(0, 0).into(),
            current_token_kind: TokenKind::Hidden,
            deferred_for_line: Default::default(),
            counter_update: Vec::new(),
            format,
            current_file_index: 0,
            file_indices: HashMap::new(),
            file_order: Vec::new(),
            file_map_header_printed: false
        }
    }

    pub fn set_format(&mut self, format: ListingOutputFormat) {
        self.format = format;
    }

    fn bytes_per_line(&self) -> usize {
        self.format.bytes_per_line.max(1)
    }

    fn logical_address_width(&self) -> usize {
        self.format.logical_address_width.max(1)
    }

    fn physical_address_width(&self) -> usize {
        self.format.physical_address_width.max(1)
    }

    fn physical_field_width(&self) -> usize {
        self.physical_address_width() + 1
    }

    fn format_address(&self, value: u32, width: usize) -> String {
        match self.format.address_radix {
            ListingAddressRadix::Hex => {
                if self.format.uppercase_hex {
                    format!("{:0width$X}", value, width = width)
                }
                else {
                    format!("{:0width$x}", value, width = width)
                }
            },
            ListingAddressRadix::Dec => format!("{:0width$}", value, width = width)
        }
    }

    fn blank(width: usize) -> String {
        " ".repeat(width)
    }

    fn register_source_file_name(&mut self, fname: &str) -> (usize, bool) {
        if let Some(index) = self.file_indices.get(fname) {
            return (*index, false);
        }

        let index = self.file_order.len();
        self.file_order.push(fname.to_string());
        self.file_indices.insert(fname.to_string(), index);
        (index, true)
    }

    fn format_bytes_raw(&self, bytes: &[u8]) -> String {
        bytes.iter().map(|b| self.hex_byte(*b)).join(" ")
    }

    fn format_bytes(&self, bytes: &[u8]) -> String {
        let rendered = self.format_bytes_raw(bytes);
        let full_width = self.bytes_per_line() * 3;
        format!("{rendered:<full_width$}")
    }

    fn format_bytes_x(&self, bytes: &[u8]) -> String {
        let rendered = self.format_bytes_raw(bytes);
        let fixed_width = self.bytes_per_line() * 3;
        format!("{rendered:<fixed_width$}")
    }

    fn format_line_with_template(
        &self,
        logical_address: Option<u32>,
        physical_address_repr: &str,
        bytes: &[u8],
        line_number: Option<u32>,
        source_line: Option<&str>
    ) -> String {
        let template = self.format.listing_line_template.as_str();
        let mut output = String::with_capacity(template.len() + 32);
        let mut consumed = HashSet::<String>::new();
        let mut chars = template.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch != '{' {
                output.push(ch);
                continue;
            }

            let mut placeholder = String::new();
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == '}' {
                    break;
                }
                placeholder.push(next);
            }

            if placeholder.is_empty() {
                output.push('{');
                output.push('}');
                continue;
            }

            if !consumed.insert(placeholder.clone()) {
                continue;
            }

            let rendered = match placeholder.as_str() {
                "A" => logical_address
                    .map(|value| self.format_address(value, self.logical_address_width()))
                    .unwrap_or_else(|| Self::blank(self.logical_address_width())),
                "P" => {
                    if self.format.show_physical_address {
                        physical_address_repr.to_string()
                    }
                    else {
                        String::new()
                    }
                },
                "C" => self.format_bytes(bytes),
                "CX" => self.format_bytes_x(bytes),
                "F" => self.current_file_index.to_string(),
                "F2" => format!("{:>2}", self.current_file_index),
                "F3" => format!("{:>3}", self.current_file_index),
                "L" => {
                    if self.format.show_line_numbers {
                        line_number.map(|value| value.to_string()).unwrap_or_default()
                    }
                    else {
                        String::new()
                    }
                },
                "L3" => {
                    if self.format.show_line_numbers {
                        line_number.map(|value| format!("{:>3}", value)).unwrap_or_default()
                    }
                    else {
                        String::new()
                    }
                },
                "L4" => {
                    if self.format.show_line_numbers {
                        line_number.map(|value| format!("{:>4}", value)).unwrap_or_default()
                    }
                    else {
                        String::new()
                    }
                },
                "L5" => {
                    if self.format.show_line_numbers {
                        line_number.map(|value| format!("{:>5}", value)).unwrap_or_default()
                    }
                    else {
                        String::new()
                    }
                },
                "S" => source_line.map(|line| line.trim_start().to_string()).unwrap_or_default(),
                _ => format!("{{{placeholder}}}")
            };

            output.push_str(&rendered);
        }

        output
    }

    fn format_deferred_line_with_template(
        &self,
        specific_content: &str,
        line_number: Option<u32>,
        source_line: &str
    ) -> String {
        let source_marker = "\u{1f}SOURCE\u{1f}";
        let rendered = self.format_line_with_template(
            None,
            &Self::blank(self.physical_field_width()),
            &[],
            line_number,
            Some(source_marker)
        );

        let anchor = if self.format.show_line_numbers {
            line_number
                .and_then(|value| rendered.find(&value.to_string()))
                .or_else(|| rendered.find(source_marker))
        }
        else {
            rendered.find(source_marker)
        };

        let rendered = rendered.replace(source_marker, source_line.trim_start());

        match anchor {
            Some(anchor) => {
                let suffix = &rendered[anchor..];
                format!("{specific_content:<anchor$}{suffix}")
            },
            None if rendered.trim().is_empty() => specific_content.to_string(),
            None if specific_content.is_empty() => rendered,
            None => format!("{specific_content} {rendered}")
        }
    }

    fn hex_byte(&self, b: u8) -> String {
        if self.format.uppercase_hex {
            format!("{b:02X}")
        }
        else {
            format!("{b:02x}")
        }
    }

    fn has_current_line_output(&self) -> bool {
        !self.current_line_bytes.is_empty() || self.current_token_kind.is_displayable()
    }

    fn current_token_specific_content(&self) -> Option<String> {
        match &self.current_token_kind {
            TokenKind::Hidden => None,
            TokenKind::Label(l) => Some(format!(
                "{} {} {l}",
                self.format_address(self.current_first_address, self.logical_address_width()),
                match self.current_physical_address {
                    PhysicalAddress::Memory(adr) => {
                        self.format_address(adr.offset_in_cpc(), self.physical_address_width())
                    },
                    PhysicalAddress::Bank(adr) => {
                        self.format_address(adr.address() as _, self.physical_address_width())
                    },
                    PhysicalAddress::Cpr(adr) => {
                        self.format_address(adr.address() as _, self.physical_address_width())
                    }
                }
            )),
            TokenKind::Set(label) => Some(format!(
                "{} {} {label}",
                self.format_address(self.current_first_address, self.logical_address_width()),
                "?".repeat(self.physical_address_width())
            )),
            TokenKind::MacroCall | TokenKind::Displayable => None,
            TokenKind::MacroDefine(name) => Some(format!("MACRO      {name}"))
        }
    }

    fn begin_current_line(&mut self, token: &LocatedToken, address: u32, physical_address: PhysicalAddress) {
        // keep the source pointer stable, but avoid copying the source string itself
        self.current_source = Some(unsafe { std::mem::transmute(token.context().complete_source()) });
        self.current_line_group = Some((token.span().location_line(), Self::extract_code(token)));
        self.current_first_address = address;
        self.current_physical_address = physical_address;
        self.current_address_kind = AddressKind::None;
    }

    fn append_current_line_bytes(&mut self, bytes: &[u8], address_kind: AddressKind) {
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
    }

    fn update_current_token_kind(&mut self, token: &LocatedToken, symbols: Option<*const SymbolsTable>) {
        self.current_token_kind = match token.deref() {
            LocatedTokenInner::Label(l) => {
                TokenKind::Label(Self::expand_listing_label(l.to_string(), symbols))
            },
            LocatedTokenInner::Equ { label, .. } | LocatedTokenInner::Assign { label, .. } => {
                TokenKind::Set(Self::expand_listing_label(label.to_string(), symbols))
            },
            LocatedTokenInner::Macro { name, .. } => TokenKind::MacroDefine(name.to_string()),
            LocatedTokenInner::MacroCall(..)
            | LocatedTokenInner::Org { .. }
            | LocatedTokenInner::Comment(..)
            | LocatedTokenInner::Include(..)
            | LocatedTokenInner::Repeat(..) => TokenKind::Displayable,
            _ => TokenKind::Hidden
        };
    }

    /// Check if the token is for the same source
    fn token_is_on_same_source(&self, token: &LocatedToken) -> bool {
        match &self.current_source {
            Some(current_source) => {
                std::ptr::eq(token.context().source.as_ptr(), current_source.as_ptr())
            },
            None => false
        }
    }

    /// Check if the token is for the same line than the previous token
    fn token_is_on_same_line(&self, token: &LocatedToken) -> bool {
        match &self.current_line_group {
            Some((current_location, _current_line)) => {
                self.token_is_on_same_source(token)
                    && *current_location == token.span().location_line()
            },
            None => false
        }
    }

    fn extract_code(token: &LocatedToken) -> String {
        match token {
            LocatedToken {
                inner: either::Left(LocatedTokenInner::Macro { .. } | LocatedTokenInner::Repeat(..)),
                span,
                ..
            } => {
                // 		self.need_to_cut = true;
                span.as_str().to_string()
            },

            _ => {
                // 			self.need_to_cut = false;
                unsafe {
                    std::str::from_utf8_unchecked(token.span().get_line_beginning().as_bytes())
                }
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
        physical_address: PhysicalAddress,
        symbols: Option<*const SymbolsTable>
    ) {
        if !self.activated {
            return;
        }

        // dbg!(token);

        let fname_handling = self.manage_fname(token);
        if let Some(specific_content) = self.current_token_specific_content() {
            self.deferred_for_line.push(specific_content.clone());
        }

        if !self.token_is_on_same_line(token) {
            self.process_current_line();
            self.begin_current_line(token, address, physical_address);
        }
        else {
            self.current_line_group = Some((token.span().location_line(), Self::extract_code(token)));
        }

        self.append_current_line_bytes(bytes, address_kind);

        if let Some(line) = fname_handling {
            writeln!(self.writer, "{line}").unwrap();
        }

        self.update_current_token_kind(token, symbols);
    }

    fn expand_listing_label(raw_label: String, symbols: Option<*const SymbolsTable>) -> String {
        // Listing expansion is intentionally done only in listing pass to avoid runtime overhead.
        symbols
            .and_then(|symbols| {
                let symbols = unsafe { symbols.as_ref() }?;
                symbols
                    .extend_local_and_patterns_for_symbol(&raw_label)
                    .ok()
                    .map(|symbol| symbol.to_string())
            })
            .unwrap_or(raw_label)
    }

    pub fn process_current_line(&mut self) {
        // retrieve the line
        let (line_number, line) = match &self.current_line_group {
            Some((idx, line)) => (idx, line),
            None => return
        };

        // build the line representation for source and generated bytes
        let line_representation = line.split('\n').collect_vec();
        let data_chunks = self
            .current_line_bytes
            .chunks(self.bytes_per_line())
            .collect_vec();
        let data_representation = data_chunks
            .iter()
            .map(|chunk| chunk.iter().map(|b| self.hex_byte(*b)).join(" "))
            .collect_vec();

        // TODO manage missing end of files/blocks if needed

        let delta = line_representation.len();
        // TODO add the line representation ?
        for specific_content in self.deferred_for_line.iter() {
            let lines = line.split('\n').collect_vec();
            let lines_count = lines.len(); // line number corresponds to the VERY LAST line and not the FIRST one
            for (line_delta, line) in lines.into_iter().enumerate() {
                let rendered = self.format_deferred_line_with_template(
                    if line_delta == 0 { specific_content } else { "" },
                    Some(line_number + delta as u32 + line_delta as u32 - lines_count as u32),
                    line
                );
                writeln!(self.writer, "{rendered}").unwrap();
            }
        }
        self.deferred_for_line.clear();

        // draw all lines that correspond to the instructions to output
        let mut byte_offset = 0usize;
        let render_lines = line_representation.len().max(data_representation.len());
        for idx in 0..render_lines {
            let current_inner_line = line_representation.get(idx).copied();
            let current_inner_data = data_representation.get(idx);
            let current_chunk = data_chunks.get(idx).copied().unwrap_or(&[]);
            let current_data_len = data_chunks.get(idx).map(|chunk| chunk.len()).unwrap_or(0);

            let logical_address = self.current_first_address.wrapping_add(byte_offset as u32);
            let logical_representation = if current_inner_line.is_none() && current_inner_data.is_none() {
                None
            }
            else {
                Some(logical_address)
            };

            // Physical address is printed when enabled and relevant.
            let base_offset = match self.current_physical_address {
                PhysicalAddress::Memory(adr) => adr.offset_in_cpc(),
                PhysicalAddress::Bank(adr) => adr.address() as _,
                PhysicalAddress::Cpr(adr) => adr.address() as _
            };
            let current_offset = base_offset.wrapping_add(byte_offset as u32);
            let phys_addr_representation = if !self.format.show_physical_address {
                Self::blank(self.physical_field_width())
            }
            else if current_inner_line.is_none() && current_inner_data.is_none() {
                Self::blank(self.physical_field_width())
            }
            else if current_offset == logical_address && self.current_address_kind == AddressKind::Address {
                Self::blank(self.physical_field_width())
            }
            else {
                format!(
                    "{}{}",
                    self.format_address(current_offset, self.physical_address_width()),
                    self.current_address_kind
                )
            };
            let rendered_line_number = current_inner_line.map(|_| line_number + idx as u32);

            // missing instruction must be added manually using TokenKind
            if self.has_current_line_output() {
                let fallback_source = current_inner_line.map(|line| line.trim_end()).unwrap_or("");
                let fallback_bytes = current_inner_data.cloned().unwrap_or_default();
                let rendered = self.format_line_with_template(
                    logical_representation,
                    &phys_addr_representation,
                    current_chunk,
                    rendered_line_number,
                    Some(fallback_source)
                );
                if rendered.trim().is_empty() {
                    writeln!(
                        self.writer,
                        "{} {} {:bytes_width$} {}",
                        logical_representation
                            .map(|value| self.format_address(value, self.logical_address_width()))
                            .unwrap_or_else(|| Self::blank(self.logical_address_width())),
                        phys_addr_representation,
                        fallback_bytes,
                        fallback_source,
                        bytes_width = self.bytes_per_line() * 3
                    )
                    .unwrap();
                }
                else {
                    writeln!(self.writer, "{rendered}").unwrap();
                }
            }

            byte_offset += current_data_len;
        }

        if self.has_current_line_output() {
            for counter in self.counter_update.iter() {
                self.writer
                    .write_all(format!("{counter}\n").as_bytes())
                    .unwrap();
            }
            self.counter_update.clear();
        }

        // cleanup all the fields of the current line
        self.current_line_group = None;
        self.current_source = None;
        self.current_line_bytes.clear();
    }

    pub fn finish(&mut self) {
        self.process_current_line();
        if !self.deferred_for_line.is_empty() {
            panic!()
        }
    }

    /// Print filename if needed
    pub fn manage_fname(&mut self, token: &LocatedToken) -> Option<String> {
        // 	dbg!(token);

        let ctx = &token.span().state;
        let fname = ctx
            .filename()
            .map(|p| p.as_os_str().to_str().unwrap_or("<NO FNAME>").to_string())
            .or_else(|| ctx.context_name().map(|s| s.to_owned()));

        match fname {
            Some(fname) => {
                let (file_index, is_new_file) = self.register_source_file_name(&fname);
                self.current_file_index = file_index;

                if self.format.source_file_output_mode == ListingSourceFileOutputMode::None {
                    self.current_fname = Some(fname);
                    return None;
                }

                if self.format.source_file_output_mode == ListingSourceFileOutputMode::FileMap {
                    self.current_fname = Some(fname.clone());

                    if !self.file_map_header_printed {
                        self.file_map_header_printed = true;
                        return Some(format!("Source file map:\n  [{file_index}] {fname}"));
                    }

                    if is_new_file {
                        return Some(format!("  [{file_index}] {fname}"));
                    }

                    return None;
                }

                if !self.format.show_context_header {
                    self.current_fname = Some(fname);
                    return None;
                }

                let print = match self.current_fname.as_ref() {
                    Some(current_fname) => *current_fname != fname,
                    None => true
                };

                if print {
                    self.current_fname = Some(fname.clone());
                    Some(format!("Context [{file_index}]: {fname}"))
                }
                else {
                    None
                }
            },
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

unsafe impl Send for ListingOutputTrigger {}
unsafe impl Sync for ListingOutputTrigger {}

/// This structure collects the necessary information to feed the output
#[derive(Clone)]
pub struct ListingOutputTrigger {
    /// the token read before collecting the bytes
    /// Because each token can have a different lifespan, we store them using a pointer
    pub(crate) token: Option<*const LocatedToken>,
    /// the bytes progressively collected
    pub(crate) bytes: Vec<u8>,
    pub(crate) symbols: Option<*const SymbolsTable>,
    pub(crate) start: u32,
    pub(crate) physical_address: PhysicalAddress,
    pub(crate) builder: Arc<RwLock<ListingOutput>>
}

impl ListingOutputTrigger {
    pub fn write_byte(&mut self, b: u8) {
        self.bytes.push(b);
    }

    pub fn new_token(
        &mut self,
        new: *const LocatedToken,
        code: u32,
        kind: AddressKind,
        physical_address: PhysicalAddress,
        symbols: Option<*const SymbolsTable>
    ) {
        // Retreive the previous token and handle it
        if let Some(token) = &self.token {
            self.builder.write().unwrap().add_token(
                unsafe { &**token },
                &self.bytes,
                self.start,
                kind,
                self.physical_address,
                self.symbols
            );
        }

        self.token.replace(new); // TODO remove that clone that is memory/time eager
        self.symbols = symbols;

        // TODO double check if these lines are current. I doubt it is the case when having severl instructions per line
        self.bytes.clear();
        self.start = code;
        self.physical_address = physical_address;
    }

    /// Override the address value by the expression result
    /// BUGGY when it is not a number ...
    pub fn replace_code_address(&mut self, address: &ExprResult) {
        Self::result_to_address(address).map(|a| self.start = a);
    }

    /// Applies the conversion when possible
    fn result_to_address(address: &ExprResult) -> Option<u32> {
        match address {
            ExprResult::Float(_f) => None,
            ExprResult::Value(v) => Some(*v as _),
            ExprResult::Char(v) => Some(*v as _),
            ExprResult::Bool(b) => Some(if *b { 1 } else { 0 }),
            ExprResult::String(s) => Some(s.len() as _),
            ExprResult::List(l) => Some(l.len() as _),
            ExprResult::Matrix {
                width,
                height,
                content: _
            } => Some((*width * *height) as _)
        }
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
                self.physical_address,
                self.symbols
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

    pub fn repeat_iteration(&mut self, counter: &str, value: Option<&ExprResult>) {
        let line = if let Some(value) = value {
            let value = Self::result_to_address(value);
            if let Some(value) = value {
                format!("{value:04X} ????? {counter}")
            }
            else {
                format!("???? ????? {counter}")
            }
        }
        else {
            format!("???? ???? {counter}")
        };

        self.builder.write().unwrap().counter_update.push(line);
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    use cpclib_tokens::symbols::MemoryPhysicalAddress;

    use super::{
        ListingAddressRadix, ListingOutput, ListingOutputFormat, ListingSourceFileOutputMode,
        TokenKind
    };

    #[derive(Clone, Default)]
    struct SharedBufferWriter {
        content: Arc<Mutex<Vec<u8>>>
    }

    impl SharedBufferWriter {
        fn snapshot(&self) -> String {
            String::from_utf8(self.content.lock().unwrap().clone()).unwrap()
        }
    }

    impl Write for SharedBufferWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.content.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn process_current_line_renders_bytes_and_source_line() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new(writer.clone());
        output.on();

        output.current_line_group = Some((12, "    ld a,0x12".to_string()));
        output.current_line_bytes.extend_from_slice(&[0x3E, 0x12]);
        output.current_first_address = 0x100;
        output.current_physical_address = MemoryPhysicalAddress::new(0x100, 0).into();
        output.current_token_kind = TokenKind::Displayable;

        output.process_current_line();

        let listing = writer.snapshot();
        assert!(listing.contains("0100"), "listing={listing}");
        assert!(listing.contains("3E 12"), "listing={listing}");
        assert!(listing.contains("ld a,0x12"), "listing={listing}");
    }

    #[test]
    fn process_current_line_renders_deferred_and_counter_updates() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new(writer.clone());
        output.on();

        output.current_line_group = Some((20, "label: nop".to_string()));
        output.current_first_address = 0x200;
        output.current_physical_address = MemoryPhysicalAddress::new(0x200, 0).into();
        output.current_token_kind = TokenKind::Displayable;
        output.deferred_for_line.push("0200 00200 my_label".to_string());
        output.counter_update.push("0201 ????? <new iteration>".to_string());

        output.process_current_line();

        let listing = writer.snapshot();
        assert!(listing.contains("my_label"), "listing={listing}");
        assert!(listing.contains("label: nop"), "listing={listing}");
        assert!(listing.contains("<new iteration>"), "listing={listing}");
    }

    #[test]
    fn process_current_line_aligns_deferred_lines_with_template_columns() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new(writer.clone());
        output.on();

        output.current_line_group = Some((30, "macro SWITCH_VALUES addr1, addr2\n\tld hl, ({addr1})".to_string()));
        output.current_first_address = 0x4000;
        output.current_physical_address = MemoryPhysicalAddress::new(0x4000, 0).into();
        output.current_token_kind = TokenKind::Displayable;
        output
            .deferred_for_line
            .push("MACRO      SWITCH_VALUES".to_string());

        output.process_current_line();

        let listing = writer.snapshot();
        assert!(listing.contains("MACRO      SWITCH_VALUES               30 macro SWITCH_VALUES addr1, addr2"), "listing={listing}");
        assert!(listing.contains("                                       31 ld hl, ({addr1})"), "listing={listing}");
    }

    #[test]
    fn process_current_line_renders_continuation_addresses_for_long_byte_sequences() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new(writer.clone());
        output.on();

        output.current_line_group = Some((30, "db 0,1,2,3,4,5,6,7,8,9".to_string()));
        output.current_line_bytes
            .extend_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        output.current_first_address = 0x4000;
        output.current_physical_address = MemoryPhysicalAddress::new(0x4000, 0).into();
        output.current_token_kind = TokenKind::Displayable;

        output.process_current_line();

        let listing = writer.snapshot();
        assert!(listing.contains("4000"), "listing={listing}");
        assert!(listing.contains("4008"), "listing={listing}");
        assert!(listing.contains("00 01 02 03 04 05 06 07"), "listing={listing}");
        assert!(listing.contains("08 09"), "listing={listing}");
    }

    #[test]
    fn process_current_line_honors_custom_format_bytes_per_line_and_hex_case() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new_with_format(
            writer.clone(),
            ListingOutputFormat {
                bytes_per_line: 4,
                show_physical_address: false,
                uppercase_hex: false,
                ..Default::default()
            }
        );
        output.on();

        output.current_line_group = Some((40, "db 0xAA,0xBB,0xCC,0xDD,0xEE".to_string()));
        output
            .current_line_bytes
            .extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE]);
        output.current_first_address = 0x5000;
        output.current_physical_address = MemoryPhysicalAddress::new(0x5000, 0).into();
        output.current_token_kind = TokenKind::Displayable;

        output.process_current_line();

        let listing = writer.snapshot();
        assert!(listing.contains("aa bb cc dd"), "listing={listing}");
        assert!(listing.contains("ee"), "listing={listing}");
        assert!(listing.contains("5004"), "listing={listing}");
    }

    #[test]
    fn process_current_line_honors_decimal_radix_and_hidden_line_numbers() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new_with_format(
            writer.clone(),
            ListingOutputFormat {
                bytes_per_line: 2,
                show_physical_address: false,
                uppercase_hex: false,
                address_radix: ListingAddressRadix::Dec,
                logical_address_width: 5,
                physical_address_width: 5,
                line_number_width: 3,
                show_line_numbers: false,
                show_context_header: false,
                listing_line_template: "{A} {C} {S}".to_string(),
                source_file_output_mode: ListingSourceFileOutputMode::None
            }
        );
        output.on();

        output.current_line_group = Some((7, "db 1,2,3".to_string()));
        output.current_line_bytes.extend_from_slice(&[1, 2, 3]);
        output.current_first_address = 100;
        output.current_physical_address = MemoryPhysicalAddress::new(100, 0).into();
        output.current_token_kind = TokenKind::Displayable;

        output.process_current_line();

        let listing = writer.snapshot();
        assert!(listing.contains("00100"), "listing={listing}");
        assert!(listing.contains("00102"), "listing={listing}");
        assert!(listing.contains("01 02"), "listing={listing}");
        assert!(listing.contains("03"), "listing={listing}");
        assert!(!listing.contains("  7 db"), "listing={listing}");
    }

    #[test]
    fn process_current_line_template_ignores_duplicate_placeholders() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new_with_format(
            writer.clone(),
            ListingOutputFormat {
                listing_line_template: "{A}|{A}|{L4}|{L4}|{S}|{S}".to_string(),
                source_file_output_mode: ListingSourceFileOutputMode::None,
                ..Default::default()
            }
        );
        output.on();

        output.current_line_group = Some((9, "  nop".to_string()));
        output.current_line_bytes.extend_from_slice(&[0x00]);
        output.current_first_address = 0x4321;
        output.current_physical_address = MemoryPhysicalAddress::new(0x4321, 0).into();
        output.current_token_kind = TokenKind::Displayable;

        output.process_current_line();

        let listing = writer.snapshot();
        assert!(listing.contains("4321||   9||nop|"), "listing={listing}");
    }

    #[test]
    fn process_current_line_cx_keeps_line_number_column_aligned() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new_with_format(
            writer.clone(),
            ListingOutputFormat {
                bytes_per_line: 8,
                listing_line_template: "{A} {CX} {L4} {S}".to_string(),
                source_file_output_mode: ListingSourceFileOutputMode::None,
                ..Default::default()
            }
        );
        output.on();

        output.current_line_group = Some((45, "ld bc, 0xbc00 + 13 : out (c),c : ld bc, 0xbd00 : out (c), l".to_string()));
        output
            .current_line_bytes
            .extend_from_slice(&[0x01, 0x0D, 0xBC, 0xED, 0x49, 0x01, 0x00, 0xBD]);
        output.current_first_address = 0x403C;
        output.current_physical_address = MemoryPhysicalAddress::new(0x403C, 0).into();
        output.current_token_kind = TokenKind::Displayable;
        output.process_current_line();

        output.current_line_group = Some((50, "ld sp, $".to_string()));
        output.current_line_bytes.extend_from_slice(&[0x31, 0x55, 0x40]);
        output.current_first_address = 0x4055;
        output.current_physical_address = MemoryPhysicalAddress::new(0x4055, 0).into();
        output.current_token_kind = TokenKind::Displayable;
        output.process_current_line();

        let listing = writer.snapshot();
        let lines: Vec<_> = listing.lines().collect();
        let line_45 = lines
            .iter()
            .find(|line| line.contains("  45 "))
            .copied()
            .unwrap_or_default();
        let line_50 = lines
            .iter()
            .find(|line| line.contains("  50 "))
            .copied()
            .unwrap_or_default();

        let idx_45 = line_45.find("  45 ").unwrap_or(usize::MAX);
        let idx_50 = line_50.find("  50 ").unwrap_or(usize::MAX);
        assert_eq!(idx_45, idx_50, "listing={listing}");
    }
}
