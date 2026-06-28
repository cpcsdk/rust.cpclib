use std::collections::HashSet;
use std::sync::LazyLock;
use tower_lsp::lsp_types::*;
use cpclib_asm::parser::context::ParserContextBuilder;
use cpclib_asm::parser::obtained::{LocatedListing, MayHaveSpan};
use cpclib_tokens::ListingElement;
use crate::document::Document;

// Semantic token type indices — must match `semantic_tokens_legend()` order
const TT_KEYWORD: u32 = 0;    // Z80 instructions
const TT_MACRO: u32 = 1;      // assembler directives
const TT_FUNCTION: u32 = 2;   // labels
const TT_NAMESPACE: u32 = 3;  // module names
const TT_VARIABLE: u32 = 4;   // registers / condition codes
const TT_NUMBER: u32 = 5;     // numeric literals
const TT_STRING: u32 = 6;     // string literals
const TT_COMMENT: u32 = 7;    // line comments
const TT_OPERATOR: u32 = 8;   // operators
const TT_ENUM_MEMBER: u32 = 9; // EQU / assign constants

const MOD_DECLARATION: u32 = 1 << 0;
const MOD_READONLY: u32 = 1 << 1;

// Full Z80 register set + condition codes used as operands
const REGISTER_LIST: &[&str] = &[
    "AF'", "AF", "BC", "DE", "HL", "IX", "IY", "SP", "PC",
    "IXH", "IXL", "IYH", "IYL",
    "A", "B", "C", "D", "E", "H", "L", "F", "I", "R",
    "NZ", "Z", "NC", "PE", "PO", "P", "M",
];

// Static lookup sets — built once, shared across all tokenizer calls
pub static INSTRUCTION_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    cpclib_asm::lsp::Z80_INSTRUCTIONS.iter().copied().collect()
});

static DIRECTIVE_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    for d in cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_STANDALONE { s.insert(*d); }
    for d in cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_START       { s.insert(*d); }
    for d in cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_END         { s.insert(*d); }
    s
});

static REGISTER_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    REGISTER_LIST.iter().copied().collect()
});

/// Returns the SemanticTokensLegend that must be advertised in `initialize()`.
pub fn semantic_tokens_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::KEYWORD,
            SemanticTokenType::MACRO,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::NAMESPACE,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::NUMBER,
            SemanticTokenType::STRING,
            SemanticTokenType::COMMENT,
            SemanticTokenType::OPERATOR,
            SemanticTokenType::ENUM_MEMBER,
        ],
        token_modifiers: vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::READONLY,
        ],
    }
}

/// Analyzer for Z80 assembly files using basm syntax
pub struct AssemblyAnalyzer {}

impl AssemblyAnalyzer {
    pub fn new() -> Self {
        Self {}
    }

    /// Parse the assembly document and return the listing
    fn parse_document(&self, document: &Document) -> Result<LocatedListing, LocatedListing> {
        let text = document.text();
        
        // Create a parser context builder
        let builder = ParserContextBuilder::default();
        
        // Parse the assembly code using new_complete_source
        LocatedListing::new_complete_source(text, builder)
    }

    /// Analyze the document and return diagnostics
    pub fn analyze(&self, document: &Document) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        
        match self.parse_document(document) {
            Ok(_listing) => {
                // Parsing succeeded - no errors to report
                // TODO: could extract warnings if the API provides them
            }
            Err(listing_with_errors) => {
                // Parse error - the listing contains the errors
                // TODO: Extract error details from the listing
                // For now, create a generic error
                let diagnostic = Diagnostic {
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 0,
                            character: 100,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("basm".to_string()),
                    message: format!("{}", listing_with_errors.cpclib_error_unchecked()),
                    related_information: None,
                    tags: None,
                    data: None,
                };
                diagnostics.push(diagnostic);
            }
        }
        
        diagnostics
    }

    /// Provide hover information at the given position
    pub fn hover(&self, document: &Document, position: Position) -> Option<Hover> {
        let line_idx = position.line as usize;
        let line = document.line(line_idx)?;
        let col = position.character as usize;

        // Numeric literal — show all bases
        if let Some((num_str, value)) = extract_number_at_position(&line, col) {
            let bin = format_binary(value);
            let md = format!(
                "**`{num_str}`**\n\n\
                | Base | Value |\n\
                |------|-------|\n\
                | Decimal | `{value}` |\n\
                | Hex | `{value:#X}` |\n\
                | Binary | `{bin}` |"
            );
            return Some(make_hover(md));
        }

        let word = self.extract_word_at_position(&line, col)?;
        let word_upper = word.to_uppercase();

        // Instruction — show timing data from the full instruction line
        if INSTRUCTION_SET.contains(word_upper.as_str()) {
            let full = crate::timings::extract_instruction_at_col(&line, col)
                .unwrap_or_else(|| word.clone());
            let entries = crate::timings::find_timings(&full);
            let md = if entries.is_empty() {
                format!("**{}** — Z80 instruction", word_upper)
            } else {
                crate::timings::format_hover(&full, &entries)
            };
            return Some(make_hover(md));
        }

        // Register / condition code
        if let Some(md) = register_description(&word_upper) {
            return Some(make_hover(md));
        }

        // Symbol — look up EQU / assign / label in the listing (only when parse succeeds)
        if let Ok(listing) = self.parse_document(document) {
            for token in listing.iter() {
                if token.is_equ() {
                    let sym = token.equ_symbol();
                    if sym.to_uppercase() == word_upper {
                        return Some(make_hover(format!(
                            "**{}** = `{}`\n\n*EQU constant*", sym, token.equ_value()
                        )));
                    }
                } else if token.is_assign() {
                    let sym = token.assign_symbol();
                    if sym.to_uppercase() == word_upper {
                        return Some(make_hover(format!(
                            "**{}** = `{}`\n\n*Assign*", sym, token.assign_value()
                        )));
                    }
                } else if token.is_label() {
                    let sym = token.label_symbol();
                    if sym.to_uppercase() == word_upper {
                        return Some(make_hover(format!("**{}** — label", sym)));
                    }
                }
            }
        }

        None
    }

    /// Provide completion suggestions
    pub fn completion(&self, _document: &Document, _position: Position) -> Vec<CompletionItem> {
        let mut completions = Vec::new();
        
        // Add Z80 instruction completions using generated data from cpclib-asm
        for mnemonic in cpclib_asm::lsp::Z80_INSTRUCTIONS {
            completions.push(CompletionItem {
                label: mnemonic.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Z80 instruction".to_string()),
                documentation: None,
                ..Default::default()
            });
        }
        
        // Add assembler directives using generated data from cpclib-asm
        for directive in cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_STANDALONE {
            completions.push(CompletionItem {
                label: directive.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(format!("Assembler directive: {}", directive)),
                documentation: None,
                ..Default::default()
            });
        }
        
        for directive in cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_START {
            completions.push(CompletionItem {
                label: directive.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(format!("Block start directive: {}", directive)),
                documentation: None,
                ..Default::default()
            });
        }
        
        for directive in cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_END {
            completions.push(CompletionItem {
                label: directive.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(format!("Block end directive: {}", directive)),
                documentation: None,
                ..Default::default()
            });
        }
        
        // Add registers using generated data from cpclib-asm
        for register in cpclib_asm::lsp::Z80_REGISTERS {
            completions.push(CompletionItem {
                label: register.to_string(),
                kind: Some(CompletionItemKind::CONSTANT),
                detail: Some("Z80 Register".to_string()),
                documentation: None,
                ..Default::default()
            });
        }
        
        completions
    }

    /// Find the definition of a symbol — looks up the word under the cursor in the parsed listing.
    pub fn goto_definition(&self, document: &Document, position: Position) -> Option<Location> {
        let line = document.line(position.line as usize)?;
        let word = self.extract_word_at_position(&line, position.character as usize)?;
        let word_upper = word.to_uppercase();

        if let Ok(listing) = self.parse_document(document) {
            for token in listing.iter() {
                let source_name: &str = if token.is_label() {
                    token.label_symbol()
                } else if token.is_equ() {
                    token.equ_symbol()
                } else if token.is_assign() {
                    token.assign_symbol()
                } else if token.is_macro_definition() {
                    token.macro_definition_name()
                } else if token.is_module() {
                    token.module_name()
                } else {
                    continue;
                };

                if source_name.to_uppercase() == word_upper {
                    let span = token.span();
                    let (line_1based, col_1based) = span.relative_line_and_column();
                    let lsp_line = line_1based.saturating_sub(1) as u32;
                    let lsp_char = col_1based.saturating_sub(1) as u32;
                    let range = Range {
                        start: Position { line: lsp_line, character: lsp_char },
                        end: Position { line: lsp_line, character: lsp_char + source_name.len() as u32 },
                    };
                    return Some(Location { uri: document.uri.clone(), range });
                }
            }
        }
        None
    }

    /// Find all references to a symbol
    pub fn find_references(&self, _document: &Document, _position: Position) -> Vec<Location> {
        // TODO: Implement symbol table tracking to find all references
        Vec::new()
    }

    /// Get document symbols (labels, EQU constants, macros, modules).
    ///
    /// Local labels (starting with `.`) are shown as `parent.local` in the outline.
    /// EQU and assign constants include their expression value in the detail.
    pub fn document_symbols(&self, document: &Document) -> Vec<DocumentSymbol> {
        let mut symbols = Vec::new();

        let Ok(listing) = self.parse_document(document) else {
            return symbols;
        };

        // Track the last seen global label to qualify local labels (`.foo` → `parent.foo`)
        let mut current_global: Option<String> = None;

        for token in listing.iter() {
            // source_name: as it appears in source (for range length)
            // display_name: what the outline shows
            let (source_name, display_name, kind, detail): (&str, String, SymbolKind, Option<String>) =
                if token.is_label() {
                    let raw = token.label_symbol();
                    let display = if raw.starts_with('.') {
                        match &current_global {
                            Some(g) => format!("{}{}", g, raw),
                            None    => raw.to_string(),
                        }
                    } else {
                        current_global = Some(raw.to_string());
                        raw.to_string()
                    };
                    (raw, display, SymbolKind::FUNCTION, None)
                } else if token.is_equ() {
                    let sym = token.equ_symbol();
                    (sym, sym.to_string(), SymbolKind::CONSTANT,
                     Some(format!("= {}", token.equ_value())))
                } else if token.is_assign() {
                    let sym = token.assign_symbol();
                    (sym, sym.to_string(), SymbolKind::VARIABLE,
                     Some(format!("= {}", token.assign_value())))
                } else if token.is_macro_definition() {
                    let name = token.macro_definition_name();
                    current_global = Some(name.to_string());
                    (name, name.to_string(), SymbolKind::FUNCTION, Some("MACRO".to_string()))
                } else if token.is_module() {
                    let name = token.module_name();
                    current_global = Some(name.to_string());
                    (name, name.to_string(), SymbolKind::MODULE, None)
                } else {
                    continue;
                };

            let span = token.span();
            let (line_1based, col_1based) = span.relative_line_and_column();
            let lsp_line = line_1based.saturating_sub(1) as u32;
            let lsp_char = col_1based.saturating_sub(1) as u32;
            // Range covers the source token, not the (potentially longer) display name
            let range = Range {
                start: Position { line: lsp_line, character: lsp_char },
                end: Position { line: lsp_line, character: lsp_char + source_name.len() as u32 },
            };

            #[allow(deprecated)]
            symbols.push(DocumentSymbol {
                name: display_name,
                detail,
                kind,
                tags: None,
                deprecated: None,
                range: range.clone(),
                selection_range: range,
                children: None,
            });
        }

        symbols
    }

    /// Produce semantic tokens for the full document.
    pub fn semantic_tokens(&self, document: &Document) -> Vec<SemanticToken> {
        // Static lookup sets (built once on first call)
        let instructions = &*INSTRUCTION_SET;
        let directives   = &*DIRECTIVE_SET;
        let registers    = &*REGISTER_SET;

        // Best-effort AST parse to identify EQU / assign / macro / module definition names
        let mut equ_names: HashSet<String> = HashSet::new();
        let mut assign_names: HashSet<String> = HashSet::new();
        let mut macro_names: HashSet<String> = HashSet::new();
        let mut module_names: HashSet<String> = HashSet::new();
        if let Ok(listing) = self.parse_document(document) {
            for token in listing.iter() {
                if token.is_equ() {
                    equ_names.insert(token.equ_symbol().to_uppercase());
                } else if token.is_assign() {
                    assign_names.insert(token.assign_symbol().to_uppercase());
                } else if token.is_macro_definition() {
                    macro_names.insert(token.macro_definition_name().to_uppercase());
                } else if token.is_module() {
                    module_names.insert(token.module_name().to_uppercase());
                }
            }
        }

        // Raw tokens collected in document order: (line, col, len, type, modifiers)
        let mut raw: Vec<(u32, u32, u32, u32, u32)> = Vec::new();
        let text = document.text();

        'line: for (line_idx, line) in text.lines().enumerate() {
            let line_u = line_idx as u32;
            let bytes = line.as_bytes();
            let mut col: usize = 0;

            while col < bytes.len() {
                let c = bytes[col];

                // Whitespace — skip
                if c == b' ' || c == b'\t' { col += 1; continue; }

                // Comment: `;` through end of line
                if c == b';' {
                    raw.push((line_u, col as u32, (bytes.len() - col) as u32, TT_COMMENT, 0));
                    continue 'line;
                }

                // Double-quoted string
                if c == b'"' {
                    let start = col; col += 1;
                    while col < bytes.len() {
                        if bytes[col] == b'\\' && col + 1 < bytes.len() { col += 2; continue; }
                        if bytes[col] == b'"' { col += 1; break; }
                        col += 1;
                    }
                    raw.push((line_u, start as u32, (col - start) as u32, TT_STRING, 0));
                    continue;
                }

                // Single-quoted string — only when NOT preceded by a word char (avoids AF')
                if c == b'\'' {
                    let prev_word = col > 0 && {
                        let p = bytes[col - 1];
                        p.is_ascii_alphanumeric() || p == b'_'
                    };
                    if !prev_word {
                        let start = col; col += 1;
                        while col < bytes.len() {
                            if bytes[col] == b'\\' && col + 1 < bytes.len() { col += 2; continue; }
                            if bytes[col] == b'\'' { col += 1; break; }
                            col += 1;
                        }
                        raw.push((line_u, start as u32, (col - start) as u32, TT_STRING, 0));
                    } else {
                        col += 1; // stray ' (e.g. AF' already consumed by register scan)
                    }
                    continue;
                }

                // Hex literal: $hexdigits
                if c == b'$' && col + 1 < bytes.len() && bytes[col + 1].is_ascii_hexdigit() {
                    let start = col; col += 1;
                    while col < bytes.len() && bytes[col].is_ascii_hexdigit() { col += 1; }
                    raw.push((line_u, start as u32, (col - start) as u32, TT_NUMBER, 0));
                    continue;
                }

                // Binary literal: %0101… (else treat % as operator)
                if c == b'%' {
                    let start = col; col += 1;
                    if col < bytes.len() && (bytes[col] == b'0' || bytes[col] == b'1') {
                        while col < bytes.len() && (bytes[col] == b'0' || bytes[col] == b'1') {
                            col += 1;
                        }
                        raw.push((line_u, start as u32, (col - start) as u32, TT_NUMBER, 0));
                    } else {
                        raw.push((line_u, start as u32, 1, TT_OPERATOR, 0));
                    }
                    continue;
                }

                // Numeric literal starting with a digit
                if c.is_ascii_digit() {
                    let start = col;
                    if c == b'0' && col + 1 < bytes.len()
                        && (bytes[col + 1] == b'x' || bytes[col + 1] == b'X')
                    {
                        col += 2;
                        while col < bytes.len() && bytes[col].is_ascii_hexdigit() { col += 1; }
                    } else if c == b'0' && col + 1 < bytes.len()
                        && (bytes[col + 1] == b'b' || bytes[col + 1] == b'B')
                    {
                        col += 2;
                        while col < bytes.len() && (bytes[col] == b'0' || bytes[col] == b'1') {
                            col += 1;
                        }
                    } else {
                        while col < bytes.len() && bytes[col].is_ascii_hexdigit() { col += 1; }
                        if col < bytes.len() && (bytes[col] == b'H' || bytes[col] == b'h') {
                            col += 1;
                        }
                    }
                    raw.push((line_u, start as u32, (col - start) as u32, TT_NUMBER, 0));
                    continue;
                }

                // Identifier: letter / _ / @ / .
                if c.is_ascii_alphabetic() || c == b'_' || c == b'@' || c == b'.' {
                    let start = col;
                    while col < bytes.len() {
                        let ch = bytes[col];
                        if ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'@' || ch == b'.' {
                            col += 1;
                        } else {
                            break;
                        }
                    }

                    // AF' special case: include trailing '
                    let word_no_prime = &line[start..col];
                    let word_upper_base = word_no_prime.to_uppercase();
                    let has_prime = col < bytes.len() && bytes[col] == b'\'';
                    let is_af_prime = has_prime && word_upper_base == "AF";
                    if is_af_prime { col += 1; }

                    let word_upper: String = if is_af_prime {
                        format!("AF'")
                    } else {
                        word_upper_base
                    };
                    let word_len = col - start;

                    // Detect label definition sites:
                    //   - identifier at column 0 (no leading whitespace on this line)
                    //     AND not a known keyword/directive
                    //   - OR identifier immediately followed by ':'
                    let followed_by_colon = col < bytes.len() && bytes[col] == b':';
                    let at_col_zero = start == 0;
                    let is_label_def = followed_by_colon
                        || (at_col_zero
                            && !instructions.contains(word_upper.as_str())
                            && !directives.contains(word_upper.as_str()));

                    let (tok_type, modifiers) =
                        if equ_names.contains(word_upper.as_str())
                            || assign_names.contains(word_upper.as_str())
                        {
                            (TT_ENUM_MEMBER, MOD_READONLY)
                        } else if macro_names.contains(word_upper.as_str()) {
                            (TT_FUNCTION, if is_label_def { MOD_DECLARATION } else { 0 })
                        } else if module_names.contains(word_upper.as_str()) {
                            (TT_NAMESPACE, if is_label_def { MOD_DECLARATION } else { 0 })
                        } else if instructions.contains(word_upper.as_str()) {
                            (TT_KEYWORD, 0)
                        } else if directives.contains(word_upper.as_str()) {
                            (TT_MACRO, 0)
                        } else if registers.contains(word_upper.as_str()) {
                            (TT_VARIABLE, 0)
                        } else if is_label_def {
                            (TT_FUNCTION, MOD_DECLARATION)
                        } else {
                            (TT_FUNCTION, 0) // label reference / unknown symbol
                        };

                    raw.push((line_u, start as u32, word_len as u32, tok_type, modifiers));

                    // Emit the ':' as an operator token
                    if followed_by_colon {
                        raw.push((line_u, col as u32, 1, TT_OPERATOR, 0));
                        col += 1;
                    }
                    continue;
                }

                // Single-character operators
                match c {
                    b'+' | b'-' | b'*' | b'/' | b'<' | b'>' | b'=' | b'!'
                    | b'&' | b'|' | b'^' | b'~' | b'#' | b'(' | b')'
                    | b'[' | b']' | b',' | b':' => {
                        raw.push((line_u, col as u32, 1, TT_OPERATOR, 0));
                    }
                    _ => {}
                }
                col += 1;
            }
        }

        // Convert raw (line, col, len, type, mods) to LSP delta-encoded SemanticToken
        let mut result = Vec::with_capacity(raw.len());
        let mut prev_line = 0u32;
        let mut prev_start = 0u32;
        for (line, start, len, tok_type, modifiers) in raw {
            let delta_line = line - prev_line;
            let delta_start = if delta_line == 0 { start - prev_start } else { start };
            result.push(SemanticToken {
                delta_line,
                delta_start,
                length: len,
                token_type: tok_type,
                token_modifiers_bitset: modifiers,
            });
            prev_line = line;
            prev_start = start;
        }
        result
    }

    // Helper methods

    fn extract_word_at_position(&self, line: &str, column: usize) -> Option<String> {
        let chars: Vec<char> = line.chars().collect();
        if column >= chars.len() {
            return None;
        }

        // Z80/basm identifier characters: alphanumeric, _, ., @
        // The dot allows `.local` labels and qualified names like `module.symbol`
        let is_word = |c: char| c.is_alphanumeric() || c == '_' || c == '.' || c == '@';

        let mut start = column;
        let mut end = column;

        while start > 0 && is_word(chars[start - 1]) {
            start -= 1;
        }
        while end < chars.len() && is_word(chars[end]) {
            end += 1;
        }

        if start < end {
            Some(chars[start..end].iter().collect())
        } else {
            None
        }
    }

}


impl Default for AssemblyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ─── hover helpers (free functions) ──────────────────────────────────────────

fn make_hover(md: String) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: md,
        }),
        range: None,
    }
}

/// Detect a numeric literal under `col` and return its text + i64 value.
/// Handles `$`, `&`, `#` (hex), `%` (binary), `0x`/`0b`/`0o`, and plain decimal.
fn extract_number_at_position(line: &str, col: usize) -> Option<(String, i64)> {
    let bytes = line.as_bytes();
    if col >= bytes.len() {
        return None;
    }
    let ch = bytes[col];
    let is_hex_digit =
        |b: u8| b.is_ascii_digit() || matches!(b, b'a'..=b'f' | b'A'..=b'F');
    let is_prefix = |b: u8| matches!(b, b'$' | b'%' | b'&' | b'#');

    // Cursor must be on a digit, hex letter, or a numeric prefix
    if !ch.is_ascii_digit() && !is_prefix(ch) && !is_hex_digit(ch) {
        return None;
    }

    // Scan backward over alphanumeric chars to find the token start
    let mut start = col;
    while start > 0 && bytes[start - 1].is_ascii_alphanumeric() {
        start -= 1;
    }
    // Include a single-char prefix ($, %, &, #) immediately before the digits
    if start > 0 && is_prefix(bytes[start - 1]) {
        start -= 1;
    }

    // Scan forward to end of token
    let body_start = if is_prefix(bytes[start]) { start + 1 } else { start };
    let mut end = body_start;
    // Consume a 0x / 0b / 0o prefix if present
    if end + 1 < bytes.len()
        && bytes[end] == b'0'
        && matches!(bytes[end + 1], b'x' | b'X' | b'b' | b'B' | b'o' | b'O')
    {
        end += 2;
    }
    while end < bytes.len() && bytes[end].is_ascii_alphanumeric() {
        end += 1;
    }

    if start >= end {
        return None;
    }
    let num_str = &line[start..end];

    let value: i64 = if let Some(h) = num_str
        .strip_prefix('$')
        .or_else(|| num_str.strip_prefix('&'))
        .or_else(|| num_str.strip_prefix('#'))
    {
        i64::from_str_radix(h, 16).ok()?
    } else if let Some(b) = num_str.strip_prefix('%') {
        i64::from_str_radix(b, 2).ok()?
    } else if let Some(h) = num_str
        .strip_prefix("0x")
        .or_else(|| num_str.strip_prefix("0X"))
    {
        i64::from_str_radix(h, 16).ok()?
    } else if let Some(b) = num_str
        .strip_prefix("0b")
        .or_else(|| num_str.strip_prefix("0B"))
    {
        i64::from_str_radix(b, 2).ok()?
    } else if let Some(o) = num_str
        .strip_prefix("0o")
        .or_else(|| num_str.strip_prefix("0O"))
    {
        i64::from_str_radix(o, 8).ok()?
    } else if num_str.bytes().all(|b| b.is_ascii_digit()) {
        num_str.parse().ok()?
    } else {
        return None;
    };

    Some((num_str.to_string(), value))
}

/// Format an i64 as a binary string with `_` every 4 bits.
/// Width is clamped to 8 or 16 bits for typical Z80 values.
fn format_binary(value: i64) -> String {
    let bits: u32 = if value >= 0 && value <= 0xFF { 8 } else { 16 };
    let mut s = String::with_capacity(bits as usize + bits as usize / 4);
    for i in (0..bits).rev() {
        if i < bits - 1 && i % 4 == 3 {
            s.push('_');
        }
        s.push(if value & (1 << i) != 0 { '1' } else { '0' });
    }
    s
}

fn register_description(upper: &str) -> Option<String> {
    let desc = match upper {
        "A"   => "**A** — Accumulator (8-bit). Primary register for arithmetic/logic.",
        "B"   => "**B** — 8-bit general purpose register.",
        "C"   => "**C** — 8-bit general purpose register. Also the carry condition code.",
        "D"   => "**D** — 8-bit general purpose register.",
        "E"   => "**E** — 8-bit general purpose register.",
        "H"   => "**H** — High byte of HL.",
        "L"   => "**L** — Low byte of HL.",
        "F"   => "**F** — Flags register (8-bit). Bits: S Z 5 H 3 P/V N C.",
        "BC"  => "**BC** — 16-bit register pair (B:C). Often used as counter or source address.",
        "DE"  => "**DE** — 16-bit register pair (D:E). Often used as destination pointer.",
        "HL"  => "**HL** — 16-bit register pair (H:L). Primary 16-bit address register.",
        "AF"  => "**AF** — Accumulator + Flags register pair.",
        "AF'" => "**AF'** — Alternate Accumulator + Flags register pair (shadow).",
        "IX"  => "**IX** — 16-bit index register X. Used with `(IX+d)` displacement addressing.",
        "IY"  => "**IY** — 16-bit index register Y. Used with `(IY+d)` displacement addressing.",
        "SP"  => "**SP** — Stack Pointer (16-bit). Points to the top of the hardware stack.",
        "PC"  => "**PC** — Program Counter (16-bit). Points to the next instruction to execute.",
        "I"   => "**I** — Interrupt vector register (8-bit). High byte of the IM 2 vector table address.",
        "R"   => "**R** — Memory Refresh register (8-bit). Auto-incremented each M1 machine cycle.",
        "IXH" => "**IXH** — High byte of IX (undocumented).",
        "IXL" => "**IXL** — Low byte of IX (undocumented).",
        "IYH" => "**IYH** — High byte of IY (undocumented).",
        "IYL" => "**IYL** — Low byte of IY (undocumented).",
        "NZ"  => "**NZ** — Condition code: not zero (Z=0).",
        "Z"   => "**Z** — Condition code: zero (Z=1).",
        "NC"  => "**NC** — Condition code: no carry (C=0).",
        "PE"  => "**PE** — Condition code: parity even / overflow set (P/V=1).",
        "PO"  => "**PO** — Condition code: parity odd / overflow clear (P/V=0).",
        "P"   => "**P** — Condition code: positive / sign clear (S=0).",
        "M"   => "**M** — Condition code: minus / sign set (S=1).",
        _     => return None,
    };
    Some(desc.to_string())
}
