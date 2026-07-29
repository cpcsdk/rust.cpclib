//! Goto-definition and references for assembly files: labels/symbols,
//! include-file navigation, embedded-BASIC line targets.

use cpclib_asm::parser::obtained::MayHaveSpan;
use cpclib_tokens::{ListingElement, Token};
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use super::embedded_basic::{block_and_text_at, extract_locomotive_blocks};
use super::token::is_ident_byte;
use crate::common::document::Document;

impl AssemblyAnalyzer {
    /// Find the definition of a symbol — looks up the word under the cursor in the parsed listing.
    pub fn goto_definition(&self, document: &Document, position: Position) -> Option<Location> {
        let line = document.line(position.line as usize)?;
        // `resolve_include_at` indexes by byte (it scans `line.as_bytes()`
        // directly), while `extract_word_at_position` below indexes by
        // `char` — these are two different conversions of the same UTF-16
        // `position.character`, not interchangeable.
        let byte_col = document.byte_column(position);

        // CTRL+CLICK on a filename string inside INCLUDE / INCBIN / BINCLUDE.
        if let Some(target_uri) = resolve_include_at(&line, byte_col, &document.uri) {
            return Some(Location {
                uri: target_uri,
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0
                    },
                    end: Position {
                        line: 0,
                        character: 0
                    }
                }
            });
        }

        // Delegate to bndbuild goto-definition for `#!bndbuild` embedded
        // block content.
        if let Some(loc) = self.embedded_bndbuild_goto_definition(document, position) {
            return Some(loc);
        }

        // Delegate to BASIC goto-definition for LOCOMOTIVE block content.
        {
            let text = document.text();
            let line_idx = position.line as usize;
            if let Some((block, basic_text)) = block_and_text_at(&text, line_idx) {
                return crate::locomotive::definition::locomotive_basic_goto_definition(
                    &basic_text,
                    position,
                    block.basic_range.start as u32,
                    &document.uri
                );
            }
        }

        let word = self.extract_word_at_position(&line, document.char_column(position))?;

        // The backend will try other open documents if this returns None.
        self.find_definition_in(document, &word, self.config().case_sensitive)
    }

    /// Extract the word (ASM identifier) under the cursor, or `None`.
    pub fn word_at_position(&self, document: &Document, position: Position) -> Option<String> {
        let line = document.line(position.line as usize)?;
        self.extract_word_at_position(&line, document.char_column(position))
    }

    /// `textDocument/prepareRename`: the range to offer renaming for, or
    /// `None` to reject (cursor not on a real label/BASIC variable).
    pub fn prepare_rename(&self, document: &Document, position: Position) -> Option<Range> {
        // Delegate to bndbuild for `#!bndbuild` embedded block content.
        if let Some(range) = self.embedded_bndbuild_prepare_rename(document, position) {
            return Some(range);
        }

        // Delegate to BASIC for LOCOMOTIVE block content (same
        // block-extraction dance as `goto_definition`/`hover`).
        let text = document.text();
        let line_idx = position.line as usize;
        if let Some((block, basic_text)) = block_and_text_at(&text, line_idx) {
            return crate::locomotive::definition::locomotive_basic_prepare_rename(
                &basic_text,
                position,
                block.basic_range.start as u32
            );
        }

        self.prepare_rename_label(document, position)
    }

    /// `textDocument/rename`: rename the label/BASIC variable under the
    /// cursor to `new_name`.
    pub fn rename(
        &self,
        document: &Document,
        position: Position,
        new_name: &str
    ) -> Option<WorkspaceEdit> {
        if let Some(edit) = self.embedded_bndbuild_rename(document, position, new_name) {
            return Some(edit);
        }

        let text = document.text();
        let line_idx = position.line as usize;
        if let Some((block, basic_text)) = block_and_text_at(&text, line_idx) {
            return crate::locomotive::definition::locomotive_basic_rename(
                &basic_text,
                position,
                block.basic_range.start as u32,
                &document.uri,
                new_name
            );
        }

        self.rename_label(document, position, new_name)
    }

    /// Delegates goto-definition to `BuildFileAnalyzer::goto_definition`
    /// when `position` is inside a `#!bndbuild` embedded block, against a
    /// synthetic `Document` wrapping just the block's own text. The
    /// returned `Location`'s range is only shifted back into outer-document
    /// coordinates when it still points at the synthetic doc's own uri
    /// (== the host `.asm` file) - a `Location` pointing at a genuinely
    /// different file (e.g. a `cmd:`-referenced on-disk path,
    /// `bndbuild::definition`'s own `goto_definition_on_a_cmd_argument_
    /// opens_the_file` test) already carries real, absolute coordinates and
    /// must pass through untouched.
    fn embedded_bndbuild_goto_definition(
        &self,
        document: &Document,
        position: Position
    ) -> Option<Location> {
        let blocks = self.embedded_bndbuild_blocks(document);
        let block = super::embedded_bndbuild::block_at(&blocks, position.line as usize)?;
        let local_pos = super::embedded_bndbuild::position_into_block(block, position)?;
        let block_doc = Document::new(document.uri.clone(), block.yaml_text.clone(), 0);
        let loc =
            crate::bndbuild::BuildFileAnalyzer::new().goto_definition(&block_doc, local_pos)?;
        Some(super::embedded_bndbuild::location_out_of_block(
            block,
            &document.uri,
            loc
        ))
    }

    /// As `embedded_bndbuild_goto_definition`, for `prepare_rename`.
    fn embedded_bndbuild_prepare_rename(
        &self,
        document: &Document,
        position: Position
    ) -> Option<Range> {
        let blocks = self.embedded_bndbuild_blocks(document);
        let block = super::embedded_bndbuild::block_at(&blocks, position.line as usize)?;
        let local_pos = super::embedded_bndbuild::position_into_block(block, position)?;
        let block_doc = Document::new(document.uri.clone(), block.yaml_text.clone(), 0);
        let range =
            crate::bndbuild::BuildFileAnalyzer::new().prepare_rename(&block_doc, local_pos)?;
        Some(super::embedded_bndbuild::range_out_of_block(block, range))
    }

    /// As `embedded_bndbuild_goto_definition`, for `rename`. Walks
    /// `WorkspaceEdit.changes` generically (not simply "grab the one
    /// entry") - `BuildFileAnalyzer::rename` only ever touches its own
    /// passed-in document today, but this doesn't assume that stays true -
    /// and shifts only the entry keyed by the synthetic doc's own uri
    /// (== `document.uri`).
    fn embedded_bndbuild_rename(
        &self,
        document: &Document,
        position: Position,
        new_name: &str
    ) -> Option<WorkspaceEdit> {
        let blocks = self.embedded_bndbuild_blocks(document);
        let block = super::embedded_bndbuild::block_at(&blocks, position.line as usize)?;
        let local_pos = super::embedded_bndbuild::position_into_block(block, position)?;
        let block_doc = Document::new(document.uri.clone(), block.yaml_text.clone(), 0);
        let edit =
            crate::bndbuild::BuildFileAnalyzer::new().rename(&block_doc, local_pos, new_name)?;

        let mut changes = edit.changes.unwrap_or_default();
        if let Some(edits) = changes.remove(&document.uri) {
            let shifted = edits
                .into_iter()
                .map(|e| {
                    TextEdit {
                        range: super::embedded_bndbuild::range_out_of_block(block, e.range),
                        new_text: e.new_text
                    }
                })
                .collect();
            changes.insert(document.uri.clone(), shifted);
        }
        Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        })
    }

    /// Search `document` for a definition of `word`.
    ///
    /// A *definition* is a label token, or a directive that assigns the symbol
    /// (`EQU` / `=`), or a macro/module declaration — never a mere reference
    /// (e.g. the operand of a `CALL`/`JR`).
    ///
    /// `case_sensitive` controls the symbol-NAME match only - basm labels
    /// are case-sensitive by default (`buffer`/`BUFFER` are different
    /// symbols), so goto-definition passes `true`. Some callers with their
    /// own established case-insensitive identity convention (e.g. call
    /// hierarchy, which canonicalizes label names to uppercase throughout
    /// its own data model) deliberately pass `false` to preserve their
    /// existing behavior - not something this function should force.
    ///
    /// Returns the first matching `Location`, or `None`.
    pub fn find_definition_in(
        &self,
        document: &Document,
        word: &str,
        case_sensitive: bool
    ) -> Option<Location> {
        if let Ok(listing) = self.parse_document(document) {
            for token in super::token::flatten_listing(listing.iter()) {
                let source_name: &str = if token.is_label() {
                    token.label_symbol()
                }
                else if token.is_equ() {
                    token.equ_symbol()
                }
                else if token.is_assign() {
                    token.assign_symbol()
                }
                else if token.is_macro_definition() {
                    // TODO add the same for struct
                    token.macro_definition_name()
                }
                else if token.is_module() {
                    token.module_name()
                }
                else {
                    // A section's *definition*: `RANGE`/`DEFSECTION` start,
                    // stop, name — the name is the last argument, so `word`
                    // here is what a `SECTION name` usage (or the definition
                    // itself) resolves to. `to_token()` is needed to extract it
                    // (no `is_range`/`range_*` accessor exists) and is only
                    // ever called for tokens that already look like a `RANGE`/
                    // `DEFSECTION` statement — see `starts_with_range_keyword`.
                    if token.is_directive()
                        && super::token::starts_with_range_keyword(token)
                        && let Token::Range(name, ..) = token.to_token().into_owned()
                        && names_match(&name, word, case_sensitive)
                    {
                        let (lsp_line, lsp_char) =
                            super::token::locate_name_in_statement(token, &name);
                        return Some(Location {
                            uri: document.uri.clone(),
                            range: Range {
                                start: Position {
                                    line: lsp_line,
                                    character: lsp_char
                                },
                                end: Position {
                                    line: lsp_line,
                                    character: lsp_char + name.len() as u32
                                }
                            }
                        });
                    }
                    continue;
                };
                if names_match(source_name, word, case_sensitive) {
                    let span = token.span();
                    let (line_1based, col_1based) = span.relative_line_and_column();
                    let lsp_line = line_1based.saturating_sub(1) as u32;
                    let lsp_char = col_1based.saturating_sub(1) as u32;
                    return Some(Location {
                        uri: document.uri.clone(),
                        range: Range {
                            start: Position {
                                line: lsp_line,
                                character: lsp_char
                            },
                            end: Position {
                                line: lsp_line,
                                character: lsp_char + source_name.len() as u32
                            }
                        }
                    });
                }
            }
        }

        // Text-based fallback, used both when the document does not fully
        // parse (goto-definition must keep working in files that don't
        // assemble yet, e.g. work-in-progress or disassembler output) and
        // when the parsed listing did not yield the symbol.
        self.find_definition_by_text(document, word, case_sensitive)
    }

    /// Line-oriented definition scan, used when the parsed listing yields
    /// nothing: matches `word:` / `word` at line start, and `word EQU ...` /
    /// `word = ...` anywhere the symbol starts the statement. `case_sensitive`
    /// controls only the symbol-name match - the `EQU`/`=` keyword check
    /// afterward is always case-insensitive, since directive keywords
    /// themselves are (independent of the symbol-name policy).
    fn find_definition_by_text(
        &self,
        document: &Document,
        word: &str,
        case_sensitive: bool
    ) -> Option<Location> {
        let text = document.text();
        let word_for_match = if case_sensitive {
            word.to_string()
        }
        else {
            word.to_uppercase()
        };
        for (line_idx, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            let indent = line.len() - trimmed.len();
            let trimmed_for_match = if case_sensitive {
                trimmed.to_string()
            }
            else {
                trimmed.to_uppercase()
            };

            if !trimmed_for_match.starts_with(word_for_match.as_str()) {
                continue;
            }
            // ASCII case-folding never changes byte length, so slicing the
            // original-case `trimmed` at `word_for_match`'s byte length
            // lands on the same boundary `strip_prefix` would have.
            let rest = &trimmed[word_for_match.len()..];
            // Must be a whole word.
            if rest.as_bytes().first().is_some_and(|&b| is_ident_byte(b)) {
                continue;
            }

            let rest_trimmed = rest.trim_start();
            let rest_trimmed_upper = rest_trimmed.to_uppercase();
            let is_label_def = (rest.starts_with(':') && !rest.starts_with("::"))
                || (indent == 0 && (rest_trimmed.is_empty() || rest_trimmed.starts_with(';')));
            let is_symbol_def = rest_trimmed_upper.starts_with("EQU")
                && !rest_trimmed
                    .as_bytes()
                    .get(3)
                    .is_some_and(|&b| is_ident_byte(b))
                || (rest_trimmed.starts_with('=') && !rest_trimmed.starts_with("=="));

            if is_label_def || is_symbol_def {
                return Some(Location {
                    uri: document.uri.clone(),
                    range: Range {
                        start: Position {
                            line: line_idx as u32,
                            character: indent as u32
                        },
                        end: Position {
                            line: line_idx as u32,
                            character: (indent + word.len()) as u32
                        }
                    }
                });
            }
        }
        None
    }

    /// TODO rename it find_reference_in_by_text and rewrite find_reference_in using the listing. The references will be stored in expressions
    /// Find all occurrences of `word_upper` (already uppercased) as whole words in `document`.
    pub fn find_references_in(&self, document: &Document, word_upper: &str) -> Vec<Location> {
        let text = document.text();
        let mut refs = Vec::new();
        for (line_idx, line) in text.lines().enumerate() {
            for (abs, after) in word_matches_on_line(line, word_upper) {
                refs.push(Location {
                    uri: document.uri.clone(),
                    range: Range {
                        start: Position {
                            line: line_idx as u32,
                            character: abs as u32
                        },
                        end: Position {
                            line: line_idx as u32,
                            character: after as u32
                        }
                    }
                });
            }
        }
        refs
    }

    /// Find all references to a symbol
    pub fn find_references(&self, document: &Document, position: Position) -> Vec<Location> {
        let word = match self.word_at_position(document, position) {
            Some(w) => w.to_uppercase(),
            None => return Vec::new()
        };
        self.find_references_in(document, &word)
    }

    fn prepare_rename_label(&self, document: &Document, position: Position) -> Option<Range> {
        let line = document.line(position.line as usize)?;
        let (word, start_col, end_col) =
            super::token::word_range_at_position(&line, document.char_column(position))?;
        let word_upper = word.to_uppercase();

        // A scoped local — a `.label`, a `FUNCTION` parameter/local, or a
        // `REPEAT`/`ITERATE` counter — is always renamable regardless of
        // whether its name happens to collide with a register/mnemonic/
        // directive keyword: basm allows this (a REPEAT counter named `i`
        // is extremely common and collides with the Z80 `I` register, but
        // is a completely distinct namespace in that context). Only a bare
        // word that falls back to a workspace-wide `Global` rename needs
        // the reserved-word check below — a register/mnemonic name showing
        // up there is essentially always genuine register/mnemonic use,
        // not a label.
        let is_scoped_local = matches!(
            self.resolve_rename_target(document, position),
            Some(
                RenameTarget::Local { .. }
                    | RenameTarget::FunctionLocal { .. }
                    | RenameTarget::LoopLocal { .. }
                    | RenameTarget::MacroLocal { .. }
                    | RenameTarget::Qualified(_)
            )
        );
        if !is_scoped_local
            && (super::token::INSTRUCTION_SET.contains(word_upper.as_str())
                || super::token::DIRECTIVE_SET.contains(word_upper.as_str())
                || super::token::REGISTER_SET.contains(word_upper.as_str()))
        {
            return None;
        }

        Some(Range {
            start: Position {
                line: position.line,
                character: crate::common::document::char_count_to_utf16_col(
                    &line,
                    start_col as usize
                ) as u32
            },
            end: Position {
                line: position.line,
                character: crate::common::document::char_count_to_utf16_col(&line, end_col as usize)
                    as u32
            }
        })
    }

    fn rename_label(
        &self,
        document: &Document,
        position: Position,
        new_name: &str
    ) -> Option<WorkspaceEdit> {
        let target = self.resolve_rename_target(document, position)?;

        let edits = self.rename_occurrences_in(document, &target, new_name);
        if edits.is_empty() {
            return None;
        }
        Some(WorkspaceEdit {
            changes: Some(std::collections::HashMap::from([(
                document.uri.clone(),
                edits
            )])),
            ..Default::default()
        })
    }

    /// What kind of rename the label/word under the cursor calls for — see
    /// [`RenameTarget`].
    pub(crate) fn resolve_rename_target(
        &self,
        document: &Document,
        position: Position
    ) -> Option<RenameTarget> {
        // Never treat BASIC content inside a LOCOMOTIVE block, or YAML
        // content inside a `#!bndbuild` embedded block, as a basm label —
        // `rename`/`prepare_rename` already delegate both cases to their
        // own analyzer (single-file), and callers use this method
        // specifically to decide whether a rename needs to expand beyond
        // the current document, which never applies to either.
        let text = document.text();
        let line_idx = position.line as usize;
        if extract_locomotive_blocks(&text)
            .iter()
            .any(|b| b.basic_range.contains(&line_idx))
            || super::embedded_bndbuild::block_at(
                &self.embedded_bndbuild_blocks(document),
                line_idx
            )
            .is_some()
        {
            return None;
        }

        let line = document.line(position.line as usize)?;
        let (word, ..) =
            super::token::word_range_at_position(&line, document.char_column(position))?;

        if let Some(local) = word.strip_prefix('.') {
            let listing = self.parse_document(document).ok()?;
            let (owner, scope) = super::token::label_scope_at_line(listing.iter(), position.line)?;
            return Some(RenameTarget::Local {
                owner,
                name: local.to_string(),
                scope
            });
        }

        if word.contains('.') {
            return Some(RenameTarget::Qualified(word));
        }

        let word_upper = word.to_uppercase();
        let listing = self.parse_document(document).ok();

        // A `REPEAT`/`ITERATE` loop's own counter variable, used bare
        // within its body — scoped to that loop, checked first since a
        // loop nested inside a `FUNCTION` sharing a name with one of the
        // function's own parameters should resolve to the (more deeply
        // nested) loop counter, matching normal lexical scoping.
        if let Some(listing) = &listing
            && let Some((keyword, name, scope)) = super::token::loop_scoped_symbol_at(
                listing.iter(),
                &text,
                position.line,
                &word_upper
            )
        {
            return Some(RenameTarget::LoopLocal {
                keyword,
                name,
                scope
            });
        }

        // A `FUNCTION`'s own parameter, used bare within its body — scoped
        // to that function, checked before falling back to a workspace-wide
        // `Global` rename (a common parameter name like `x` isn't meant to
        // rename every unrelated `x` in the workspace).
        if let Some(listing) = &listing
            && let Some((function_name, name, scope)) = super::token::function_scoped_symbol_at(
                listing.iter(),
                &text,
                position.line,
                &word_upper
            )
        {
            return Some(RenameTarget::FunctionLocal {
                function_name,
                name,
                scope
            });
        }

        // A `MACRO`'s own declared parameter, used bare (or `{param}`
        // brace-interpolated) within its body — scoped to that macro, same
        // reasoning as `FUNCTION` parameters above.
        if let Some(listing) = &listing
            && let Some((macro_name, name, scope)) = super::token::macro_scoped_symbol_at(
                listing.iter(),
                &text,
                position.line,
                &word_upper
            )
        {
            return Some(RenameTarget::MacroLocal {
                macro_name,
                name,
                scope
            });
        }

        Some(RenameTarget::Global(word))
    }

    /// The `TextEdit`s `target`'s rename produces *within `document`* —
    /// callers handling `RenameTarget::Global` are expected to call this
    /// once per file across the workspace and merge the results;
    /// `RenameTarget::Local` is only ever meaningful for the one document
    /// its scope was computed against.
    pub(crate) fn rename_occurrences_in(
        &self,
        document: &Document,
        target: &RenameTarget,
        new_name: &str
    ) -> Vec<TextEdit> {
        match target {
            RenameTarget::Global(old) => {
                // Exclude any `FUNCTION`/`REPEAT`/`ITERATE`/`MACRO` body in
                // *this* document that shadows `old` as its own parameter/
                // counter or a local `EQU`/`=` symbol — inside such a
                // block, `old` refers to that local definition, a different
                // symbol entirely, and a rename of the outer one must not
                // touch it (mirrors `RenameTarget::FunctionLocal`/
                // `LoopLocal`/`MacroLocal`'s scoping in reverse).
                let old_upper = old.to_uppercase();
                let text = document.text();
                let shadow_ranges: Vec<std::ops::Range<u32>> = self
                    .parse_document(document)
                    .ok()
                    .map(|listing| {
                        super::token::all_function_shadow_ranges(listing.iter(), &text, &old_upper)
                            .into_iter()
                            .chain(super::token::all_loop_shadow_ranges(
                                listing.iter(),
                                &text,
                                &old_upper
                            ))
                            .chain(super::token::all_macro_shadow_ranges(
                                listing.iter(),
                                &text,
                                &old_upper
                            ))
                            .collect()
                    })
                    .unwrap_or_default();

                find_label_word_and_prefix_matches(document, old)
                    .into_iter()
                    .filter(|(range, _)| {
                        !shadow_ranges
                            .iter()
                            .any(|scope| scope.contains(&range.start.line))
                    })
                    .map(|(range, matched)| {
                        let suffix = &matched[old.len()..]; // "" or ".rest"
                        TextEdit {
                            range,
                            new_text: format!("{new_name}{suffix}")
                        }
                    })
                    .collect()
            },
            RenameTarget::Qualified(old) => {
                self.find_references_in(document, &old.to_uppercase())
                    .into_iter()
                    .map(|loc| {
                        TextEdit {
                            range: loc.range,
                            new_text: new_name.to_string()
                        }
                    })
                    .collect()
            },
            RenameTarget::Local { name, scope, .. } => {
                // `prepare_rename`'s range covers the leading `.` (it's
                // part of the word), so an LSP client's pre-filled rename
                // input typically includes it too (`.foo` rather than just
                // `foo`) — strip it back off if present, or `new_text`
                // would end up double-dotted (`..bar`).
                let new_name = new_name.strip_prefix('.').unwrap_or(new_name);
                find_local_matches_in_scope(document, name, scope)
                    .into_iter()
                    .map(|range| {
                        TextEdit {
                            range,
                            new_text: format!(".{new_name}")
                        }
                    })
                    .collect()
            },
            // `FUNCTION` bodies have a restricted grammar with no real Z80
            // instructions, so a bare reference to a parameter/local is
            // unambiguous.
            RenameTarget::FunctionLocal { name, scope, .. } => {
                find_bare_word_matches_in_scope(document, name, scope)
                    .into_iter()
                    .map(|range| {
                        TextEdit {
                            range,
                            new_text: new_name.to_string()
                        }
                    })
                    .collect()
            },
            // `REPEAT`/`ITERATE`/`MACRO` bodies can contain arbitrary Z80
            // code, where a bare single-letter name can equally be a real
            // register (`ld a, {a}`) — only the declaration itself is
            // matched bare, every other reference must be an explicit
            // `{name}` interpolation (basm's own convention here, and what
            // real code — including the reported case — actually writes).
            RenameTarget::LoopLocal { name, scope, .. }
            | RenameTarget::MacroLocal { name, scope, .. } => {
                find_scoped_local_matches(document, name, scope)
                    .into_iter()
                    .map(|(range, is_braced)| {
                        TextEdit {
                            range,
                            new_text: if is_braced {
                                format!("{{{new_name}}}")
                            }
                            else {
                                new_name.to_string()
                            }
                        }
                    })
                    .collect()
            },
        }
    }
}

/// What kind of rename a resolved word calls for, mirroring basm's own
/// label-scoping rules (`handle_global_and_local_labels` in
/// `cpclib-asm/src/assembler/mod.rs`):
///
/// - `Global`: a bare label with no leading `.` — rename is workspace-wide,
///   covering both plain occurrences and `.`-qualified references to its
///   own locals (`OLD.foo` → `NEW.foo`).
/// - `Local`: a bare `.foo` reference/definition — confined to the
///   `scope` (line range) of the *specific* enclosing global it was
///   resolved against; a same-named local under a different global is a
///   different symbol and must not be touched.
/// - `Qualified`: an already-`global.local`-qualified compound word — text
///   extraction can't tell whether the cursor meant the global or the local
///   part, so this renames the exact qualified string, workspace-wide,
///   without trying to decompose it.
/// - `FunctionLocal`: a bare word that's either a declared parameter of the
///   `FUNCTION` enclosing the cursor, or a symbol `EQU`/`=`-defined within
///   its body (basm functions can't contain genuine label definitions —
///   `ParsingState::FunctionLimited` only accepts `Equ`/`Let` — so those are
///   the only two shapes a function-local symbol takes) — confined to that
///   function's own body (`FUNCTION` line through `ENDFUNCTION`, inclusive),
///   checked *before* falling back to `Global` so a common name (`x`,
///   `value`, ...) doesn't trigger an unrelated workspace-wide rename, and
///   a same-named definition *outside* the function is a different symbol
///   that must not be touched.
/// - `LoopLocal`: a bare word matching the counter variable of the
///   `REPEAT`/`ITERATE` loop enclosing the cursor (`REPEAT count, counter,
///   ...` / `ITERATE counter, ...`) — same treatment as `FunctionLocal`,
///   confined to the loop's own body, checked *first* (a loop nested inside
///   a `FUNCTION` takes precedence over an outer parameter of the same
///   name, matching normal lexical scoping).
/// - `MacroLocal`: a bare word matching a declared parameter of the
///   `MACRO` enclosing the cursor — same treatment as `FunctionLocal`,
///   confined to the macro's own body (`MACRO` line through
///   `ENDM`/`ENDMACRO`/`MEND`). Unlike `FUNCTION`, a `MACRO` body has no
///   restricted grammar of its own (it's pure text substitution — any
///   `EQU`/label inside becomes part of the real program at the call site
///   once expanded), so only declared parameters are checked, not
///   body-defined symbols.
#[derive(Debug)]
pub(crate) enum RenameTarget {
    Global(String),
    Local {
        owner: String,
        name: String,
        scope: std::ops::Range<u32>
    },
    Qualified(String),
    FunctionLocal {
        function_name: String,
        name: String,
        scope: std::ops::Range<u32>
    },
    LoopLocal {
        keyword: String,
        name: String,
        scope: std::ops::Range<u32>
    },
    MacroLocal {
        macro_name: String,
        name: String,
        scope: std::ops::Range<u32>
    }
}

/// `a == b`, either exactly (`case_sensitive`) or ASCII-case-insensitively -
/// used by `find_definition_in`/`find_definition_by_text` for the
/// symbol-NAME comparison specifically (basm labels are case-sensitive by
/// default; see those functions' own doc comments for why a caller might
/// deliberately pass `false`).
fn names_match(a: &str, b: &str, case_sensitive: bool) -> bool {
    if case_sensitive {
        a == b
    }
    else {
        a.eq_ignore_ascii_case(b)
    }
}

/// Case-insensitive occurrences of `pattern_upper` (already uppercased) in
/// `line`, as byte-offset `[start, start+len)` pairs — only the leading
/// word-boundary check (`before_ok`: not preceded by another identifier
/// character) is applied here. The shared core every scanner in this file
/// used to reimplement independently: `word_matches_on_line` (below)
/// builds on this by also checking the trailing boundary, for the four
/// scanners that need a strict "whole word" match;
/// `find_label_word_and_prefix_matches` uses this lower-level function
/// directly since it needs its own, different trailing-boundary rule (a
/// match immediately followed by `.` must be *accepted*, not rejected —
/// `.` is itself an `is_ident_byte` character).
fn pattern_starts_on_line(line: &str, pattern_upper: &str) -> Vec<(usize, usize)> {
    let line_up = line.to_uppercase();
    let bytes = line.as_bytes();
    let plen = pattern_upper.len();
    let mut out = Vec::new();
    let mut start = 0;
    while start + plen <= line_up.len() {
        let Some(pos) = line_up[start..].find(pattern_upper)
        else {
            break;
        };
        let abs = start + pos;
        let before_ok = abs == 0 || !is_ident_byte(bytes[abs - 1]);
        if before_ok {
            out.push((abs, abs + plen));
        }
        start = abs + 1;
    }
    out
}

/// As `pattern_starts_on_line`, additionally requiring the trailing
/// boundary too (not immediately followed by another identifier
/// character) — the strict "whole word" match `find_references_in`,
/// `find_local_matches_in_scope`, `find_bare_word_matches_in_scope`, and
/// `bare_word_matches_on_line` all need.
fn word_matches_on_line(line: &str, pattern_upper: &str) -> Vec<(usize, usize)> {
    let bytes = line.as_bytes();
    pattern_starts_on_line(line, pattern_upper)
        .into_iter()
        .filter(|&(_, after)| after >= bytes.len() || !is_ident_byte(bytes[after]))
        .collect()
}

/// Occurrences of `word_upper` in `document`, either as an exact whole word
/// or as a `.`-qualifying prefix (`word_upper` immediately followed by `.`
/// and further identifier characters, e.g. matching the `OLD` in
/// `OLD.local`) — used for global label rename, which must also update
/// qualified references to its own locals. Returns `(range, matched_text)`;
/// `matched_text` is exactly `word_upper` for a plain match, or
/// `word_upper` plus its `.suffix` for a qualified-prefix match.
fn find_label_word_and_prefix_matches(document: &Document, word: &str) -> Vec<(Range, String)> {
    // Callers pass the word as written (whatever case the source uses) —
    // matching must be case-insensitive regardless, like every other
    // symbol lookup in this module (`find_references_in`,
    // `find_local_matches_in_scope`).
    let word_upper = word.to_uppercase();
    let text = document.text();
    let mut matches = Vec::new();
    for (line_idx, line) in text.lines().enumerate() {
        let bytes = line.as_bytes();
        for (abs, after) in pattern_starts_on_line(line, &word_upper) {
            if after < bytes.len() && bytes[after] == b'.' {
                let mut end = after + 1;
                while end < bytes.len() && is_ident_byte(bytes[end]) {
                    end += 1;
                }
                matches.push((
                    Range {
                        start: Position {
                            line: line_idx as u32,
                            character: abs as u32
                        },
                        end: Position {
                            line: line_idx as u32,
                            character: end as u32
                        }
                    },
                    line[abs..end].to_string()
                ));
            }
            else if after >= bytes.len() || !is_ident_byte(bytes[after]) {
                matches.push((
                    Range {
                        start: Position {
                            line: line_idx as u32,
                            character: abs as u32
                        },
                        end: Position {
                            line: line_idx as u32,
                            character: after as u32
                        }
                    },
                    word_upper.clone()
                ));
            }
        }
    }
    matches
}

/// Occurrences of the bare local label `.name` (case-insensitive) within
/// `scope` (a line range, exclusive end) in `document` — used for local
/// label rename, confined to its owning global's scope.
fn find_local_matches_in_scope(
    document: &Document,
    name: &str,
    scope: &std::ops::Range<u32>
) -> Vec<Range> {
    let pattern = format!(".{}", name.to_uppercase());
    let mut matches = Vec::new();
    for line_idx in scope.start..scope.end {
        let Some(line) = document.line(line_idx as usize)
        else {
            break;
        };
        let line = line.trim_end_matches(['\n', '\r']);
        for (abs, after) in word_matches_on_line(line, &pattern) {
            matches.push(Range {
                start: Position {
                    line: line_idx,
                    character: abs as u32
                },
                end: Position {
                    line: line_idx,
                    character: after as u32
                }
            });
        }
    }
    matches
}

/// Occurrences of the bare word `name` (case-insensitive, no leading `.`)
/// within `scope` (a line range, exclusive end) in `document` — used for
/// `FUNCTION` parameter rename, confined to the function's own body.
fn find_bare_word_matches_in_scope(
    document: &Document,
    name: &str,
    scope: &std::ops::Range<u32>
) -> Vec<Range> {
    let name_upper = name.to_uppercase();
    let mut matches = Vec::new();
    for line_idx in scope.start..scope.end {
        let Some(line) = document.line(line_idx as usize)
        else {
            break;
        };
        let line = line.trim_end_matches(['\n', '\r']);
        for (abs, after) in word_matches_on_line(line, &name_upper) {
            matches.push(Range {
                start: Position {
                    line: line_idx,
                    character: abs as u32
                },
                end: Position {
                    line: line_idx,
                    character: after as u32
                }
            });
        }
    }
    matches
}

/// Whole-word (case-insensitive) matches of `name_upper` (already
/// uppercased) on a single `line`, as `[start, end)` column pairs.
fn bare_word_matches_on_line(line: &str, name_upper: &str) -> Vec<(u32, u32)> {
    word_matches_on_line(line, name_upper)
        .into_iter()
        .map(|(s, e)| (s as u32, e as u32))
        .collect()
}

/// `{name_upper}` (case-insensitive, braces required) matches on a single
/// `line`, as `[start, end)` column pairs covering the braces themselves.
fn braced_word_matches_on_line(line: &str, name_upper: &str) -> Vec<(u32, u32)> {
    let pattern = format!("{{{name_upper}}}");
    let line_up = line.to_uppercase();
    let plen = pattern.len();
    let mut cols = Vec::new();
    let mut start = 0;
    while start + plen <= line_up.len() {
        let Some(pos) = line_up[start..].find(pattern.as_str())
        else {
            break;
        };
        let abs = start + pos;
        cols.push((abs as u32, (abs + plen) as u32));
        start = abs + 1;
    }
    cols
}

/// Occurrences of `name` relevant for `MACRO`/`REPEAT`/`ITERATE` parameter/
/// counter rename, as `(range, is_braced)` pairs — `is_braced` tells the
/// caller whether the replacement needs to be re-wrapped in `{}` (an
/// interpolation reference) or left bare (the declaration itself).
///
/// Unlike `FUNCTION` (whose body has a restricted grammar with no real
/// instructions, so a bare `x` is unambiguous), a `MACRO`/`REPEAT`/
/// `ITERATE` body can contain arbitrary Z80 code, where a bare
/// single-letter name like `a` can equally be the real accumulator
/// register (`ld a, {a}` — the first `a` is the register, the second is
/// the parameter). So only the *declaration* line's own occurrence
/// (unambiguous: a comma/paren-delimited name list, never an instruction
/// operand) is matched bare; every other occurrence must be an explicit
/// `{name}` interpolation — basm's actual convention for referencing these
/// values from within arbitrary code.
fn find_scoped_local_matches(
    document: &Document,
    name: &str,
    scope: &std::ops::Range<u32>
) -> Vec<(Range, bool)> {
    let name_upper = name.to_uppercase();
    let mut matches = Vec::new();
    for line_idx in scope.start..scope.end {
        let Some(line) = document.line(line_idx as usize)
        else {
            break;
        };
        let line = line.trim_end_matches(['\n', '\r']);
        let cols_and_braced: Vec<(u32, u32, bool)> = if line_idx == scope.start {
            bare_word_matches_on_line(line, &name_upper)
                .into_iter()
                .map(|(s, e)| (s, e, false))
                .collect()
        }
        else {
            braced_word_matches_on_line(line, &name_upper)
                .into_iter()
                .map(|(s, e)| (s, e, true))
                .collect()
        };
        for (start_col, end_col, is_braced) in cols_and_braced {
            matches.push((
                Range {
                    start: Position {
                        line: line_idx,
                        character: start_col
                    },
                    end: Position {
                        line: line_idx,
                        character: end_col
                    }
                },
                is_braced
            ));
        }
    }
    matches
}

// ─── Include file navigation ──────────────────────────────────────────────────

const INCLUDE_DIRECTIVES: &[&str] = &["INCLUDE", "INCBIN", "BINCLUDE"];

/// Directory-level markers that indicate the project root.  We stop walking
/// up the ancestor tree when we find one of these in the current directory.
const PROJECT_ROOT_MARKERS: &[&str] = &[
    ".git",
    ".hg",
    "Cargo.toml",
    "Cargo.lock",
    "Makefile",
    "makefile"
];

/// If `col` is inside a double-quoted string on a line that starts with an
/// include-like directive, return the resolved file URI.
fn resolve_include_at(line: &str, col: usize, doc_uri: &Url) -> Option<Url> {
    let filename = include_filename_at(line, col)?;
    let path = resolve_include_path(&filename, doc_uri)?;
    Url::from_file_path(path).ok()
}

/// If `col` is inside a double-quoted string on a line that starts with an
/// include-like directive (`INCLUDE`/`INCBIN`/`BINCLUDE`), return the raw
/// filename text (unresolved — may be a relative on-disk path or an
/// `inner://...` embedded-resource reference). Shared by ctrl+click
/// navigation (`resolve_include_at`) and hover content preview.
pub(super) fn include_filename_at(line: &str, col: usize) -> Option<String> {
    include_directive_and_filename_at(line, col).map(|(_directive, filename)| filename)
}

/// As [`include_filename_at`], but also reports which directive matched
/// (`"INCLUDE"`/`"INCBIN"`/`"BINCLUDE"`) — hover needs to tell them apart:
/// `INCBIN` targets are raw binary data and get a hex/ASCII dump instead of
/// a text preview.
pub(super) fn include_directive_and_filename_at(
    line: &str,
    col: usize
) -> Option<(&'static str, String)> {
    let bytes = line.as_bytes();
    if col >= bytes.len() {
        return None;
    }

    // Find the `"..."` string that contains (or starts at) `col`.
    let (str_start, str_end) = find_quoted_string(bytes, col)?;
    let filename = &line[str_start + 1..str_end]; // strip surrounding quotes

    // The part before the string must end with a recognised include keyword.
    let before = line[..str_start].trim().to_uppercase();
    let directive = INCLUDE_DIRECTIVES.iter().find(|d| {
        before == **d || before.ends_with(&format!(" {d}")) || before.ends_with(&format!("\t{d}"))
    })?;

    Some((directive, filename.to_string()))
}

/// Every ancestor directory of `doc_uri`'s own directory, closest first, up
/// to and including the directory containing a project-root marker (or the
/// filesystem root) — the same walk [`resolve_include_path`] performs when
/// looking for one specific file, generalized into a search-path list.
///
/// Needed because real `include`s are conventionally written relative to a
/// project root, not the including file's own directory — e.g. a file at
/// `linking/src/hbl_inner.asm` doing `include 'src/demosystem/foo.asm'`
/// means relative to `linking/`, not `linking/src/`. A single directory
/// (just the file's own) isn't enough to resolve that.
pub(super) fn ancestor_search_directories(doc_uri: &Url) -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    let Ok(doc_path) = doc_uri.to_file_path()
    else {
        return dirs;
    };
    let Some(mut dir) = doc_path.parent().map(std::path::Path::to_path_buf)
    else {
        return dirs;
    };
    loop {
        let at_root = PROJECT_ROOT_MARKERS.iter().any(|m| dir.join(m).exists());
        dirs.push(dir.clone());
        if at_root {
            break;
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => break
        }
    }
    dirs
}

/// Walk up from `doc_uri`'s directory, trying each ancestor as a base for
/// `filename`, stopping once a project-root marker or the filesystem root is
/// reached. Shared by ctrl+click include navigation and the eager
/// cross-file goto-definition fallback.
pub fn resolve_include_path(filename: &str, doc_uri: &Url) -> Option<std::path::PathBuf> {
    let doc_path = doc_uri.to_file_path().ok()?;
    let mut dir = doc_path.parent()?;
    loop {
        let candidate = dir.join(filename);
        if candidate.exists() {
            return Some(candidate);
        }
        // If this directory contains a project-root marker, don't go further up.
        let at_root = PROJECT_ROOT_MARKERS.iter().any(|m| dir.join(m).exists());
        match dir.parent() {
            Some(parent) if !at_root => dir = parent,
            _ => break
        }
    }
    None
}

/// Every filename referenced by an `INCLUDE`/`INCBIN`/`BINCLUDE` directive in
/// `text`, in document order. Best-effort text scan (recognizes the same
/// directives as `resolve_include_at`, but line-anchored rather than
/// cursor-anchored so the whole file can be scanned in one pass).
pub fn extract_include_filenames(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let upper = trimmed.to_uppercase();

        let Some(directive) = INCLUDE_DIRECTIVES.iter().find(|d| {
            upper == **d
                || upper.starts_with(&format!("{d} "))
                || upper.starts_with(&format!("{d}\t"))
        })
        else {
            continue;
        };

        let after = &trimmed[directive.len()..];
        let Some(q1) = after.find('"')
        else {
            continue;
        };
        let Some(q2_rel) = after[q1 + 1..].find('"')
        else {
            continue;
        };
        out.push(after[q1 + 1..q1 + 1 + q2_rel].to_string());
    }
    out
}

/// The nearest ancestor directory of `doc_uri` containing a project-root
/// marker (`.git`, `Cargo.toml`, ...), or the document's own directory if no
/// marker is found anywhere up to the filesystem root. Used as the search
/// base for the eager cross-file goto-definition fallback when the LSP
/// client didn't report any workspace folder.
pub fn project_root_for(doc_uri: &Url) -> Option<std::path::PathBuf> {
    let doc_path = doc_uri.to_file_path().ok()?;
    let own_dir = doc_path.parent()?.to_path_buf();
    let mut dir = own_dir.clone();
    loop {
        if PROJECT_ROOT_MARKERS.iter().any(|m| dir.join(m).exists()) {
            return Some(dir);
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => return Some(own_dir)
        }
    }
}

/// Find the byte range of the quoted string `"..."` that covers position `col`.
/// Returns `(open_quote_pos, close_quote_pos)` where both positions are byte indices.
fn find_quoted_string(bytes: &[u8], col: usize) -> Option<(usize, usize)> {
    // Scan leftward to find the opening quote.
    let open = (0..=col).rev().find(|&i| bytes[i] == b'"')?;
    // Scan rightward to find the closing quote.
    let close = (col + 1..bytes.len()).find(|&i| bytes[i] == b'"')?;
    // `col` must be inside or on the opening/closing quote.
    if col >= open && col <= close {
        Some((open, close))
    }
    else {
        None
    }
}

#[cfg(test)]
mod definition_tests {
    use super::*;

    #[test]
    fn label_definition_found_not_reference() {
        let text = r#"
        call    nz,output_char    ;{{c390:c4a0c3}} ; display text char
        jr      nz,_output_asciiz_string_2;{{c393:20f8}}  (-$08)

        pop     hl                ;{{c395:e1}}
        pop     af                ;{{c396:f1}}
        ret                       ;{{c397:c9}}
output_char:                      ;{{Addr=$c3a0 Code Calls/jump count: 12 Data
        ret
"#;
        let uri = tower_lsp::lsp_types::Url::parse("file:///test.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        let loc = analyzer.find_definition_in(&doc, "output_char", true);
        assert!(loc.is_some(), "definition of output_char should be found");
        let loc = loc.unwrap();
        assert_eq!(
            loc.range.start.line, 7,
            "definition is the label line, not the call reference"
        );
    }

    #[test]
    fn label_definition_found_despite_parse_error_elsewhere() {
        // The `!!!` line does not assemble; goto-definition must still work.
        let text = "        call nz,output_char\n        !!! invalid line !!!\noutput_char:\n        ret\n";
        let uri = tower_lsp::lsp_types::Url::parse("file:///test2.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        let loc = analyzer.find_definition_in(&doc, "output_char", true);
        assert!(
            loc.is_some(),
            "definition should be found even with parse errors"
        );
        assert_eq!(loc.unwrap().range.start.line, 2);
    }

    /// Regression test for treating `position.character` (UTF-16 code
    /// units) as a raw `char` count: a supplementary-plane character (😀,
    /// 2 UTF-16 units but 1 `char`) earlier on the line desyncs the two by
    /// one. Positioned so the desync pushes the *uncorrected* char index
    /// out of bounds entirely (`column >= chars.len()`), not just to a
    /// different offset within the same word - so a regression here fails
    /// outright (`None`) rather than silently landing on the right word by
    /// luck.
    #[test]
    fn goto_definition_handles_utf16_columns_with_a_supplementary_plane_char_before_it() {
        let text = "GOAL:\n  ret\n  ; \u{1F600} call GOAL";
        let uri = tower_lsp::lsp_types::Url::parse("file:///utf16.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        // UTF-16 column 15 lands on the last char of "GOAL" once correctly
        // converted (char index 14); treated as a raw char index it's
        // `>= chars.len()` (15) and finds nothing.
        let loc = analyzer
            .goto_definition(
                &doc,
                Position {
                    line: 2,
                    character: 15
                }
            )
            .expect("goto-definition should resolve GOAL despite the emoji earlier on the line");
        assert_eq!(loc.range.start.line, 0);
    }

    #[test]
    fn equ_and_assign_definitions_found() {
        let text = "        ld a,(screen_base)\nscreen_base equ 0xC000\nother_sym = 12\n        ld hl,other_sym\n";
        let uri = tower_lsp::lsp_types::Url::parse("file:///test3.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        let loc = analyzer.find_definition_in(&doc, "screen_base", true);
        assert_eq!(loc.expect("equ definition").range.start.line, 1);
        let loc = analyzer.find_definition_in(&doc, "other_sym", true);
        assert_eq!(loc.expect("= definition").range.start.line, 2);
    }

    /// Regression test: a label wrapped in an `ifndef ... endif` header
    /// guard must still be reachable via goto-definition — `listing.iter()`
    /// alone only sees the top-level `IF` token, not what's inside it.
    #[test]
    fn definition_wrapped_in_an_ifndef_guard_is_found() {
        let text = "    ifndef GUARD\nGUARDED_LABEL:\n    ret\n    endif\n";
        let uri = tower_lsp::lsp_types::Url::parse("file:///guarded.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        let loc = analyzer.find_definition_in(&doc, "GUARDED_LABEL", true);
        assert_eq!(loc.expect("label inside ifndef").range.start.line, 1);
    }

    /// A `SECTION name` usage must jump to the `RANGE`/`DEFSECTION` line
    /// that defines that section name (the last argument), landing on the
    /// name itself rather than on the `RANGE`/`DEFSECTION` keyword.
    #[test]
    fn section_usage_jumps_to_its_range_definition() {
        let text = "RANGE 0x4000, 0x8000, MY_SECTION\nSECTION MY_SECTION\n    ret\n";
        let uri = tower_lsp::lsp_types::Url::parse("file:///sect.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        let loc = analyzer
            .find_definition_in(&doc, "MY_SECTION", true)
            .expect("section definition");
        assert_eq!(loc.range.start.line, 0, "{loc:?}");
        assert_eq!(loc.range.start.character, 22, "{loc:?}");
        assert_eq!(loc.range.end.character, 32, "{loc:?}");
    }

    /// Regression test for the reported bug: basm is case-sensitive by
    /// default, so `buffer` and `BUFFER` are two distinct symbols - looking
    /// up one must never resolve to the other's declaration.
    #[test]
    fn case_sensitive_lookup_does_not_confuse_differently_cased_symbols() {
        let text = "buffer:\n    ret\nBUFFER:\n    ret\n";
        let uri = tower_lsp::lsp_types::Url::parse("file:///case.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();

        let loc = analyzer
            .find_definition_in(&doc, "buffer", true)
            .expect("lowercase declaration");
        assert_eq!(loc.range.start.line, 0, "{loc:?}");

        let loc = analyzer
            .find_definition_in(&doc, "BUFFER", true)
            .expect("uppercase declaration");
        assert_eq!(loc.range.start.line, 2, "{loc:?}");

        // A case that matches neither declaration must not fall back to
        // either one.
        assert!(analyzer.find_definition_in(&doc, "Buffer", true).is_none());
    }

    /// Same fix, exercised through the text-only fallback path
    /// (`find_definition_by_text`) rather than the parsed-listing path -
    /// a syntax error elsewhere forces the fallback, per
    /// `label_definition_found_despite_parse_error_elsewhere` above.
    #[test]
    fn case_sensitive_lookup_holds_in_the_text_only_fallback_too() {
        let text = "buffer:\n    ret\n!!! invalid line !!!\nBUFFER:\n    ret\n";
        let uri = tower_lsp::lsp_types::Url::parse("file:///case2.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();

        let loc = analyzer
            .find_definition_in(&doc, "buffer", true)
            .expect("lowercase declaration");
        assert_eq!(loc.range.start.line, 0, "{loc:?}");

        let loc = analyzer
            .find_definition_in(&doc, "BUFFER", true)
            .expect("uppercase declaration");
        assert_eq!(loc.range.start.line, 3, "{loc:?}");
    }

    #[test]
    fn goto_definition_from_a_section_usage_position_finds_the_range_line() {
        let text = "RANGE 0x4000, 0x8000, MY_SECTION\nSECTION MY_SECTION\n    ret\n";
        let uri = tower_lsp::lsp_types::Url::parse("file:///sect2.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        // Cursor on "MY_SECTION" within the `SECTION MY_SECTION` usage line.
        let loc = analyzer
            .goto_definition(
                &doc,
                Position {
                    line: 1,
                    character: 10
                }
            )
            .expect("goto-definition from a SECTION usage");
        assert_eq!(loc.range.start.line, 0, "{loc:?}");
    }

    #[test]
    fn goto_definition_on_the_range_name_itself_returns_its_own_position() {
        let text = "RANGE 0x4000, 0x8000, MY_SECTION\n    ret\n";
        let uri = tower_lsp::lsp_types::Url::parse("file:///sect3.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        let loc = analyzer
            .goto_definition(
                &doc,
                Position {
                    line: 0,
                    character: 25
                }
            )
            .expect("goto-definition on the section name inside its own definition");
        assert_eq!(loc.range.start.line, 0, "{loc:?}");
        assert_eq!(loc.range.start.character, 22, "{loc:?}");
    }

    /// Goto-definition on a `dep:` reference inside a `#!bndbuild` embedded
    /// block must delegate to bndbuild's own goto-definition and shift the
    /// resulting line back into the host `.asm` file's coordinates.
    #[test]
    fn goto_definition_on_a_dep_inside_an_embedded_block_resolves_with_a_shifted_line() {
        let text = "; #!bndbuild\n\
                     ; - tgt: helper.o\n\
                     ;   cmd: basm helper.asm -o helper.o\n\
                     ; - tgt: out.bin\n\
                     ;   dep:\n\
                     ;     - helper.o\n\
                     ;   cmd: link helper.o\n";
        let uri = tower_lsp::lsp_types::Url::parse("file:///embedded.asm").unwrap();
        let doc = crate::common::document::Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        // Cursor on "helper.o" within the dep list item (outer-doc line 5,
        // block-local line 4 "    - helper.o", block-local character 8;
        // "; " (2) + 8 = 10).
        let loc = analyzer
            .goto_definition(
                &doc,
                Position {
                    line: 5,
                    character: 10
                }
            )
            .expect("goto-definition on a dep inside an embedded block");
        assert_eq!(
            loc.range.start.line, 1,
            "should jump to the rule declaring helper.o, shifted to outer-doc line 1: {loc:?}"
        );
    }

    /// Goto-definition on a `cmd:` argument referencing a real on-disk file,
    /// inside an embedded block, must resolve to that file directly - and,
    /// since the `Location` now points at a genuinely *different* file, its
    /// range must NOT be shifted (regression-proofing
    /// `location_out_of_block`'s uri-equality gate).
    #[test]
    fn goto_definition_on_a_cmd_argument_inside_an_embedded_block_opens_the_file_unshifted() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("main.asm"), "").unwrap();
        let host_uri =
            tower_lsp::lsp_types::Url::from_file_path(tmp.path().join("host.asm")).unwrap();
        let text = "; #!bndbuild\n; - tgt: out.bin\n;   cmd: basm main.asm -o out.bin\n";
        let doc = crate::common::document::Document::new(host_uri.clone(), text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        // Cursor on "main.asm" within the cmd argument list (outer-doc line
        // 2, block-local line 1 "  cmd: basm main.asm -o out.bin",
        // block-local character 12; "; " (2) + 12 = 14).
        let loc = analyzer
            .goto_definition(
                &doc,
                Position {
                    line: 2,
                    character: 14
                }
            )
            .expect("goto-definition on a cmd argument inside an embedded block");
        assert_eq!(
            loc.uri,
            tower_lsp::lsp_types::Url::from_file_path(tmp.path().join("main.asm")).unwrap()
        );
        assert_ne!(loc.uri, host_uri);
        assert_eq!(
            loc.range.start.line, 0,
            "a different-file location must not be shifted: {loc:?}"
        );
    }
}

#[cfg(test)]
mod rename_tests {
    use super::*;

    fn doc(uri: &str, text: &str) -> crate::common::document::Document {
        let uri = tower_lsp::lsp_types::Url::parse(uri).unwrap();
        crate::common::document::Document::new(uri, text.to_string(), 1)
    }

    /// Regression test: renaming a `RANGE`/`DEFSECTION` section name must
    /// also update every `SECTION name` usage and every quoted reference in
    /// `section_start("name")`/`section_length("name")` calls — and, since
    /// real basm code very commonly uses lowercase directives/names (as
    /// this fixture, mirrored from `good_section.asm`, does), it must work
    /// regardless of case: `find_label_word_and_prefix_matches` used to
    /// compare an un-uppercased search word against an uppercased line,
    /// silently matching nothing whenever the name itself wasn't already
    /// all-caps.
    #[test]
    fn section_name_rename_updates_every_occurrence_including_lowercase() {
        let text = "range $0080, $3FFF, code\nrange $4000, $7FFF, data\n\nsection code\n  ld hl, message_1\n  call print_message\n\nsection data\nmessage_1: db \"This is message #1.\", $00\n\nsection code\nprint_message:\n\tld a, (hl)\n\tret\n\n\tassert section_start(\"data\") ==  0x4000\n\tassert section_length(\"data\") == 0x4000\n";
        let d = doc("file:///sect.asm", text);
        let analyzer = AssemblyAnalyzer::new();

        // Cursor on "data" in "range $4000, $7FFF, data" (line 1).
        let col = text.lines().nth(1).unwrap().find("data").unwrap() as u32;
        let edit = analyzer
            .rename(
                &d,
                Position {
                    line: 1,
                    character: col
                },
                "info"
            )
            .expect("expected a workspace edit");
        let edits = edit.changes.unwrap().remove(&d.uri).unwrap();
        // range definition, `section data`, and two quoted references.
        assert_eq!(edits.len(), 4, "{edits:?}");
        for e in &edits {
            assert_eq!(e.new_text, "info");
        }
    }

    /// Regression test: a `FUNCTION`'s own parameter (`x` in
    /// `FUNCTION double(x)`) must rename within the function's body, not
    /// trigger a workspace-wide rename of every unrelated `x`.
    #[test]
    fn function_parameter_rename_is_scoped_to_its_own_body() {
        let text = "FUNCTION double(x)\n  RETURN x*2\nENDFUNCTION\n\nDB double(5)\n";
        let d = doc("file:///fn.asm", text);
        let analyzer = AssemblyAnalyzer::new();
        // Cursor on "x" in "RETURN x*2" (line 1).
        let col = text.lines().nth(1).unwrap().find('x').unwrap() as u32;

        let target = analyzer
            .resolve_rename_target(
                &d,
                Position {
                    line: 1,
                    character: col
                }
            )
            .expect("target");
        assert!(
            matches!(&target, RenameTarget::FunctionLocal { function_name, name, .. }
                if function_name == "double" && name == "x"),
            "{target:?}"
        );

        let edit = analyzer
            .rename(
                &d,
                Position {
                    line: 1,
                    character: col
                },
                "y"
            )
            .expect("expected a workspace edit");
        let edits = edit.changes.unwrap().remove(&d.uri).unwrap();
        // The parameter list ("(x)") and the RETURN expression's "x".
        assert_eq!(edits.len(), 2, "{edits:?}");
        assert_eq!(edits[0].range.start.line, 0);
        assert_eq!(edits[1].range.start.line, 1);
        for e in &edits {
            assert_eq!(e.new_text, "y");
        }
    }

    /// Regression test: a symbol `=`/`EQU`-defined *inside* a `FUNCTION`
    /// body is local to that function (basm functions can't contain a
    /// genuine label definition, only `Equ`/`Let` — see
    /// `ParsingState::FunctionLimited`) — rename must stay confined to the
    /// function and must not touch a same-named definition/reference
    /// outside it, even elsewhere in the same file.
    #[test]
    fn function_local_symbol_rename_does_not_touch_an_unrelated_outer_definition() {
        let text = "FUNCTION compute(x)\n  y = x * 2\n  RETURN y\nENDFUNCTION\n\nDB compute(5)\n\ny EQU 99\n";
        let d = doc("file:///fnlocal.asm", text);
        let analyzer = AssemblyAnalyzer::new();
        // Cursor on "y" in "RETURN y" (line 2).
        let col = text.lines().nth(2).unwrap().find('y').unwrap() as u32;

        let target = analyzer
            .resolve_rename_target(
                &d,
                Position {
                    line: 2,
                    character: col
                }
            )
            .expect("target");
        assert!(
            matches!(&target, RenameTarget::FunctionLocal { function_name, .. } if function_name == "compute"),
            "{target:?}"
        );

        let edit = analyzer
            .rename(
                &d,
                Position {
                    line: 2,
                    character: col
                },
                "z"
            )
            .expect("expected a workspace edit");
        let edits = edit.changes.unwrap().remove(&d.uri).unwrap();
        // Only the assignment (line 1) and the RETURN usage (line 2) —
        // never the unrelated `y EQU 99` outside the function (line 7).
        assert_eq!(edits.len(), 2, "{edits:?}");
        assert_eq!(edits[0].range.start.line, 1);
        assert_eq!(edits[1].range.start.line, 2);
        for e in &edits {
            assert_eq!(e.new_text, "z");
        }
    }

    /// Regression test, the inverse of
    /// `function_local_symbol_rename_does_not_touch_an_unrelated_outer_definition`:
    /// renaming an *outer* global symbol must not reach inside a `FUNCTION`
    /// that shadows the same name with its own local definition — inside
    /// that function, the name refers to the local, a different symbol.
    #[test]
    fn global_rename_does_not_reach_inside_a_function_that_shadows_it() {
        let text =
            "y EQU 99\n\nFUNCTION compute(x)\n  y = x * 2\n  RETURN y\nENDFUNCTION\n\nDB y\n";
        let d = doc("file:///shadow.asm", text);
        let analyzer = AssemblyAnalyzer::new();
        // Cursor on "y" at "y EQU 99" (line 0) — the outer definition.
        let target = analyzer
            .resolve_rename_target(
                &d,
                Position {
                    line: 0,
                    character: 0
                }
            )
            .expect("target");
        assert!(
            matches!(&target, RenameTarget::Global(s) if s == "y"),
            "{target:?}"
        );

        let edit = analyzer
            .rename(
                &d,
                Position {
                    line: 0,
                    character: 0
                },
                "z"
            )
            .expect("expected a workspace edit");
        let edits = edit.changes.unwrap().remove(&d.uri).unwrap();
        // Only the outer definition (line 0) and its outer usage (line 7) —
        // never the function-local `y = x * 2` / `RETURN y` (lines 3, 4).
        assert_eq!(edits.len(), 2, "{edits:?}");
        assert_eq!(edits[0].range.start.line, 0);
        assert_eq!(edits[1].range.start.line, 7);
        for e in &edits {
            assert_eq!(e.new_text, "z");
        }
    }

    #[test]
    /// Regression test: a `REPEAT`'s own counter variable (`i` in
    /// `REPEAT 3, i, 0`), used bare within its body, must rename within
    /// that loop only — mirroring `FunctionLocal` — and must not touch an
    /// unrelated same-named symbol outside the loop.
    #[test]
    fn repeat_counter_rename_is_scoped_to_its_own_loop() {
        // The body references the counter via `{i}` interpolation — basm's
        // real convention (and what real code, including the reported
        // case, actually writes) for referencing a loop counter from
        // within a body that can contain arbitrary Z80 code, where a bare
        // `i` could equally be something else entirely.
        let text = "REPEAT 3, i, 0\n  db {i}\nENDR\n\ni EQU 99\nDB i\n";
        let d = doc("file:///rep.asm", text);
        let analyzer = AssemblyAnalyzer::new();

        // Cursor on "i" inside "{i}" (line 1) — inside the loop.
        let col = text.lines().nth(1).unwrap().find("{i}").unwrap() as u32 + 1;
        let target = analyzer
            .resolve_rename_target(
                &d,
                Position {
                    line: 1,
                    character: col
                }
            )
            .expect("target");
        assert!(
            matches!(&target, RenameTarget::LoopLocal { keyword, name, .. }
                if keyword == "REPEAT" && name == "i"),
            "{target:?}"
        );
        let edit = analyzer
            .rename(
                &d,
                Position {
                    line: 1,
                    character: col
                },
                "idx"
            )
            .expect("expected a workspace edit");
        let edits = edit.changes.unwrap().remove(&d.uri).unwrap();
        // Only the counter declaration (line 0, bare) and the body's `{i}`
        // interpolation (line 1, re-wrapped in braces) — never the
        // unrelated outer `i EQU 99` / `DB i` (lines 4, 5).
        assert_eq!(edits.len(), 2, "{edits:?}");
        assert_eq!(edits[0].range.start.line, 0);
        assert_eq!(edits[0].new_text, "idx");
        assert_eq!(edits[1].range.start.line, 1);
        assert_eq!(edits[1].new_text, "{idx}");

        // The inverse: renaming the *outer*, unrelated "i" must not reach
        // inside the loop that shadows it with its own counter.
        let target2 = analyzer
            .resolve_rename_target(
                &d,
                Position {
                    line: 4,
                    character: 0
                }
            )
            .expect("target");
        assert!(
            matches!(&target2, RenameTarget::Global(s) if s == "i"),
            "{target2:?}"
        );
        let edit2 = analyzer
            .rename(
                &d,
                Position {
                    line: 4,
                    character: 0
                },
                "idx"
            )
            .expect("expected a workspace edit");
        let edits2 = edit2.changes.unwrap().remove(&d.uri).unwrap();
        assert_eq!(edits2.len(), 2, "{edits2:?}");
        assert_eq!(edits2[0].range.start.line, 4);
        assert_eq!(edits2[1].range.start.line, 5);
    }

    /// Regression test: a `MACRO`'s own declared parameter, referenced via
    /// `{param}` interpolation from within its body, must rename only that
    /// interpolation — never a genuine register/mnemonic occurrence that
    /// happens to share the same letter (`ld a, {a}`: the bare `a` is the
    /// real accumulator register, only `{a}` is the parameter). This is the
    /// reported real-world case, reproduced verbatim in spirit.
    #[test]
    fn macro_param_rename_does_not_touch_a_same_named_register() {
        let text = "MACRO foo(a)\n  ld a, {a}\nENDM\n\nfoo(5)\n";
        let d = doc("file:///macro_param.asm", text);
        let analyzer = AssemblyAnalyzer::new();
        // Cursor on "a" inside "{a}" (line 1).
        let col = text.lines().nth(1).unwrap().find("{a}").unwrap() as u32 + 1;

        let target = analyzer
            .resolve_rename_target(
                &d,
                Position {
                    line: 1,
                    character: col
                }
            )
            .expect("target");
        assert!(
            matches!(&target, RenameTarget::MacroLocal { macro_name, name, .. }
                if macro_name == "foo" && name == "a"),
            "{target:?}"
        );

        let range = analyzer
            .prepare_rename(
                &d,
                Position {
                    line: 1,
                    character: col
                }
            )
            .expect("prepare_rename must offer to rename the macro parameter");
        assert_eq!(range.start.character, col);

        let edit = analyzer
            .rename(
                &d,
                Position {
                    line: 1,
                    character: col
                },
                "val"
            )
            .expect("expected a workspace edit");
        let edits = edit.changes.unwrap().remove(&d.uri).unwrap();
        // The declaration "(a)" (bare) and the body's "{a}" (re-wrapped) —
        // never the bare "a" register operand in "ld a,".
        assert_eq!(edits.len(), 2, "{edits:?}");
        assert_eq!(edits[0].range.start.line, 0);
        assert_eq!(edits[0].new_text, "val");
        assert_eq!(edits[1].range.start.line, 1);
        assert_eq!(edits[1].range.start.character, 8);
        assert_eq!(edits[1].new_text, "{val}");
    }

    /// Regression test: an `ITERATE`'s own counter variable, used bare
    /// within its body, must rename within that loop only.
    #[test]
    fn iterate_counter_rename_is_scoped_to_its_own_loop() {
        // The body references the counter via `{v}` interpolation, basm's
        // real convention for referencing it from within arbitrary code.
        let text = "ITERATE v, [1,2,3]\n  db {v}\nENDITERATE\n";
        let d = doc("file:///iter.asm", text);
        let analyzer = AssemblyAnalyzer::new();
        let col = text.lines().nth(1).unwrap().find("{v}").unwrap() as u32 + 1;

        let target = analyzer
            .resolve_rename_target(
                &d,
                Position {
                    line: 1,
                    character: col
                }
            )
            .expect("target");
        assert!(
            matches!(&target, RenameTarget::LoopLocal { keyword, name, .. }
                if keyword == "ITERATE" && name == "v"),
            "{target:?}"
        );

        let edit = analyzer
            .rename(
                &d,
                Position {
                    line: 1,
                    character: col
                },
                "val"
            )
            .expect("expected a workspace edit");
        let edits = edit.changes.unwrap().remove(&d.uri).unwrap();
        assert_eq!(edits.len(), 2, "{edits:?}");
        assert_eq!(edits[0].range.start.line, 0);
        assert_eq!(edits[0].new_text, "val");
        assert_eq!(edits[1].range.start.line, 1);
        assert_eq!(edits[1].new_text, "{val}");
    }

    /// Regression test: `prepare_rename` used to reject any word matching a
    /// register/mnemonic/directive keyword *before* checking whether it was
    /// actually a scoped local — a `REPEAT` counter named `i` (an extremely
    /// common convention, and the reported real-world case) collides with
    /// the Z80 `I` register, so VS Code's rename UI never even opened.
    /// Scoped locals (loop counters, function parameters/locals, `.label`s)
    /// must be renamable regardless of such a collision — only a bare
    /// `Global` fallback should still reject genuine register/mnemonic use.
    #[test]
    fn prepare_rename_allows_a_repeat_counter_named_like_a_register() {
        let text = "repeat 4, i, 0\n    inks = list_set(inks, {i}+1<<3+1<<2, list_get(writter, {i}))\n    inks = list_set(inks, {i}+1<<3, list_get(writter, {i}))\nendrepeat\n";
        let d = doc("file:///vscode_repro.asm", text);
        let analyzer = AssemblyAnalyzer::new();
        // Cursor on "i" inside "{i}" on line 1.
        let col = text.lines().nth(1).unwrap().find("{i}").unwrap() as u32 + 1;

        let target = analyzer
            .resolve_rename_target(
                &d,
                Position {
                    line: 1,
                    character: col
                }
            )
            .expect("target");
        assert!(
            matches!(&target, RenameTarget::LoopLocal { keyword, name, .. }
                if keyword == "REPEAT" && name == "i"),
            "{target:?}"
        );

        let range = analyzer
            .prepare_rename(
                &d,
                Position {
                    line: 1,
                    character: col
                }
            )
            .expect("prepare_rename must offer to rename the loop counter");
        assert_eq!(range.start.character, col);
        assert_eq!(range.end.character, col + 1);

        // A genuine register use (not scoped to any loop/function) must
        // still be rejected.
        let text2 = "    ld i, a\n";
        let d2 = doc("file:///reg.asm", text2);
        assert!(
            analyzer
                .prepare_rename(
                    &d2,
                    Position {
                        line: 0,
                        character: 7
                    }
                )
                .is_none()
        );
    }

    #[test]
    fn global_label_rename_updates_definition_and_references() {
        let text = "OLD_LABEL:\n    ret\n    call OLD_LABEL\n";
        let d = doc("file:///g.asm", text);
        let analyzer = AssemblyAnalyzer::new();
        // Cursor on "OLD_LABEL" at its own definition (line 0).
        let target = analyzer
            .resolve_rename_target(
                &d,
                Position {
                    line: 0,
                    character: 2
                }
            )
            .expect("target");
        assert!(
            matches!(target, RenameTarget::Global(ref s) if s == "OLD_LABEL"),
            "{target:?}"
        );

        let edits = analyzer.rename_occurrences_in(&d, &target, "NEW_LABEL");
        assert_eq!(edits.len(), 2, "{edits:?}");
        for e in &edits {
            assert_eq!(e.new_text, "NEW_LABEL");
        }
    }

    /// Regression test for the `pattern_starts_on_line`/
    /// `find_label_word_and_prefix_matches` dedup: a longer identifier that
    /// merely starts with the rename target (`OLD_LABELFOO`, not followed
    /// by `.`) must not be touched — only the `.`-qualified-suffix case
    /// gets the special "accept even though followed by an ident byte"
    /// treatment, everything else still needs a real word boundary.
    #[test]
    fn global_label_rename_does_not_touch_a_longer_identifier_with_the_same_prefix() {
        let text = "OLD_LABEL:\n    ret\n    call OLD_LABELFOO\n";
        let d = doc("file:///g3.asm", text);
        let analyzer = AssemblyAnalyzer::new();
        let target = analyzer
            .resolve_rename_target(
                &d,
                Position {
                    line: 0,
                    character: 2
                }
            )
            .expect("target");

        let edits = analyzer.rename_occurrences_in(&d, &target, "NEW_LABEL");
        // Only the definition itself - "OLD_LABELFOO" must not be touched.
        assert_eq!(edits.len(), 1, "{edits:?}");
        assert_eq!(edits[0].range.start.line, 0);
    }

    #[test]
    fn global_label_rename_also_rewrites_qualified_local_references() {
        let text = "OLD_LABEL:\n.local\n    ret\n    jr OLD_LABEL.local\n";
        let d = doc("file:///g2.asm", text);
        let analyzer = AssemblyAnalyzer::new();
        let target = analyzer
            .resolve_rename_target(
                &d,
                Position {
                    line: 0,
                    character: 2
                }
            )
            .expect("target");

        let edits = analyzer.rename_occurrences_in(&d, &target, "NEW_LABEL");
        assert_eq!(edits.len(), 2, "{edits:?}");
        assert!(edits.iter().any(|e| e.new_text == "NEW_LABEL"), "{edits:?}");
        let qualified = edits
            .iter()
            .find(|e| e.new_text == "NEW_LABEL.local")
            .expect("qualified rewrite");
        assert_eq!(qualified.range.start.line, 3);
    }

    #[test]
    fn local_label_rename_is_scoped_to_its_own_global() {
        let text = "GLOBAL_A:\n.foo\n    ret\nGLOBAL_B:\n.foo\n    ret\n";
        let d = doc("file:///l.asm", text);
        let analyzer = AssemblyAnalyzer::new();
        // Cursor on ".foo" under GLOBAL_A (line 1).
        let target = analyzer
            .resolve_rename_target(
                &d,
                Position {
                    line: 1,
                    character: 1
                }
            )
            .expect("target");
        assert!(matches!(&target, RenameTarget::Local { owner, .. } if owner == "GLOBAL_A"));

        let edits = analyzer.rename_occurrences_in(&d, &target, "bar");
        assert_eq!(edits.len(), 1, "{edits:?}");
        assert_eq!(edits[0].range.start.line, 1);
        assert_eq!(edits[0].new_text, ".bar");
    }

    /// Regression test: `prepare_rename`'s range for a local label covers
    /// the leading `.` (it's part of the word), so an LSP client's
    /// pre-filled rename input typically includes it too — `new_name`
    /// arriving as `.bar` (not just `bar`) must not produce `..bar`.
    #[test]
    fn local_label_rename_does_not_duplicate_the_leading_dot() {
        let text = "GLOBAL_A:\n.foo\n    ret\n";
        let d = doc("file:///l2.asm", text);
        let analyzer = AssemblyAnalyzer::new();
        let target = analyzer
            .resolve_rename_target(
                &d,
                Position {
                    line: 1,
                    character: 1
                }
            )
            .expect("target");

        let edits = analyzer.rename_occurrences_in(&d, &target, ".bar");
        assert_eq!(edits.len(), 1, "{edits:?}");
        assert_eq!(edits[0].new_text, ".bar", "{edits:?}");
    }

    #[test]
    fn prepare_rename_rejects_a_mnemonic() {
        let text = "    ld a,1\n";
        let d = doc("file:///m.asm", text);
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .prepare_rename(
                    &d,
                    Position {
                        line: 0,
                        character: 5
                    }
                )
                .is_none()
        );
    }

    /// Simulates the workspace-wide expansion `backend.rs` performs: the
    /// same `RenameTarget` (resolved from one document) is applied to a
    /// completely different document's own text.
    #[test]
    fn a_rename_target_can_be_applied_to_a_different_documents_text() {
        let d1 = doc("file:///main.asm", "GLOBAL_X:\n    ret\n");
        let analyzer = AssemblyAnalyzer::new();
        let target = analyzer
            .resolve_rename_target(
                &d1,
                Position {
                    line: 0,
                    character: 2
                }
            )
            .unwrap();

        let d2 = doc(
            "file:///other.asm",
            "    call GLOBAL_X\n    jr GLOBAL_X.sub\n"
        );
        let edits = analyzer.rename_occurrences_in(&d2, &target, "GLOBAL_Y");
        assert_eq!(edits.len(), 2, "{edits:?}");
        assert!(edits.iter().any(|e| e.new_text == "GLOBAL_Y"), "{edits:?}");
        assert!(
            edits.iter().any(|e| e.new_text == "GLOBAL_Y.sub"),
            "{edits:?}"
        );
    }

    /// A BASIC variable rename inside a `LOCOMOTIVE` block embedded in a
    /// `.asm` file must produce absolute document coordinates, not ones
    /// relative to the extracted block.
    #[test]
    fn rename_delegates_to_basic_inside_a_locomotive_block() {
        let text = "LOCOMOTIVE\n10 LET X=5\n20 PRINT X\nENDLOCOMOTIVE\n";
        let d = doc("file:///embedded.asm", text);
        let analyzer = AssemblyAnalyzer::new();
        // Cursor on the "X" in "10 LET X=5" (document line 1, column 7).
        let edit = analyzer
            .rename(
                &d,
                Position {
                    line: 1,
                    character: 7
                },
                "Y"
            )
            .expect("expected a workspace edit");
        let edits = edit.changes.unwrap().remove(&d.uri).unwrap();
        assert_eq!(edits.len(), 2, "{edits:?}");
        assert_eq!(edits[0].range.start.line, 1);
        assert_eq!(edits[1].range.start.line, 2);
    }

    /// A Jinja variable rename inside a `#!bndbuild` embedded block must
    /// produce absolute document coordinates for every occurrence, not ones
    /// relative to the extracted block.
    #[test]
    fn rename_delegates_to_bndbuild_inside_an_embedded_block() {
        let text = "; #!bndbuild\n\
                     ; {% set NAME = \"test\" %}\n\
                     ; - tgt: {{ NAME }}\n\
                     ;   cmd: echo hi\n";
        let d = doc("file:///embedded.asm", text);
        let analyzer = AssemblyAnalyzer::new();
        // Cursor on "NAME" in "{% set NAME = ... %}" (outer-doc line 1,
        // block-local line 0 "{% set NAME = \"test\" %}", block-local
        // character 9 lands inside "NAME"; "; " (2) + 9 = 11).
        let edit = analyzer
            .rename(
                &d,
                Position {
                    line: 1,
                    character: 11
                },
                "OTHER"
            )
            .expect("expected a workspace edit");
        let edits = edit.changes.unwrap().remove(&d.uri).unwrap();
        // Both the `{% set NAME %}` definition (outer line 1) and the
        // `{{ NAME }}` reference (outer line 2) must be renamed.
        assert_eq!(edits.len(), 2, "{edits:?}");
        assert!(edits.iter().any(|e| e.range.start.line == 1), "{edits:?}");
        assert!(edits.iter().any(|e| e.range.start.line == 2), "{edits:?}");
    }

    /// Regression test for the `backend.rs`-direct-caller gotcha:
    /// `resolve_rename_target` is called directly by the top-level `rename`
    /// handler *before* it ever reaches `AssemblyAnalyzer::rename` (where
    /// the embedded-block delegation lives) - without its own guard,
    /// ordinary-identifier-shaped text inside a `#!bndbuild` block (e.g.
    /// "tgt") would be misidentified as a workspace-wide basm label rename
    /// target instead of correctly deferring to the block-scoped rename.
    #[test]
    fn resolve_rename_target_returns_none_inside_an_embedded_bndbuild_block() {
        let text = "; #!bndbuild\n; - tgt: test\n;   cmd: echo hi\n";
        let d = doc("file:///embedded.asm", text);
        let analyzer = AssemblyAnalyzer::new();
        // Cursor on "tgt" (outer-doc line 1, character 4).
        let target = analyzer.resolve_rename_target(
            &d,
            Position {
                line: 1,
                character: 4
            }
        );
        assert!(target.is_none(), "{target:?}");
    }
}

#[cfg(test)]
mod word_scanner_tests {
    use super::*;

    #[test]
    fn pattern_starts_on_line_only_checks_the_leading_boundary() {
        // "OLD" followed by "." (an ident byte) is still reported here -
        // rejecting on the trailing side is left to the caller.
        let hits = pattern_starts_on_line("call OLD.local", "OLD");
        assert_eq!(hits, vec![(5, 8)]);
    }

    #[test]
    fn pattern_starts_on_line_rejects_a_match_preceded_by_an_ident_byte() {
        // "OLD" inside "FOO_OLD" is not preceded by a word boundary.
        let hits = pattern_starts_on_line("call FOO_OLD", "OLD");
        assert!(hits.is_empty(), "{hits:?}");
    }

    #[test]
    fn word_matches_on_line_rejects_a_match_followed_by_an_ident_byte() {
        // "OLD" inside "OLDFOO" fails the trailing boundary check.
        let hits = word_matches_on_line("call OLDFOO", "OLD");
        assert!(hits.is_empty(), "{hits:?}");
    }

    #[test]
    fn word_matches_on_line_accepts_a_standalone_word() {
        let hits = word_matches_on_line("call OLD", "OLD");
        assert_eq!(hits, vec![(5, 8)]);
    }
}
