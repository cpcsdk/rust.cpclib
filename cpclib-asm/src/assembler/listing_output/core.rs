use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::io::Write;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

use cpclib_common::itertools::Itertools;
use cpclib_common::smallvec::SmallVec;
use cpclib_tokens::ExprResult;
use cpclib_tokens::symbols::{
    MemoryPhysicalAddress, PhysicalAddress, SymbolsTable, SymbolsTableTrait, Value
};

use super::format::*;
use super::render::*;
use crate::preamble::{LocatedToken, LocatedTokenInner, MayHaveSpan, SourceString};

#[derive(Clone, PartialEq)]
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

#[derive(Clone)]
struct ListingTokenItem {
    token_id: usize,
    raw: String,
    expanded: String,
    bytes: Vec<u8>,
    token_kind: TokenKind,
    /// 1-based column where this token starts on its source line, and where it
    /// ends.
    ///
    /// A line is often several instructions - `ld a,l : inc a : ld (.p),a` is
    /// three - and a debugger that can only say "line 42" puts the cursor at
    /// the start of all three. These are what let it point at the one that is
    /// actually executing.
    column: u16,
    column_end: u16,
    /// Whether this token is a data directive (`db`/`defs`/`defw`/`incbin`/a
    /// string), as opposed to an instruction. Kept separate from `token_kind`,
    /// which already collapses both onto `Hidden` for unrelated formatting
    /// reasons - folding this in would break that collapse.
    is_data: bool
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
    /// Line number and raw/expanded line content.
    current_line_group: Option<(u32, String, String)>, /* clone view of the line XXX avoid this clone */

    current_first_address: u32,
    current_address_kind: AddressKind,
    current_physical_address: PhysicalAddress,
    crunched_section_counter: usize,
    current_token_kind: TokenKind,
    /// Whether the token currently being accumulated is a data directive -
    /// see `ListingTokenItem::is_data`.
    current_token_is_data: bool,
    current_token_bytes: Vec<u8>,
    current_token_raw: String,
    current_token_expanded: String,
    current_line_tokens: Vec<ListingTokenItem>,
    /// Where the token currently being accumulated starts and ends on its line.
    current_token_column: u16,
    current_token_column_end: u16,
    next_token_id: usize,
    deferred_for_line: Vec<String>,
    counter_update: Vec<String>,
    delayed_counter_update: Vec<String>,
    pending_iteration_notices: Vec<String>,
    format: ListingOutputFormat,
    renderer: ListingRenderer,
    current_file_index: usize,
    /// Collects `(file, line, address, length)` alongside the rendered
    /// listing when a caller asked for a source map. Independent of the
    /// writer: both, either or neither can be wanted.
    source_map: Option<super::SourceMapCollector>,
    /// Nobody is reading the rendered listing, so do not render it.
    ///
    /// Asking for a source map installs one of these over `io::sink()`, which
    /// used to format every line of the program - twice per line, character by
    /// character, through `qualify_locals_in_line` - and throw the result
    /// away. On a real demo that is tens of megabytes of text nobody will ever
    /// see, and it dominated the time to start a debug session.
    map_only: bool,
    /// Where in the source the last token of the current line group started,
    /// so a re-executed token (a new `REPEAT` iteration) is recognised rather
    /// than glued onto the previous one. See `token_is_on_same_line`.
    current_line_last_offset: Option<usize>,
    file_indices: HashMap<String, usize>,
    file_order: Vec<String>,
    file_map_header_printed: bool,
    listing_current_global_label: Option<String>,
    repeat_depth: usize
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
        let renderer = ListingRenderer::from_format(&format);
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
            current_token_is_data: false,
            current_token_bytes: Vec::new(),
            current_token_raw: String::new(),
            current_token_expanded: String::new(),
            current_line_tokens: Vec::new(),
            current_token_column: 1,
            current_token_column_end: 1,
            next_token_id: 0,
            deferred_for_line: Default::default(),
            counter_update: Vec::new(),
            delayed_counter_update: Vec::new(),
            pending_iteration_notices: Vec::new(),
            format,
            renderer,
            current_file_index: 0,
            source_map: None,
            map_only: false,
            current_line_last_offset: None,
            file_indices: HashMap::new(),
            file_order: Vec::new(),
            file_map_header_printed: false,
            listing_current_global_label: None,
            repeat_depth: 0
        }
    }

    pub fn set_format(&mut self, format: ListingOutputFormat) {
        self.format = format;
        self.renderer = ListingRenderer::from_format(&self.format);
    }

    fn bytes_per_line(&self) -> usize {
        self.format.bytes_per_line.max(1)
    }

    fn logical_address_width(&self) -> usize {
        logical_address_width(&self.format)
    }

    fn physical_address_width(&self) -> usize {
        physical_address_width(&self.format)
    }

    fn physical_field_width(&self) -> usize {
        physical_field_width(&self.format)
    }

    fn format_address(&self, value: u32, width: usize) -> String {
        format_address_for(&self.format, value, width)
    }

    fn blank(width: usize) -> String {
        blank(width)
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

    #[cfg(test)]
    fn format_bytes_raw(&self, bytes: &[u8]) -> String {
        format_bytes_raw_for(&self.format, bytes)
    }

    #[cfg(test)]
    fn format_bytes(&self, bytes: &[u8]) -> String {
        format_bytes_for(&self.format, self.bytes_per_line(), bytes)
    }

    #[cfg(test)]
    fn format_bytes_x(&self, bytes: &[u8]) -> String {
        format_bytes_for(&self.format, self.bytes_per_line(), bytes)
    }

    #[cfg(test)]
    fn render_source_column(line: Option<&str>) -> String {
        render_source_column(line)
    }

    #[cfg(test)]
    fn format_line_with_template(
        &self,
        logical_address: Option<u32>,
        physical_address_repr: &str,
        bytes: &[u8],
        line_number: Option<u32>,
        source_line_raw: Option<&str>,
        source_line_expanded: Option<&str>
    ) -> String {
        format_line_with_template_for(
            &self.format,
            self.bytes_per_line(),
            self.current_file_index,
            logical_address,
            physical_address_repr,
            bytes,
            line_number,
            source_line_raw,
            source_line_expanded
        )
    }

    #[cfg(test)]
    fn format_deferred_line_with_template(
        &self,
        specific_content: &str,
        line_number: Option<u32>,
        source_line_raw: &str,
        source_line_expanded: &str
    ) -> Vec<String> {
        format_deferred_line_with_template_for(
            &self.format,
            self.bytes_per_line(),
            self.current_file_index,
            specific_content,
            line_number,
            source_line_raw,
            source_line_expanded
        )
    }

    fn hex_byte(&self, b: u8) -> String {
        hex_byte_for(&self.format, b)
    }

    fn has_current_line_output(&self) -> bool {
        !self.current_line_bytes.is_empty() || self.current_token_kind.is_displayable()
    }

    fn current_token_specific_content(&self) -> Option<String> {
        match &self.current_token_kind {
            TokenKind::Hidden => None,
            TokenKind::Label(l) => {
                Some(format!(
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
                ))
            },
            TokenKind::Set(label) => {
                Some(format!(
                    "{} {} {label}",
                    self.format_address(self.current_first_address, self.logical_address_width()),
                    "?".repeat(self.physical_address_width())
                ))
            },
            TokenKind::MacroCall | TokenKind::Displayable => None,
            TokenKind::MacroDefine(name) => Some(format!("MACRO      {name}"))
        }
    }

    fn should_expand_source_for_token(token: &LocatedToken) -> bool {
        !matches!(
            token.deref(),
            LocatedTokenInner::Repeat(..)
                | LocatedTokenInner::RepeatToken { .. }
                | LocatedTokenInner::RepeatUntil(..)
        )
    }

    fn update_repeat_depth(&mut self, token: &LocatedToken) {
        match token.deref() {
            LocatedTokenInner::Repeat(..) | LocatedTokenInner::RepeatUntil(..) => {
                self.repeat_depth = self.repeat_depth.saturating_add(1);
            },
            LocatedTokenInner::RepeatToken { .. } => {
                // RepeatToken represents emitted code from repeat expansion.
                // Keep depth untouched and rely on direct token check for raw rendering.
            },
            _ => {
                let lower = token.span().as_str().trim().to_ascii_lowercase();
                if lower == "endr" || lower == "endrepeat" {
                    self.repeat_depth = self.repeat_depth.saturating_sub(1);
                }
            }
        }
    }

    fn begin_current_line(
        &mut self,
        token: &LocatedToken,
        address: u32,
        physical_address: PhysicalAddress,
        symbols: Option<*const SymbolsTable>
    ) {
        // keep the source pointer stable, but avoid copying the source string itself
        self.current_source =
            Some(unsafe { std::mem::transmute(token.context().complete_source()) });
        let raw_line = Self::extract_code(token);
        let expanded_line = if !Self::should_expand_source_for_token(token) {
            raw_line.clone()
        }
        else {
            Self::expand_source_line_patterns(&raw_line, symbols)
        };
        self.current_line_group = Some((token.span().location_line(), raw_line, expanded_line));
        self.current_first_address = address;
        self.current_physical_address = physical_address;
        self.current_address_kind = AddressKind::None;
        self.current_line_tokens.clear();
        self.current_token_raw.clear();
        self.current_token_expanded.clear();
        self.current_token_bytes.clear();
    }

    fn append_current_line_bytes(&mut self, bytes: &[u8], address_kind: AddressKind) {
        self.current_line_bytes.extend_from_slice(bytes);
        self.current_token_bytes.extend_from_slice(bytes);
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

    fn update_current_token_kind(
        &mut self,
        token: &LocatedToken,
        symbols: Option<*const SymbolsTable>
    ) {
        self.current_token_kind = match token.deref() {
            LocatedTokenInner::Label(l) => {
                let raw_label = l.to_string();
                let expanded_label = self.expand_listing_label(&raw_label, symbols);
                if !raw_label.starts_with('.') && !raw_label.starts_with('@') {
                    self.listing_current_global_label = Some(expanded_label.clone());
                }
                TokenKind::Label(expanded_label)
            },
            LocatedTokenInner::Equ { label, .. } | LocatedTokenInner::Assign { label, .. } => {
                TokenKind::Set(self.expand_listing_label(label.as_ref(), symbols))
            },
            LocatedTokenInner::Macro { name, .. } => TokenKind::MacroDefine(name.to_string()),
            LocatedTokenInner::MacroCall(..)
            | LocatedTokenInner::Org { .. }
            | LocatedTokenInner::Bank(..)
            | LocatedTokenInner::Bankset(..)
            | LocatedTokenInner::Comment(..)
            | LocatedTokenInner::Include(..)
            | LocatedTokenInner::Repeat(..) => TokenKind::Displayable,
            _ => TokenKind::Hidden
        };
        // A sibling classification, not folded into `token_kind` above: that
        // enum already collapses data directives and opcodes onto the same
        // `Hidden` arm for unrelated formatting reasons, so reusing it here
        // would make data indistinguishable from code again.
        //
        // `Defs` is deliberately excluded: unlike a string table or `db`
        // list, which are never meant to execute, a `defs` region genuinely
        // executes every frame (it's just a shorthand for a run of zero
        // bytes, which decode as `NOP`). Folding it into the data overlay
        // would hide that from `-dv`. Step-over already treats a `defs` run
        // the same way, as a repetition of NOPs rather than inert data.
        self.current_token_is_data = matches!(
            token.deref(),
            LocatedTokenInner::Defb(..)
                | LocatedTokenInner::Defw(..)
                | LocatedTokenInner::Incbin { .. }
                | LocatedTokenInner::Str(..)
        );
    }

    fn flush_current_token(&mut self) {
        if self.current_line_group.is_none() {
            self.current_token_bytes.clear();
            self.current_token_raw.clear();
            self.current_token_expanded.clear();
            return;
        }

        if self.current_token_raw.is_empty()
            && self.current_token_expanded.is_empty()
            && self.current_token_bytes.is_empty()
        {
            return;
        }

        self.current_line_tokens.push(ListingTokenItem {
            token_id: self.next_token_id,
            raw: self.current_token_raw.clone(),
            expanded: self.current_token_expanded.clone(),
            bytes: self.current_token_bytes.clone(),
            token_kind: self.current_token_kind.clone(),
            column: self.current_token_column,
            column_end: self.current_token_column_end,
            is_data: self.current_token_is_data
        });
        self.next_token_id = self.next_token_id.saturating_add(1);
        self.current_token_bytes.clear();
        self.current_token_raw.clear();
        self.current_token_expanded.clear();
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
    /// Whether `token` continues the line currently being accumulated.
    ///
    /// Being on the same line is necessary but not sufficient. A `REPEAT` body
    /// re-executes the *same* tokens, so iteration two arrives on the same line
    /// as iteration one and would silently extend its row - a listing showing
    /// one line with the bytes of three iterations glued together, and a source
    /// map with one address where there should be three. Source position going
    /// backwards (or standing still) is what distinguishes re-execution from a
    /// genuine multi-statement line like `nop : nop : nop`, whose statements do
    /// advance.
    fn token_is_on_same_line(&self, token: &LocatedToken) -> bool {
        match &self.current_line_group {
            Some((current_location, _current_line, _current_line_expanded)) => {
                self.token_is_on_same_source(token)
                    && *current_location == token.span().location_line()
                    && self
                        .current_line_last_offset
                        .is_none_or(|last| token.span().offset_from_start() > last)
            },
            None => false
        }
    }

    fn extract_code(token: &LocatedToken) -> String {
        if matches!(
            token.deref(),
            LocatedTokenInner::Macro { .. }
                | LocatedTokenInner::MacroCall(..)
                | LocatedTokenInner::Basic(..)
                | LocatedTokenInner::Repeat(..)
        ) {
            // keep complete multiline source for macro/repeat style constructs
            token.span().as_str().to_string()
        }
        else {
            unsafe { std::str::from_utf8_unchecked(token.span().get_line_beginning().as_bytes()) }
                .to_owned()
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

        self.flush_current_token();

        if let Some(specific_content) = self.current_token_specific_content() {
            self.deferred_for_line.push(specific_content.clone());
        }

        if !self.token_is_on_same_line(token) {
            self.process_current_line();
            self.manage_fname(token);
            self.begin_current_line(token, address, physical_address, symbols);
        }
        else {
            self.manage_fname(token);
            let raw_line = Self::extract_code(token);
            let expanded_line = if !Self::should_expand_source_for_token(token) {
                raw_line.clone()
            }
            else {
                Self::expand_source_line_patterns(&raw_line, symbols)
            };
            self.current_line_group = Some((token.span().location_line(), raw_line, expanded_line));
        }

        self.current_line_last_offset = Some(token.span().offset_from_start());
        self.update_current_token_kind(token, symbols);
        self.current_token_raw = token.span().as_str().to_string();
        // Where this token sits on its line. `relative_line_and_column` is the
        // same computation the parser uses for error reporting, so the columns
        // agree with what a diagnostic would point at.
        let (column, column_end) = Self::source_columns(token, &self.current_token_raw);
        self.current_token_column = column;
        self.current_token_column_end = column_end;
        self.current_token_expanded = if !Self::should_expand_source_for_token(token) {
            self.current_token_raw.clone()
        }
        else {
            Self::expand_source_line_patterns(&self.current_token_raw, symbols)
        };
        self.current_token_bytes.clear();
        self.append_current_line_bytes(bytes, address_kind);
        self.update_repeat_depth(token);
    }

    /// The columns a token occupies, in the file the user has open.
    ///
    /// Outside an expansion the span's own columns are the file's, and columns
    /// count bytes from the start of the line, so the token's byte length is
    /// its width.
    ///
    /// Inside one they are not: a macro body is substituted textually and
    /// re-parsed, so `({addr1})` has become `(0xc000)` and everything after it
    /// on that line has moved. Only the map built while substituting can put
    /// them back. Without it - an orgams-flavor macro, a struct - the whole
    /// line is recorded instead of a guess, because a column pointing at the
    /// wrong instruction is worse than one pointing at all of them, and one
    /// past the end of the line selects nothing at all.
    fn source_columns(token: &LocatedToken, raw: &str) -> (u16, u16) {
        /// `column_end` no greater than `column` is how a row says it has no
        /// columns worth trusting; the debugger then selects the line.
        const WHOLE_LINE: (u16, u16) = (1, 1);

        let span = token.span();
        let (_, column) = span.relative_line_and_column();
        let column = column.max(1);
        let width = raw.trim_end().len();

        let context = span.context();
        if !context.is_expansion() {
            let start = column as u16;
            return (start, start.saturating_add(width as u16));
        }

        context
            .expansion_columns()
            .and_then(|columns| columns.source_columns(span.offset_from_start(), column, width))
            .unwrap_or(WHOLE_LINE)
    }

    fn expand_symbol_with_listing_context(
        raw: &str,
        symbols: Option<*const SymbolsTable>
    ) -> String {
        // Listing expansion is intentionally done only in listing pass to avoid runtime overhead.
        symbols
            .and_then(|symbols| {
                let symbols = unsafe { symbols.as_ref() }?;
                symbols
                    .extend_local_and_patterns_for_symbol(raw)
                    .ok()
                    .map(|symbol| symbol.to_string())
            })
            .unwrap_or_else(|| raw.to_string())
    }

    fn expand_source_line_patterns(raw_line: &str, symbols: Option<*const SymbolsTable>) -> String {
        let normalized = raw_line.replace("{{", "{").replace("}}", "}");

        // Keep string literals untouched. Expanding `.` / local placeholders inside quoted
        // text corrupts glyph-like data such as "###." and ".#.." in macro arguments.
        let has_string_literal = normalized.chars().any(|ch| ch == '"' || ch == '\'');

        // First keep the fast path for lines that can be expanded as a whole.
        if !has_string_literal {
            let whole = Self::expand_symbol_with_listing_context(&normalized, symbols);
            if whole != normalized {
                return Self::expand_embedded_braces(&whole, symbols);
            }
        }

        // Fallback: expand each lexical chunk separately so inline expressions like
        // `jr z, .has_nops{nb_nops}` are resolved even when full-line expansion fails.
        let mut out = String::with_capacity(normalized.len());
        let mut current = String::new();
        let mut in_string: Option<char> = None;
        let mut prev_escape = false;

        let flush_chunk = |chunk: &mut String, dst: &mut String| {
            if chunk.is_empty() {
                return;
            }
            let expanded = Self::expand_symbol_with_listing_context(chunk, symbols);
            dst.push_str(&expanded);
            chunk.clear();
        };

        for ch in normalized.chars() {
            if let Some(quote) = in_string {
                out.push(ch);
                if prev_escape {
                    prev_escape = false;
                    continue;
                }
                if ch == '\\' {
                    prev_escape = true;
                    continue;
                }
                if ch == quote {
                    in_string = None;
                }
                continue;
            }

            if ch == '"' || ch == '\'' {
                flush_chunk(&mut current, &mut out);
                out.push(ch);
                in_string = Some(ch);
                prev_escape = false;
                continue;
            }

            let chunk_char =
                ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '@' | '{' | '}');

            if chunk_char {
                current.push(ch);
            }
            else {
                flush_chunk(&mut current, &mut out);
                out.push(ch);
            }
        }

        flush_chunk(&mut current, &mut out);
        Self::expand_embedded_braces(&out, symbols)
    }

    fn expand_embedded_braces(text: &str, symbols: Option<*const SymbolsTable>) -> String {
        let mut out = String::with_capacity(text.len());
        let mut cursor = 0usize;

        while let Some(open_rel) = text[cursor..].find('{') {
            let open = cursor + open_rel;
            out.push_str(&text[cursor..open]);

            let Some(close_rel) = text[open + 1..].find('}')
            else {
                out.push_str(&text[open..]);
                return out;
            };
            let close = open + 1 + close_rel;

            let inner = &text[open + 1..close];
            let expanded_inner = Self::expand_symbol_with_listing_context(inner, symbols);
            if expanded_inner != inner {
                out.push_str(&Self::resolve_expanded_symbol_value(
                    &expanded_inner,
                    symbols
                ));
            }
            else {
                let wrapped = format!("__BASM_WRAP_BEGIN__{{{inner}}}__BASM_WRAP_END__");
                let expanded_wrapped = Self::expand_symbol_with_listing_context(&wrapped, symbols);
                if expanded_wrapped != wrapped {
                    if let Some(stripped) = expanded_wrapped
                        .strip_prefix("__BASM_WRAP_BEGIN__")
                        .and_then(|s| s.strip_suffix("__BASM_WRAP_END__"))
                    {
                        out.push_str(&Self::resolve_expanded_symbol_value(stripped, symbols));
                    }
                    else {
                        out.push_str(&Self::resolve_expanded_symbol_value(
                            &expanded_wrapped,
                            symbols
                        ));
                    }
                }
                else {
                    out.push('{');
                    out.push_str(inner);
                    out.push('}');
                }
            }

            cursor = close + 1;
        }

        if cursor < text.len() {
            out.push_str(&text[cursor..]);
        }

        out
    }

    fn resolve_expanded_symbol_value(
        expanded: &str,
        symbols: Option<*const SymbolsTable>
    ) -> String {
        let Some(symbols_ptr) = symbols
        else {
            return expanded.to_string();
        };
        let Some(symbols) = (unsafe { symbols_ptr.as_ref() })
        else {
            return expanded.to_string();
        };

        let lookup_value = |symbol: &str| {
            let Ok(Some(value)) = symbols.any_value::<&str>(symbol)
            else {
                return None;
            };

            Some(match value.value() {
                Value::String(s) => s.to_string(),
                Value::Expr(ExprResult::Char(c)) => (*c as char).to_string(),
                Value::Expr(ExprResult::String(s)) => s.to_string(),
                Value::Expr(other) => other.to_string(),
                Value::Counter(counter) => counter.to_string(),
                _ => symbol.to_string()
            })
        };

        if let Some(resolved) = lookup_value(expanded) {
            return resolved;
        }

        let qualified =
            Self::expand_symbol_with_listing_context(expanded, Some(symbols as *const _));
        if qualified != expanded
            && let Some(resolved) = lookup_value(&qualified)
        {
            return resolved;
        }

        if expanded.starts_with('.') {
            let needle = expanded.trim_start_matches('.');
            for symbol in symbols.available_symbols() {
                let candidate = symbol.value();
                if candidate.ends_with(needle)
                    && let Some(resolved) = lookup_value(candidate)
                {
                    return resolved;
                }
            }
        }

        expanded.to_string()
    }

    fn expand_listing_label(
        &self,
        raw_label: &str,
        symbols: Option<*const SymbolsTable>
    ) -> String {
        let normalized = raw_label.replace("{{", "{").replace("}}", "}");
        let expanded = Self::expand_symbol_with_listing_context(&normalized, symbols);
        if raw_label.starts_with('.')
            && expanded.starts_with('.')
            && let Some(global) = &self.listing_current_global_label
        {
            return format!("{global}{expanded}");
        }
        expanded
    }

    pub fn process_current_line(&mut self) {
        self.flush_current_token();

        // retrieve the line
        let (line_number, line_raw, line_expanded) = match self.current_line_group.clone() {
            Some((idx, line_raw, line_expanded)) => (idx, line_raw, line_expanded),
            None => return
        };

        // build the line representation for source and generated bytes
        let line_representation_raw = line_raw.split('\n').collect_vec();
        let line_representation_expanded = line_expanded.split('\n').collect_vec();
        let bytes_per_line = self.bytes_per_line();
        let data_chunks = self.current_line_bytes.chunks(bytes_per_line).collect_vec();
        let data_representation = data_chunks
            .iter()
            .map(|chunk| chunk.iter().map(|b| self.hex_byte(*b)).join(" "))
            .collect_vec();
        let mut tokens_remaining = self.current_line_tokens.clone();

        let mut token_chunks: Vec<Vec<ListingTokenItem>> = Vec::with_capacity(data_chunks.len());
        for chunk in data_chunks.iter() {
            let mut expected = chunk.len();
            let mut current = Vec::new();
            while expected > 0 {
                if tokens_remaining.is_empty() {
                    break;
                }
                let mut token = tokens_remaining.remove(0);

                if token.bytes.is_empty() {
                    current.push(token);
                    continue;
                }

                if token.bytes.len() <= expected {
                    expected -= token.bytes.len();
                    current.push(token);
                }
                else {
                    let right_bytes = token.bytes.split_off(expected);
                    let mut right = token.clone();
                    right.bytes = right_bytes;
                    token.bytes.truncate(expected);
                    current.push(token);
                    tokens_remaining.insert(0, right);
                    expected = 0;
                }
            }
            token_chunks.push(current);
        }

        let source_token_renders = self
            .current_line_tokens
            .iter()
            .map(|token| {
                ListingTokenRender {
                    token_id: token.token_id,
                    raw_text: token.raw.as_str(),
                    expanded_text: token.expanded.as_str(),
                    bytes: token.bytes.as_slice(),
                    token_kind: &token.token_kind
                }
            })
            .collect_vec();

        // TODO manage missing end of files/blocks if needed

        let delta = line_representation_raw.len();
        // TODO add the line representation ?
        for specific_content in self.deferred_for_line.iter() {
            let lines_count = line_representation_raw.len(); // line number corresponds to the VERY LAST line and not the FIRST one
            for (line_delta, line_raw) in line_representation_raw.iter().copied().enumerate() {
                let line_expanded = line_representation_expanded
                    .get(line_delta)
                    .copied()
                    .unwrap_or(line_raw);
                self.renderer.render_deferred(
                    &mut *self.writer,
                    &self.format,
                    bytes_per_line,
                    ListingDeferredRender {
                        row_id: 0,
                        file_index: self.current_file_index,
                        specific_content: if line_delta == 0 {
                            specific_content
                        }
                        else {
                            ""
                        },
                        line_number: Some(
                            line_number + delta as u32 + line_delta as u32 - lines_count as u32
                        ),
                        source_line_raw: line_raw,
                        source_line_expanded: line_expanded,
                        token_kind: &self.current_token_kind,
                        definition_target: None,
                        highlighted_symbols: &[],
                        collapsible: false,
                        collapsed_block: false
                    }
                );
            }
        }
        self.deferred_for_line.clear();

        // draw all lines that correspond to the instructions to output
        let mut last_mapped_line: Option<u32> = None;
        // Parallel to `last_mapped_line`: carries whether the run being
        // continued was data across a continuation chunk (a `defs`/`incbin`
        // tail with no token of its own).
        let mut last_mapped_is_data: Option<bool> = None;
        let mut byte_offset = 0usize;
        let render_lines = line_representation_raw.len().max(data_representation.len());
        for idx in 0..render_lines {
            let current_inner_line_raw = line_representation_raw.get(idx).copied();
            let current_inner_line_expanded = line_representation_expanded.get(idx).copied();
            let current_inner_data = data_representation.get(idx);
            let current_chunk = data_chunks.get(idx).copied().unwrap_or(&[]);
            let current_data_len = data_chunks.get(idx).map(|chunk| chunk.len()).unwrap_or(0);
            let is_multiline_continuation =
                idx > 0 && line_representation_raw.len() > 1 && current_inner_line_raw.is_some();
            let is_continuation_without_data =
                is_multiline_continuation && current_inner_data.is_none();

            let logical_address = self.current_first_address.wrapping_add(byte_offset as u32);
            let logical_representation = if is_continuation_without_data {
                None
            }
            else if current_inner_line_raw.is_none() && current_inner_data.is_none() {
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
            let phys_addr_representation = if is_continuation_without_data {
                Self::blank(self.physical_field_width())
            }
            else if !self.format.show_physical_address {
                Self::blank(self.physical_field_width())
            }
            else if current_inner_line_raw.is_none() && current_inner_data.is_none() {
                Self::blank(self.physical_field_width())
            }
            else if current_offset == logical_address
                && self.current_address_kind == AddressKind::Address
            {
                Self::blank(self.physical_field_width())
            }
            else {
                format!(
                    "{}{}",
                    self.format_address(current_offset, self.physical_address_width()),
                    self.current_address_kind
                )
            };
            let rendered_line_number = current_inner_line_raw.map(|_| line_number + idx as u32);

            // missing instruction must be added manually using TokenKind
            if self.has_current_line_output() {
                // A long byte run (`defs 16`, an `incbin`) renders as several
                // chunks, and only the first carries the source line - the
                // continuations are the *same* line's bytes, so they must be
                // recorded against it rather than dropped. `last_mapped_line`
                // carries it across the chunks.
                if let Some(line) = rendered_line_number {
                    last_mapped_line = Some(line);
                }
                if let Some(collector) = self.source_map.as_mut()
                    && let Some(line) = last_mapped_line
                    && let Some(logical) = logical_representation
                    && !current_chunk.is_empty()
                {
                    let name = self
                        .file_order
                        .get(self.current_file_index)
                        .map(String::as_str)
                        .unwrap_or("");
                    let file = collector.file_id(name);
                    // A macro or struct body is re-parsed as its own source, so
                    // the lines the spans carry count from the body, not from
                    // the file. The context name still says where the body was
                    // written - and it must be consulted *here*, before
                    // `file_id` collapses two different expansions of the same
                    // file onto one id: `INNER` called from `OUTER`'s body
                    // gives two rows both reading "line 2" that belong on
                    // different lines of the file.
                    let line_offset = super::source_map::expansion_line_offset(name);
                    // `current_offset` is already the physical position of
                    // these very bytes; the page is what distinguishes the
                    // same logical address in two banks.
                    let page = match self.current_physical_address {
                        PhysicalAddress::Memory(adr) => adr.page(),
                        PhysicalAddress::Bank(adr) => adr.bank() as u8,
                        PhysicalAddress::Cpr(adr) => adr.bloc() as u8
                    };

                    // One row per *instruction*, not per line: `ld a,l : inc a`
                    // is two, at two addresses, and a debugger stopped at the
                    // second should say so rather than pointing at the start of
                    // the line. The tokens of this chunk already carry their own
                    // bytes and columns, so the split is a walk over them.
                    let emitting: Vec<(u16, u16, u16, bool)> = token_chunks
                        .get(idx)
                        .map(|tokens| {
                            tokens
                                .iter()
                                .filter(|token| !token.bytes.is_empty())
                                .map(|token| {
                                    (
                                        token.column,
                                        token.column_end,
                                        token.bytes.len() as u16,
                                        token.is_data
                                    )
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    if emitting.is_empty() {
                        // A continuation chunk of a long run (`defs 16`), which
                        // carries bytes but no token of its own - inherit
                        // whether the run it continues was data, the same way
                        // `last_mapped_line` inherits which line it continues.
                        collector.push(
                            file,
                            line + line_offset,
                            logical,
                            current_offset,
                            page,
                            1,
                            1,
                            current_chunk.len() as u16,
                            last_mapped_is_data.unwrap_or(false)
                        );
                    }
                    else {
                        let mut offset = 0u32;
                        for (column, column_end, len, is_data) in emitting {
                            collector.push(
                                file,
                                line + line_offset,
                                logical + offset,
                                current_offset + offset,
                                page,
                                column,
                                column_end,
                                len,
                                is_data
                            );
                            offset += len as u32;
                            last_mapped_is_data = Some(is_data);
                        }
                    }
                }

                // The map has been collected above; everything below is for
                // the text, which nobody asked for when only the map was
                // wanted - so it is not built either, not merely not written.
                if self.map_only {
                    byte_offset += current_data_len;
                    continue;
                }

                let fallback_source_expanded = current_inner_line_expanded
                    .map(|line| line.trim_end())
                    .unwrap_or("");
                let fallback_source_raw = current_inner_line_raw
                    .map(|line| line.trim_end())
                    .unwrap_or("");
                let fallback_bytes = current_inner_data.cloned().unwrap_or_default();
                let token_renders = token_chunks
                    .get(idx)
                    .into_iter()
                    .flat_map(|tokens| tokens.iter())
                    .map(|token| {
                        ListingTokenRender {
                            token_id: token.token_id,
                            raw_text: token.raw.as_str(),
                            expanded_text: token.expanded.as_str(),
                            bytes: token.bytes.as_slice(),
                            token_kind: &token.token_kind
                        }
                    })
                    .collect_vec();

                self.renderer.render_line(
                    &mut *self.writer,
                    &self.format,
                    bytes_per_line,
                    ListingLineRender {
                        row_id: 0,
                        file_index: self.current_file_index,
                        logical_address: logical_representation,
                        physical_address_repr: &phys_addr_representation,
                        bytes: current_chunk,
                        fallback_bytes: &fallback_bytes,
                        line_number: rendered_line_number,
                        source_line_raw: fallback_source_raw,
                        source_line_expanded: fallback_source_expanded,
                        is_multiline_continuation,
                        token_kind: &self.current_token_kind,
                        tokens: token_renders.as_slice(),
                        source_tokens: source_token_renders.as_slice(),
                        definition_target: None,
                        highlighted_symbols: &[],
                        collapsible: false,
                        collapsed_block: false
                    }
                );
            }

            byte_offset += current_data_len;
        }

        if self.has_current_line_output() {
            let pending_iteration_notices = std::mem::take(&mut self.pending_iteration_notices);
            for notice in pending_iteration_notices.iter() {
                self.render_raw_notice_line(notice);
            }

            for counter in self.delayed_counter_update.iter() {
                self.renderer
                    .render_notice(&mut *self.writer, ListingNotice::RawLine(counter));
            }
            self.delayed_counter_update.clear();

            self.delayed_counter_update.append(&mut self.counter_update);
        }

        // cleanup all the fields of the current line
        self.current_line_group = None;
        self.current_line_last_offset = None;
        self.current_source = None;
        self.current_line_bytes.clear();
        self.current_line_tokens.clear();
        self.current_token_bytes.clear();
        self.current_token_raw.clear();
        self.current_token_expanded.clear();
    }

    pub fn finish(&mut self) {
        self.process_current_line();
        let pending_iteration_notices = std::mem::take(&mut self.pending_iteration_notices);
        for notice in pending_iteration_notices.iter() {
            self.render_raw_notice_line(notice);
        }
        let delayed_counter_update = std::mem::take(&mut self.delayed_counter_update);
        for counter in delayed_counter_update.iter() {
            self.render_raw_notice_line(counter);
        }
        let counter_update = std::mem::take(&mut self.counter_update);
        for counter in counter_update.iter() {
            self.render_raw_notice_line(counter);
        }
        if !self.deferred_for_line.is_empty() {
            panic!()
        }
        self.renderer.finish(&mut *self.writer);
    }

    /// Print filename if needed
    /// Start collecting a source map alongside (or instead of) the rendered
    /// listing.
    pub fn collect_source_map(&mut self) {
        self.source_map = Some(super::SourceMapCollector::new());
    }

    /// Collect the map and render nothing.
    pub fn collect_source_map_only(&mut self) {
        self.collect_source_map();
        self.map_only = true;
    }

    /// The collected rows, if any were asked for.
    pub fn take_source_map(&mut self) -> Option<super::RawSourceMap> {
        self.source_map.take().map(|c| c.finish())
    }

    /// The collected rows, without consuming them.
    pub fn source_map_snapshot(&self) -> Option<super::RawSourceMap> {
        self.source_map.as_ref().map(|c| c.snapshot())
    }

    pub fn manage_fname(&mut self, token: &LocatedToken) {
        // 	dbg!(token);

        let ctx = &token.span().state;
        let fname = ctx
            .filename()
            .map(|p| p.as_os_str().to_str().unwrap_or("<NO FNAME>").to_string())
            .or_else(|| ctx.context_name().map(|s| s.to_owned()));

        if let Some(fname) = fname {
            let (file_index, is_new_file) = self.register_source_file_name(&fname);
            self.current_file_index = file_index;

            if self.format.source_file_output_mode == ListingSourceFileOutputMode::None {
                self.current_fname = Some(fname);
                return;
            }

            if self.format.source_file_output_mode == ListingSourceFileOutputMode::FileMap {
                self.current_fname = Some(fname.clone());

                if !self.file_map_header_printed {
                    self.file_map_header_printed = true;
                    self.renderer.render_notice(
                        &mut *self.writer,
                        ListingNotice::FileMapHeader {
                            file_index,
                            fname: &fname
                        }
                    );
                    return;
                }

                if is_new_file {
                    self.renderer.render_notice(
                        &mut *self.writer,
                        ListingNotice::FileMapEntry {
                            file_index,
                            fname: &fname
                        }
                    );
                }

                return;
            }

            if !self.format.show_context_header {
                self.current_fname = Some(fname);
                return;
            }

            let print = match self.current_fname.as_ref() {
                Some(current_fname) => *current_fname != fname,
                None => true
            };

            if print {
                self.current_fname = Some(fname.clone());
                self.renderer.render_notice(
                    &mut *self.writer,
                    ListingNotice::ContextHeader {
                        file_index,
                        fname: &fname
                    }
                );
            }
        }
    }

    pub fn on(&mut self) {
        self.renderer.start(&mut *self.writer);
        self.activated = true;
    }

    pub fn off(&mut self) {
        self.finish();
        self.activated = false;
    }

    fn render_raw_notice_line(&mut self, line: &str) {
        let writer = &mut self.writer;
        let renderer = &mut self.renderer;
        renderer.render_notice(&mut **writer, ListingNotice::RawLine(line));
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
            self.token = None;
            self.bytes.clear();
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

    pub fn repeat_iteration(&mut self, counter: &str, value: Option<&ExprResult>, depth: usize) {
        let counter = format!("{counter} [depth {depth}]");
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

        let mut builder = self.builder.write().unwrap();
        builder.process_current_line();
        builder.pending_iteration_notices.push(line);
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::sync::{Arc, Mutex, RwLock};

    use cpclib_tokens::ExprResult;
    use cpclib_tokens::symbols::{MemoryPhysicalAddress, SymbolsTable, SymbolsTableTrait};

    use super::{
        DEFAULT_LISTING_LINE_TEMPLATE, ListingAddressRadix, ListingOutput, ListingOutputFormat,
        ListingOutputKind, ListingOutputTrigger, ListingSourceFileOutputMode, ListingTokenItem,
        MAX_RENDERED_SOURCE_COLUMN_CHARS, TokenKind
    };
    use crate::parse_z80_with_context_builder;
    use crate::preamble::{LocatedTokenInner, ParserContextBuilder};

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

        output.current_line_group =
            Some((12, "    ld a,0x12".to_string(), "    ld a,0x12".to_string()));
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

        output.current_line_group = Some((20, "label: nop".to_string(), "label: nop".to_string()));
        output.current_first_address = 0x200;
        output.current_physical_address = MemoryPhysicalAddress::new(0x200, 0).into();
        output.current_token_kind = TokenKind::Displayable;
        output
            .deferred_for_line
            .push("0200 00200 my_label".to_string());
        output
            .counter_update
            .push("0201 ????? <new iteration>".to_string());

        output.process_current_line();
        output.finish();

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

        output.current_line_group = Some((
            30,
            "macro SWITCH_VALUES addr1, addr2\n\tld hl, ({addr1})".to_string(),
            "macro SWITCH_VALUES addr1, addr2\n\tld hl, ({addr1})".to_string()
        ));
        output.current_first_address = 0x4000;
        output.current_physical_address = MemoryPhysicalAddress::new(0x4000, 0).into();
        output.current_token_kind = TokenKind::Displayable;
        output
            .deferred_for_line
            .push("MACRO      SWITCH_VALUES".to_string());

        output.process_current_line();

        let listing = writer.snapshot();
        assert!(
            listing.contains(
                "MACRO      SWITCH_VALUES               30 macro SWITCH_VALUES addr1, addr2"
            ),
            "listing={listing}"
        );
        assert!(
            listing.contains("                                       31 ld hl, ({addr1})"),
            "listing={listing}"
        );
    }

    #[test]
    fn process_current_line_wraps_overlong_deferred_label() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new_with_format(
            writer.clone(),
            ListingOutputFormat {
                listing_line_template: "{A} {P} {C} {L4} {S}".to_string(),
                output_kind: ListingOutputKind::Text,
                ..Default::default()
            }
        );
        output.on();

        output.current_line_group = Some((
            202,
            "scroller_generate_initial_code".to_string(),
            "scroller_generate_initial_code".to_string()
        ));
        output.current_first_address = 0x428B;
        output.current_physical_address = MemoryPhysicalAddress::new(0x428B, 0).into();
        output.current_token_kind = TokenKind::Displayable;
        output
            .deferred_for_line
            .push("428B 0428B scroller_generate_initial_code".to_string());

        output.process_current_line();

        let listing = writer.snapshot();
        assert!(
            listing.contains("428B 0428B scroller_generate_initial_code\n>"),
            "listing={listing}"
        );
        assert!(
            listing.contains(" 202 scroller_generate_initial_code"),
            "listing={listing}"
        );
    }

    #[test]
    fn extract_code_keeps_multiline_macro_call_span() {
        let source = "\
SWITCH_VALUES(\n\
    scroller_configuration_table.current_generating_code_table,\n\
    scroller_configuration_table.next_generating_code_table\n\
)\n";

        let listing = parse_z80_with_context_builder(source, ParserContextBuilder::default())
            .expect("parse should succeed");
        let token = listing
            .iter()
            .find(|token| matches!(&token.inner, either::Left(LocatedTokenInner::MacroCall(..))))
            .expect("macro call token expected");

        let extracted = ListingOutput::extract_code(token);
        assert!(
            extracted.contains("SWITCH_VALUES("),
            "extracted={extracted:?}"
        );
        assert!(
            extracted.contains("scroller_configuration_table.current_generating_code_table"),
            "extracted={extracted:?}"
        );
        assert!(
            extracted.contains("scroller_configuration_table.next_generating_code_table"),
            "extracted={extracted:?}"
        );
        assert!(extracted.contains('\n'), "extracted={extracted:?}");
    }

    #[test]
    fn extract_code_keeps_multiline_macro_definition_span() {
        let source = "\
macro SWITCH_VALUES addr1, addr2\n\
    ld hl, ({addr1})\n\
    ld de, ({addr2})\n\
endm\n";

        let listing = parse_z80_with_context_builder(source, ParserContextBuilder::default())
            .expect("parse should succeed");
        let token = listing
            .iter()
            .find(|token| matches!(&token.inner, either::Left(LocatedTokenInner::Macro { .. })))
            .expect("macro definition token expected");

        let extracted = ListingOutput::extract_code(token);
        assert!(
            extracted.contains("macro SWITCH_VALUES addr1, addr2"),
            "extracted={extracted:?}"
        );
        assert!(
            extracted.contains("ld hl, ({addr1})"),
            "extracted={extracted:?}"
        );
        assert!(
            extracted.contains("ld de, ({addr2})"),
            "extracted={extracted:?}"
        );
        assert!(extracted.contains("endm"), "extracted={extracted:?}");
    }

    #[test]
    fn expand_source_line_patterns_uses_shared_symbol_expansion() {
        let mut symbols = SymbolsTable::default();
        symbols
            .assign_symbol_to_value("line", ExprResult::Value(0))
            .unwrap();

        let rendered = ListingOutput::expand_source_line_patterns(
            "dw SCROLLER_CODE_{{line}}_a",
            Some(&symbols as *const _)
        );

        assert_eq!(rendered, "dw SCROLLER_CODE_0_a");
    }

    #[test]
    fn expand_source_line_patterns_expands_inline_local_placeholder() {
        let mut symbols = SymbolsTable::default();
        symbols
            .assign_symbol_to_value("nb_nops", ExprResult::Value(0))
            .unwrap();

        let rendered = ListingOutput::expand_source_line_patterns(
            "jr z, .has_nops{{nb_nops}}",
            Some(&symbols as *const _)
        );

        assert_eq!(rendered, "jr z, .has_nops0");
    }

    #[test]
    fn expand_source_line_patterns_accepts_repeat_counter_style_symbol_name() {
        let mut symbols = SymbolsTable::default();
        symbols
            .assign_symbol_to_value("{nb_nops}", ExprResult::Value(0))
            .unwrap();

        let rendered = ListingOutput::expand_source_line_patterns(
            "jr z, .has_nops{{nb_nops}}",
            Some(&symbols as *const _)
        );

        assert_eq!(rendered, "jr z, .has_nops0");
    }

    #[test]
    fn expand_source_line_patterns_expands_embedded_braced_symbol_name() {
        let rendered = ListingOutput::expand_source_line_patterns("dw letter_ipm{@letter}", None);
        assert_eq!(rendered, "dw letter_ipm{@letter}");
    }

    #[test]
    fn resolve_expanded_symbol_value_dereferences_hidden_symbol_by_suffix() {
        let mut symbols = SymbolsTable::default();
        symbols
            .assign_symbol_to_value(
                "FONT_CHAR_TABLE.__hidden__2012__letter",
                ExprResult::String("r".into())
            )
            .unwrap();

        let rendered = ListingOutput::resolve_expanded_symbol_value(
            ".__hidden__2012__letter",
            Some(&symbols as *const _)
        );

        assert_eq!(rendered, "r");
    }

    #[test]
    fn expand_source_line_patterns_preserves_hash_glyph_strings() {
        let mut symbols = SymbolsTable::default();
        symbols
            .set_current_global_label("letter_ipms_line_7")
            .unwrap();

        let rendered = ListingOutput::expand_source_line_patterns(
            "FONT_CREATE_IPM_CHAR( t, \"###.\", \".#..\")",
            Some(&symbols as *const _)
        );

        assert_eq!(rendered, "FONT_CREATE_IPM_CHAR( t, \"###.\", \".#..\")");
    }

    #[test]
    fn expand_listing_label_expands_local_pattern_placeholders() {
        let mut symbols = SymbolsTable::default();
        symbols
            .assign_symbol_to_value("nb_nops", ExprResult::Value(0))
            .unwrap();

        let mut output = ListingOutput::new(SharedBufferWriter::default());
        output.listing_current_global_label = Some("main".to_string());

        let rendered =
            output.expand_listing_label(".has_nops{{nb_nops}}", Some(&symbols as *const _));

        assert_eq!(rendered, "main.has_nops0");
    }

    #[test]
    fn process_current_line_can_render_expanded_and_raw_source_columns() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new_with_format(
            writer.clone(),
            ListingOutputFormat {
                listing_line_template: "{L4} {S} | {SR}".to_string(),
                source_file_output_mode: ListingSourceFileOutputMode::None,
                output_kind: ListingOutputKind::Text,
                ..Default::default()
            }
        );
        output.on();

        let mut symbols = SymbolsTable::default();
        symbols
            .assign_symbol_to_value("line", ExprResult::Value(0))
            .unwrap();

        output.current_line_group = Some((
            293,
            "dw SCROLLER_CODE_{{line}}_a".to_string(),
            "dw SCROLLER_CODE_0_a".to_string()
        ));
        output.current_first_address = 0x436B;
        output.current_physical_address = MemoryPhysicalAddress::new(0x436B, 0).into();
        output.current_token_kind = TokenKind::Displayable;

        output.process_current_line();

        let listing = writer.snapshot();
        assert!(
            listing.contains(" 293 dw SCROLLER_CODE_0_a | dw SCROLLER_CODE_{{line}}_a"),
            "listing={listing}"
        );
    }

    #[test]
    fn default_listing_template_contains_both_source_columns() {
        assert_eq!(
            ListingOutputFormat::default().listing_line_template,
            DEFAULT_LISTING_LINE_TEMPLATE
        );
    }

    #[test]
    fn source_columns_are_capped_to_80_chars() {
        let output = ListingOutput::new(SharedBufferWriter::default());
        let raw = format!(" {}", "R".repeat(MAX_RENDERED_SOURCE_COLUMN_CHARS + 5));
        let expanded = format!(" {}", "E".repeat(MAX_RENDERED_SOURCE_COLUMN_CHARS + 7));

        let rendered = output.format_line_with_template(
            Some(0x1234),
            "",
            &[],
            Some(1),
            Some(&raw),
            Some(&expanded)
        );

        let expected_expanded = "E".repeat(MAX_RENDERED_SOURCE_COLUMN_CHARS);
        let expected_raw = "R".repeat(MAX_RENDERED_SOURCE_COLUMN_CHARS);
        assert!(rendered.contains(&expected_expanded), "rendered={rendered}");
        assert!(!rendered.contains(&expected_raw), "rendered={rendered}");
        assert!(
            !rendered.contains(&"E".repeat(MAX_RENDERED_SOURCE_COLUMN_CHARS + 1)),
            "rendered={rendered}"
        );
        assert!(
            !rendered.contains(&"R".repeat(MAX_RENDERED_SOURCE_COLUMN_CHARS + 1)),
            "rendered={rendered}"
        );
    }

    #[test]
    fn html_renderer_outputs_listing_rows() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new_with_format(
            writer.clone(),
            ListingOutputFormat {
                output_kind: ListingOutputKind::Html,
                source_file_output_mode: ListingSourceFileOutputMode::None,
                ..Default::default()
            }
        );
        output.on();

        output.current_line_group = Some((12, "nop".to_string(), "nop".to_string()));
        output.current_line_bytes.extend_from_slice(&[0x00]);
        output.current_first_address = 0x0100;
        output.current_physical_address = MemoryPhysicalAddress::new(0x0100, 0).into();
        output.current_token_kind = TokenKind::Displayable;

        output.finish();

        let listing = writer.snapshot();
        assert!(listing.contains("<!DOCTYPE html>"), "listing={listing}");
        assert!(listing.contains("class=\"row\""), "listing={listing}");
        assert!(
            listing.contains("cell source-expanded interactive"),
            "listing={listing}"
        );
        assert!(
            !listing.contains("cell source-raw interactive"),
            "listing={listing}"
        );
        assert!(
            listing.contains("token byte\" data-hover-row=\"row-0\">00</span>"),
            "listing={listing}"
        );
    }

    #[test]
    fn html_renderer_marks_macro_definitions_as_collapsible() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new_with_format(
            writer.clone(),
            ListingOutputFormat {
                output_kind: ListingOutputKind::Html,
                source_file_output_mode: ListingSourceFileOutputMode::None,
                ..Default::default()
            }
        );
        output.on();

        output.current_line_group = Some((
            30,
            "macro SWITCH_VALUES addr1, addr2".to_string(),
            "macro SWITCH_VALUES addr1, addr2".to_string()
        ));
        output.current_token_kind = TokenKind::MacroDefine("SWITCH_VALUES".to_string());
        output
            .deferred_for_line
            .push("MACRO      SWITCH_VALUES".to_string());

        output.finish();

        let listing = writer.snapshot();
        assert!(
            listing.contains("data-block-kind=\"macro\""),
            "listing={listing}"
        );
        assert!(
            listing.contains("row deferred block-start"),
            "listing={listing}"
        );
    }

    #[test]
    fn html_renderer_links_token_and_bytes_with_same_hover_group() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new_with_format(
            writer.clone(),
            ListingOutputFormat {
                output_kind: ListingOutputKind::Html,
                source_file_output_mode: ListingSourceFileOutputMode::None,
                ..Default::default()
            }
        );
        output.on();

        output.current_line_group = Some((12, "ld a, 1".to_string(), "ld a, 1".to_string()));
        output.current_line_bytes.extend_from_slice(&[0x3E, 0x01]);
        output.current_first_address = 0x0100;
        output.current_physical_address = MemoryPhysicalAddress::new(0x0100, 0).into();
        output.current_token_kind = TokenKind::Displayable;
        output.current_line_tokens = vec![
            ListingTokenItem {
                token_id: 0,
                raw: "ld".to_string(),
                expanded: "ld".to_string(),
                bytes: vec![0x3E],
                token_kind: TokenKind::Displayable,
                column: 1,
                column_end: 1,
                is_data: false
            },
            ListingTokenItem {
                token_id: 1,
                raw: "1".to_string(),
                expanded: "1".to_string(),
                bytes: vec![0x01],
                token_kind: TokenKind::Displayable,
                column: 1,
                column_end: 1,
                is_data: false
            },
        ];

        output.finish();

        let listing = writer.snapshot();
        assert!(
            listing.contains("data-hover-row=\"tok-0\"><span class=\"token\" data-symbol-candidate=\"ld\">ld</span>"),
            "listing={listing}"
        );
        assert!(
            listing.contains("token byte\" data-hover-row=\"tok-0\">3E</span>"),
            "listing={listing}"
        );
        assert!(
            listing.contains("data-hover-row=\"tok-1\"><span class=\"token number\">1</span>"),
            "listing={listing}"
        );
        assert!(
            listing.contains("token byte\" data-hover-row=\"tok-1\">01</span>"),
            "listing={listing}"
        );
        assert!(
            listing.contains("class=\"byte-sep\" data-hover-row=\"tok-0\""),
            "listing={listing}"
        );
    }

    #[test]
    fn html_renderer_highlights_source_tokens_on_wrapped_byte_rows() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new_with_format(
            writer.clone(),
            ListingOutputFormat {
                output_kind: ListingOutputKind::Html,
                source_file_output_mode: ListingSourceFileOutputMode::None,
                ..Default::default()
            }
        );
        output.on();

        output.current_line_group = Some((
            39,
            "ld bc, 0xbc00 + 1 : out (c), c : ld bc, 0xbd00 + 96/2 : out (c), c".to_string(),
            "ld bc, 0xbc00 + 1 : out (c), c : ld bc, 0xbd00 + 96/2 : out (c), c".to_string()
        ));
        output
            .current_line_bytes
            .extend_from_slice(&[0x01, 0x01, 0xBC, 0xED, 0x49, 0x01, 0x30, 0xBD, 0xED, 0x49]);
        output.current_first_address = 0x4007;
        output.current_physical_address = MemoryPhysicalAddress::new(0x4007, 0).into();
        output.current_token_kind = TokenKind::Displayable;
        output.current_line_tokens = vec![
            ListingTokenItem {
                token_id: 29,
                raw: "ld bc, 0xbc00 + 1".to_string(),
                expanded: "ld bc, 0xbc00 + 1".to_string(),
                bytes: vec![0x01, 0x01, 0xBC],
                token_kind: TokenKind::Displayable,
                column: 1,
                column_end: 1,
                is_data: false
            },
            ListingTokenItem {
                token_id: 30,
                raw: "out (c), c".to_string(),
                expanded: "out (c), c".to_string(),
                bytes: vec![0xED, 0x49],
                token_kind: TokenKind::Displayable,
                column: 1,
                column_end: 1,
                is_data: false
            },
            ListingTokenItem {
                token_id: 31,
                raw: "ld bc, 0xbd00 + 96/2".to_string(),
                expanded: "ld bc, 0xbd00 + 96/2".to_string(),
                bytes: vec![0x01, 0x30, 0xBD],
                token_kind: TokenKind::Displayable,
                column: 1,
                column_end: 1,
                is_data: false
            },
            ListingTokenItem {
                token_id: 32,
                raw: "out (c), c".to_string(),
                expanded: "out (c), c".to_string(),
                bytes: vec![0xED, 0x49],
                token_kind: TokenKind::Displayable,
                column: 1,
                column_end: 1,
                is_data: false
            },
        ];

        output.finish();

        let listing = writer.snapshot();
        assert!(
            listing.contains("<span class=\"token fragment\" data-hover-row=\"tok-32\"><span class=\"token\" data-symbol-candidate=\"out\">out</span>&nbsp;(<span class=\"token\" data-symbol-candidate=\"c\">c</span>),&nbsp;<span class=\"token\" data-symbol-candidate=\"c\">c</span></span>"),
            "listing={listing}"
        );
        assert!(
            listing.contains("token byte\" data-hover-row=\"tok-32\">ED</span>"),
            "listing={listing}"
        );
    }

    #[test]
    fn html_renderer_styles_trailing_comment_in_precise_source_mode() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new_with_format(
            writer.clone(),
            ListingOutputFormat {
                output_kind: ListingOutputKind::Html,
                source_file_output_mode: ListingSourceFileOutputMode::None,
                ..Default::default()
            }
        );
        output.on();

        output.current_line_group = Some((
            12,
            "add hl, de ; trailing comment".to_string(),
            "add hl, de ; trailing comment".to_string()
        ));
        output.current_line_bytes.extend_from_slice(&[0x19]);
        output.current_first_address = 0x0100;
        output.current_physical_address = MemoryPhysicalAddress::new(0x0100, 0).into();
        output.current_token_kind = TokenKind::Displayable;
        output.current_line_tokens = vec![ListingTokenItem {
            token_id: 0,
            raw: "add hl, de".to_string(),
            expanded: "add hl, de".to_string(),
            bytes: vec![0x19],
            token_kind: TokenKind::Displayable,
            column: 1,
            column_end: 1,
            is_data: false
        }];

        output.finish();

        let listing = writer.snapshot();
        assert!(
            listing.contains("<span class=\"token comment\">; trailing comment</span>"),
            "listing={listing}"
        );
    }

    #[test]
    fn html_renderer_can_link_to_symbol_defined_on_deferred_row() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new_with_format(
            writer.clone(),
            ListingOutputFormat {
                output_kind: ListingOutputKind::Html,
                source_file_output_mode: ListingSourceFileOutputMode::None,
                ..Default::default()
            }
        );
        output.on();

        output.current_line_group = Some((10, "start:".to_string(), "start:".to_string()));
        output.current_first_address = 0x0100;
        output.current_physical_address = MemoryPhysicalAddress::new(0x0100, 0).into();
        output.current_token_kind = TokenKind::Label("start".to_string());
        output.deferred_for_line.push("0100 0100 start".to_string());
        output.process_current_line();

        output.current_line_group = Some((11, "jp start".to_string(), "jp start".to_string()));
        output
            .current_line_bytes
            .extend_from_slice(&[0xC3, 0x00, 0x01]);
        output.current_first_address = 0x0101;
        output.current_physical_address = MemoryPhysicalAddress::new(0x0101, 0).into();
        output.current_token_kind = TokenKind::Displayable;
        output.current_line_tokens = vec![ListingTokenItem {
            token_id: 0,
            raw: "start".to_string(),
            expanded: "start".to_string(),
            bytes: vec![0x00, 0x01],
            token_kind: TokenKind::Displayable,
            column: 1,
            column_end: 1,
            is_data: false
        }];

        output.finish();

        let listing = writer.snapshot();
        assert!(
            listing.contains("data-symbol-candidate=\"start\""),
            "listing={listing}"
        );
        assert!(listing.contains("['start', 0]"), "listing={listing}");
        assert!(
            !listing.contains("if (row && row.id === `row-${target}`)"),
            "listing={listing}"
        );
    }

    #[test]
    fn html_renderer_uses_first_destination_for_duplicate_labels() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new_with_format(
            writer.clone(),
            ListingOutputFormat {
                output_kind: ListingOutputKind::Html,
                source_file_output_mode: ListingSourceFileOutputMode::None,
                ..Default::default()
            }
        );
        output.on();

        output.current_line_group = Some((10, "dup_label:".to_string(), "dup_label:".to_string()));
        output.current_first_address = 0x0100;
        output.current_physical_address = MemoryPhysicalAddress::new(0x0100, 0).into();
        output.current_token_kind = TokenKind::Label("dup_label".to_string());
        output
            .deferred_for_line
            .push("0100 0100 dup_label".to_string());
        output.process_current_line();

        output.current_line_group = Some((20, "dup_label:".to_string(), "dup_label:".to_string()));
        output.current_first_address = 0x0200;
        output.current_physical_address = MemoryPhysicalAddress::new(0x0200, 0).into();
        output.current_token_kind = TokenKind::Label("dup_label".to_string());
        output
            .deferred_for_line
            .push("0200 0200 dup_label".to_string());
        output.process_current_line();

        output.finish();

        let listing = writer.snapshot();
        assert!(listing.contains("['dup_label', 0]"), "listing={listing}");
        assert!(!listing.contains("['dup_label', 1]"), "listing={listing}");
    }

    #[test]
    fn local_label_is_rendered_with_global_prefix_in_deferred_listing() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new_with_format(
            writer.clone(),
            ListingOutputFormat {
                source_file_output_mode: ListingSourceFileOutputMode::None,
                ..Default::default()
            }
        );
        output.on();

        output.current_line_group = Some((10, "main:".to_string(), "main:".to_string()));
        output.current_first_address = 0x0100;
        output.current_physical_address = MemoryPhysicalAddress::new(0x0100, 0).into();
        output.current_token_kind = TokenKind::Label("main".to_string());
        output.deferred_for_line.push("0100 0100 main".to_string());
        output.process_current_line();

        output.current_line_group = Some((11, ".loop:".to_string(), ".loop:".to_string()));
        output.current_first_address = 0x0101;
        output.current_physical_address = MemoryPhysicalAddress::new(0x0101, 0).into();
        output.current_token_kind = TokenKind::Label("main.loop".to_string());
        output
            .deferred_for_line
            .push("0101 0101 main.loop".to_string());

        output.finish();

        let listing = writer.snapshot();
        assert!(listing.contains("main.loop"), "listing={listing}");
    }

    #[test]
    fn text_renderer_qualifies_local_reference_with_current_global_label() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new_with_format(
            writer.clone(),
            ListingOutputFormat {
                source_file_output_mode: ListingSourceFileOutputMode::None,
                output_kind: ListingOutputKind::Text,
                ..Default::default()
            }
        );
        output.on();

        output.current_line_group = Some((10, "main:".to_string(), "main:".to_string()));
        output.current_first_address = 0x0100;
        output.current_physical_address = MemoryPhysicalAddress::new(0x0100, 0).into();
        output.current_token_kind = TokenKind::Label("main".to_string());
        output.deferred_for_line.push("0100 0100 main".to_string());
        output.process_current_line();

        output.current_line_group = Some((
            11,
            "jr z, .has_nops0".to_string(),
            "jr z, .has_nops0".to_string()
        ));
        output.current_line_bytes.extend_from_slice(&[0x28, 0x00]);
        output.current_first_address = 0x0101;
        output.current_physical_address = MemoryPhysicalAddress::new(0x0101, 0).into();
        output.current_token_kind = TokenKind::Displayable;

        output.finish();

        let listing = writer.snapshot();
        assert!(
            listing.contains("jr z, main.has_nops0"),
            "listing={listing}"
        );
    }

    #[test]
    fn process_current_line_marks_multiline_continuation_with_gt() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new(writer.clone());
        output.on();

        output.current_line_group = Some((
            227,
            "SWITCH_VALUES(\n\ta,\n\tb\n)".to_string(),
            "SWITCH_VALUES(\n\ta,\n\tb\n)".to_string()
        ));
        output.current_first_address = 0x42A2;
        output.current_physical_address = MemoryPhysicalAddress::new(0x42A2, 0).into();
        output.current_token_kind = TokenKind::Displayable;

        output.process_current_line();

        let listing = writer.snapshot();
        let mut lines = listing.lines();
        let _first = lines.next().unwrap_or_default();
        let second = lines.next().unwrap_or_default();
        assert!(second.starts_with('>'), "listing={listing}");
    }

    #[test]
    fn process_current_line_multiline_without_data_uses_dot_marker() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new(writer.clone());
        output.on();

        output.current_line_group = Some((
            257,
            "repeat 3\nline2\nline3\nendr".to_string(),
            "repeat 3\nline2\nline3\nendr".to_string()
        ));
        output.current_first_address = 0x42CC;
        output.current_physical_address = MemoryPhysicalAddress::new(0x42CC, 0).into();
        output.current_token_kind = TokenKind::Displayable;

        output.process_current_line();

        let listing = writer.snapshot();
        let second = listing.lines().nth(1).unwrap_or_default();
        assert!(
            second.contains("                            258"),
            "listing={listing}"
        );
        assert!(!second.contains('.'), "listing={listing}");
    }

    #[test]
    fn finish_flushes_iteration_counter_after_block_content() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new(writer.clone());
        output.on();

        output.current_line_group = Some((
            257,
            "repeat 2\nline2\nendr".to_string(),
            "repeat 2\nline2\nendr".to_string()
        ));
        output.current_first_address = 0x42CC;
        output.current_physical_address = MemoryPhysicalAddress::new(0x42CC, 0).into();
        output.current_token_kind = TokenKind::Displayable;
        output
            .counter_update
            .push("0001 ????? <new iteration>".to_string());

        output.finish();

        let listing = writer.snapshot();
        let block_pos = listing.find("42CC").unwrap_or(usize::MAX);
        let counter_pos = listing.find("0001 ????? <new iteration>").unwrap_or(0);
        assert!(counter_pos > block_pos, "listing={listing}");
    }

    #[test]
    fn repeat_iteration_renders_counter_notice_after_completed_iteration_line() {
        let writer = SharedBufferWriter::default();
        let output = ListingOutput::new(writer.clone());
        let builder = Arc::new(RwLock::new(output));
        builder.write().unwrap().on();

        let mut trigger = ListingOutputTrigger {
            token: None,
            bytes: Vec::new(),
            symbols: None,
            start: 0,
            physical_address: MemoryPhysicalAddress::new(0x0100, 0).into(),
            builder: builder.clone()
        };

        trigger.repeat_iteration("<new iteration>", Some(&ExprResult::Value(1)), 1);

        {
            let mut output = builder.write().unwrap();
            output.current_line_group =
                Some((10, "db 0xBE, 0xEF".to_string(), "db 0xBE, 0xEF".to_string()));
            output.current_line_bytes.extend_from_slice(&[0xBE, 0xEF]);
            output.current_first_address = 0x0100;
            output.current_physical_address = MemoryPhysicalAddress::new(0x0100, 0).into();
            output.current_token_kind = TokenKind::Hidden;
        }

        trigger.repeat_iteration("<new iteration>", Some(&ExprResult::Value(2)), 2);

        {
            let mut output = builder.write().unwrap();
            output.current_line_group =
                Some((10, "db 0xBE, 0xEF".to_string(), "db 0xBE, 0xEF".to_string()));
            output.current_line_bytes.extend_from_slice(&[0xBE, 0xEF]);
            output.current_first_address = 0x0102;
            output.current_physical_address = MemoryPhysicalAddress::new(0x0102, 0).into();
            output.current_token_kind = TokenKind::Displayable;
            output.finish();
        }

        let listing = writer.snapshot();
        let first_counter = listing
            .find("0001 ????? <new iteration>")
            .unwrap_or(usize::MAX);
        let first_db = listing.find("0100 00100N BE EF").unwrap_or(usize::MAX);
        let second_counter = listing
            .find("0002 ????? <new iteration>")
            .unwrap_or(usize::MAX);
        let second_db = listing.find("0102 00102N BE EF").unwrap_or(usize::MAX);

        assert!(first_db < first_counter, "listing={listing}");
        assert!(first_counter < second_db, "listing={listing}");
        assert!(second_db < second_counter, "listing={listing}");
    }

    #[test]
    fn process_current_line_renders_continuation_addresses_for_long_byte_sequences() {
        let writer = SharedBufferWriter::default();
        let mut output = ListingOutput::new(writer.clone());
        output.on();

        output.current_line_group = Some((
            30,
            "db 0,1,2,3,4,5,6,7,8,9".to_string(),
            "db 0,1,2,3,4,5,6,7,8,9".to_string()
        ));
        output
            .current_line_bytes
            .extend_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        output.current_first_address = 0x4000;
        output.current_physical_address = MemoryPhysicalAddress::new(0x4000, 0).into();
        output.current_token_kind = TokenKind::Displayable;

        output.process_current_line();

        let listing = writer.snapshot();
        assert!(listing.contains("4000"), "listing={listing}");
        assert!(listing.contains("4008"), "listing={listing}");
        assert!(
            listing.contains("00 01 02 03 04 05 06 07"),
            "listing={listing}"
        );
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

        output.current_line_group = Some((
            40,
            "db 0xAA,0xBB,0xCC,0xDD,0xEE".to_string(),
            "db 0xAA,0xBB,0xCC,0xDD,0xEE".to_string()
        ));
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
                source_file_output_mode: ListingSourceFileOutputMode::None,
                output_kind: ListingOutputKind::Text
            }
        );
        output.on();

        output.current_line_group = Some((7, "db 1,2,3".to_string(), "db 1,2,3".to_string()));
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
                output_kind: ListingOutputKind::Text,
                ..Default::default()
            }
        );
        output.on();

        output.current_line_group = Some((9, "  nop".to_string(), "  nop".to_string()));
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
                output_kind: ListingOutputKind::Text,
                ..Default::default()
            }
        );
        output.on();

        output.current_line_group = Some((
            45,
            "ld bc, 0xbc00 + 13 : out (c),c : ld bc, 0xbd00 : out (c), l".to_string(),
            "ld bc, 0xbc00 + 13 : out (c),c : ld bc, 0xbd00 : out (c), l".to_string()
        ));
        output
            .current_line_bytes
            .extend_from_slice(&[0x01, 0x0D, 0xBC, 0xED, 0x49, 0x01, 0x00, 0xBD]);
        output.current_first_address = 0x403C;
        output.current_physical_address = MemoryPhysicalAddress::new(0x403C, 0).into();
        output.current_token_kind = TokenKind::Displayable;
        output.process_current_line();

        output.current_line_group = Some((50, "ld sp, $".to_string(), "ld sp, $".to_string()));
        output
            .current_line_bytes
            .extend_from_slice(&[0x31, 0x55, 0x40]);
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
