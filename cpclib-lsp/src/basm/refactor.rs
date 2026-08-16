//! Refactoring code-actions for assembly files: wrap in REPEAT/loop,
//! join statements onto one line, split multi-statement lines.

use cpclib_asm::unused_bindings::UnusedBindingKind;
use cpclib_tokens::{DataAccess, DataAccessElem, ExprElement, ListingElement, Mnemonic};
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use super::command::{
    remove_unused_parameter_command, select_range_command, single_file_edit, single_file_multi_edit
};
use super::format::{split_at_colon, strip_asm_comment};
use super::remove_parameter::RemoveParameterKind;
use crate::common::document::{Document, byte_offset_to_utf16_col};

impl AssemblyAnalyzer {
    /// Offer to remove an unused REPEAT loop counter (`REPEAT 5, i` →
    /// `REPEAT 5`) when `range` (the requested code-action position - a
    /// plain cursor, i.e. `range.start == range.end`, in the overwhelmingly
    /// common case) falls on the declaring REPEAT header's own line.
    ///
    /// Re-derives the condition fresh from `(document, range)` at request
    /// time, the same way every other action in this file/`command.rs`
    /// already does - this codebase has no diagnostic↔code-action pairing
    /// mechanism (`CodeActionParams.context.diagnostics` is never read
    /// anywhere), so this doesn't invent one just for this quickfix.
    ///
    /// Only `RepeatCounter` bindings with a `removable_clause` are ever
    /// offered here: ITERATE/FOR counters are mandatory (nothing to
    /// remove), a REPEAT counter with an explicit start/step value is
    /// deliberately left warning-only too (see `removable_clause`'s own doc
    /// comment in `cpclib-asm`). MACRO/FUNCTION parameter removal is a
    /// separate action, `unused_macro_or_function_parameter_removal_action`
    /// below - it needs a call-site-aware, cross-file rewrite, a
    /// structurally different (async, command-based) shape than this
    /// synchronous single-file edit.
    pub(super) fn unused_repeat_counter_removal_action(
        &self,
        document: &Document,
        range: Range
    ) -> Option<CodeAction> {
        let listing = self.parse_document(document).ok()?;
        let (lo, hi) = (range.start.line, range.end.line);

        for binding in cpclib_asm::unused_bindings::find_unused_bindings(listing.iter()) {
            if binding.kind != UnusedBindingKind::RepeatCounter {
                continue;
            }
            let def_line = (binding.line as u32).saturating_sub(1);
            if def_line < lo || def_line > hi {
                continue;
            }
            let Some((clause_line, clause_column, clause_len)) = binding.removable_clause
            else {
                continue;
            };
            let line0 = (clause_line as u32).saturating_sub(1);
            let line_text = document.line(line0 as usize).unwrap_or_default();
            let byte_col = clause_column.saturating_sub(1);
            let start_char = byte_offset_to_utf16_col(&line_text, byte_col) as u32;
            let end_char = byte_offset_to_utf16_col(&line_text, byte_col + clause_len) as u32;
            let edit_range = Range {
                start: Position {
                    line: line0,
                    character: start_char
                },
                end: Position {
                    line: line0,
                    character: end_char
                }
            };
            return Some(CodeAction {
                title: format!("Remove unused loop counter '{}'", binding.name),
                kind: Some(CodeActionKind::QUICKFIX),
                edit: Some(single_file_edit(
                    document.uri.clone(),
                    edit_range,
                    String::new()
                )),
                ..Default::default()
            });
        }
        None
    }

    /// Offer to strip a redundant, explicit `A,` accumulator prefix
    /// (`CP A,r` → `CP r`, and likewise for `ADD`/`ADC`/`SBC`/`SUB`/`AND`/
    /// `OR`/`XOR`) when the cursor/selection sits on the instruction's own
    /// line. Same cheap, synchronous, same-file, re-derive-fresh-from-
    /// `(document, range)` shape as `unused_repeat_counter_removal_action`
    /// above - there's no cross-file impact here at all, since the prefix
    /// is purely local to the one instruction that carries it.
    pub(super) fn redundant_accumulator_prefix_removal_action(
        &self,
        document: &Document,
        range: Range
    ) -> Option<CodeAction> {
        let listing = self.parse_document(document).ok()?;
        let cursor_line = range.start.line;

        let token = super::token::flatten_listing(listing.iter()).find(|t| {
            super::token::span_line(*t) == cursor_line && t.is_redundant_accumulator_prefix()
        })?;

        // Delete from the start of the redundant `A` operand through to the
        // start of the real operand - this removes "A" + the comma + the
        // whitespace between them in one go, leaving the whitespace that
        // was already between the mnemonic and `A` in place (`CP A, C` →
        // `CP C`, not `CPC` or `CP  C`).
        let arg1_span = token.mnemonic_arg1()?.span();
        let arg2_span = token.mnemonic_arg2()?.span();
        let (arg1_line_1based, arg1_col_1based) = arg1_span.relative_line_and_column();
        let (arg2_line_1based, arg2_col_1based) = arg2_span.relative_line_and_column();
        if arg1_line_1based != arg2_line_1based {
            // Not a realistic shape for this instruction family, but guard
            // against it defensively rather than build a cross-line edit.
            return None;
        }
        let line0 = arg1_line_1based.saturating_sub(1) as u32;
        let line_text = document.line(line0 as usize).unwrap_or_default();
        let start_char =
            byte_offset_to_utf16_col(&line_text, arg1_col_1based.saturating_sub(1)) as u32;
        let end_char =
            byte_offset_to_utf16_col(&line_text, arg2_col_1based.saturating_sub(1)) as u32;
        let edit_range = Range {
            start: Position {
                line: line0,
                character: start_char
            },
            end: Position {
                line: line0,
                character: end_char
            }
        };

        Some(CodeAction {
            title: "Remove redundant explicit 'A,' accumulator prefix".to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            edit: Some(single_file_edit(
                document.uri.clone(),
                edit_range,
                String::new()
            )),
            ..Default::default()
        })
    }

    /// Offer to delete or simplify a no-op / trivially-improvable
    /// instruction when the cursor/selection sits on its line - same cheap,
    /// synchronous, same-file, re-derive-fresh-from-`(document, range)`
    /// shape as `redundant_accumulator_prefix_removal_action` above. Every
    /// rule here is a pure syntactic pattern match on the instruction's own
    /// operands (no register-*value* tracking needed - see
    /// `classify_no_op_or_improvable`'s own doc comment).
    pub(super) fn no_op_or_improvable_instruction_action(
        &self,
        document: &Document,
        range: Range
    ) -> Option<CodeAction> {
        let listing = self.parse_document(document).ok()?;
        let cursor_line = range.start.line;

        let token = super::token::flatten_listing(listing.iter())
            .find(|t| super::token::span_line(*t) == cursor_line)?;
        let (title, replacement) = classify_no_op_or_improvable(token)?;

        let edit_range = super::token::token_lsp_range(token);
        let original = token.to_string();
        let new_text = match_case_like(&original, replacement);

        Some(CodeAction {
            title: title.to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            edit: Some(single_file_edit(document.uri.clone(), edit_range, new_text)),
            ..Default::default()
        })
    }

    /// Offer to make the self-modifying-code idiom name the right byte.
    ///
    /// Two shapes, one intent. With no `equ` at all the missing half is
    /// appended, leaving whatever spacing and comment the line already had.
    /// With an `equ $-N` that misses the operand, only the `N` is rewritten -
    /// appending a second `equ` would not even assemble.
    pub(super) fn smc_label_equ_action(
        &self,
        document: &Document,
        range: Range
    ) -> Option<CodeAction> {
        if !self.config().warnings.smc_label_without_equ {
            return None;
        }
        let listing = self.parse_document(document).ok()?;
        let cursor_line = range.start.line;

        let found = super::lint_smc_label::find_suspicious_smc_labels(&listing)
            .into_iter()
            .find(|f| f.line == cursor_line)?;

        let line_text = document.line(cursor_line as usize)?;
        let label_end = line_text.find(&found.name)? + found.name.len();
        let suggestion = found.suggestion();

        let (edit_range, new_text) = match found.problem {
            super::lint_smc_label::SmcLabelProblem::Missing => {
                let position = Position {
                    line: cursor_line,
                    character: crate::common::document::byte_offset_to_utf16_col(
                        &line_text, label_end
                    ) as u32
                };
                (
                    Range {
                        start: position,
                        end: position
                    },
                    format!(" {suggestion}")
                )
            },
            super::lint_smc_label::SmcLabelProblem::WrongOffset(_) => {
                // Rewrite just the number, so `$ - 1` keeps its spacing and a
                // trailing comment stays put.
                let (start, end) = offset_literal_span(&line_text, label_end)?;
                (
                    Range {
                        start: Position {
                            line: cursor_line,
                            character: crate::common::document::byte_offset_to_utf16_col(
                                &line_text, start
                            ) as u32
                        },
                        end: Position {
                            line: cursor_line,
                            character: crate::common::document::byte_offset_to_utf16_col(
                                &line_text, end
                            ) as u32
                        }
                    },
                    found.offset.to_string()
                )
            }
        };

        Some(CodeAction {
            title: format!("Name the operand byte: '{} {suggestion}'", found.name),
            kind: Some(CodeActionKind::QUICKFIX),
            edit: Some(single_file_edit(document.uri.clone(), edit_range, new_text)),
            ..Default::default()
        })
    }

    /// Offer to replace a "fake instruction" (accepted by this parser as a
    /// convenience, assembled using several real opcodes, e.g. `ld hl, de`)
    /// with the real instruction(s) it expands to, joined on one line with
    /// `:` (basm's real statement separator).
    ///
    /// Deliberately does **not** reuse hover's `disassemble_snippet_lines`
    /// (`disassemble.rs`) for this: that path actually assembles the
    /// snippet then disassembles the resulting bytes back into text, which
    /// round-trips any symbolic operand through its *resolved numeric
    /// value* - fine for a hover preview, but wrong for a quickfix, since it
    /// would silently hardcode a value that goes stale the moment the
    /// symbol's address changes. Instead, this calls
    /// `ListingElement::fake_to_listing_from_access` directly - the same
    /// function the real assembler itself calls to expand a fake
    /// instruction - which builds its result from the token's own
    /// `DataAccess` operands, never from assembled bytes, so any symbolic
    /// operand (should a future fake instruction ever carry one) is
    /// preserved verbatim rather than resolved away.
    pub(super) fn fake_instruction_to_real_action(
        &self,
        document: &Document,
        range: Range
    ) -> Option<CodeAction> {
        let listing = self.parse_document(document).ok()?;
        let cursor_line = range.start.line;

        let token = super::token::flatten_listing(listing.iter())
            .find(|t| super::token::span_line(*t) == cursor_line && t.is_fake_instruction())?;

        let expansion = fake_instruction_real_expansion(token)?;
        if expansion.is_empty() {
            // Not actually representable this way - decline rather than
            // offer a wrong/empty replacement.
            return None;
        }

        let upper = is_uppercase_style(&token.to_string());
        let new_text = expansion
            .iter()
            .map(|(m, a1, a2)| render_expanded_instruction(*m, a1, a2, upper))
            .collect::<Vec<_>>()
            .join(":");
        let edit_range = super::token::token_lsp_range(token);

        Some(CodeAction {
            title: "Replace fake instruction with its real instruction(s)".to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            edit: Some(single_file_edit(document.uri.clone(), edit_range, new_text)),
            ..Default::default()
        })
    }

    /// Offer to replace a numeric literal under the cursor with its
    /// firmware symbol name (e.g. `0xbb5a` → `TXT_OUTPUT`) when it resolves
    /// to a documented firmware routine/constant, reusing the exact same
    /// detection hover already uses (`extract_number_at_position` +
    /// `firmware_docs::lookup_by_value`, `hover.rs`/`firmware_docs.rs`) -
    /// both of those already carry everything needed (the symbol name *and*
    /// which `inner://firmware/...` file documents it), no new lookup data.
    /// Also prepends `INCLUDE ONCE "<that file>"` at the very top of the
    /// document, unless a matching `INCLUDE` (of any form) is already
    /// present - otherwise the new symbol wouldn't actually resolve.
    #[allow(clippy::type_complexity)]
    /// The first number on `line` that names a documented firmware routine.
    ///
    /// Scans left to right, trying each position a number could start at, and
    /// takes the first that resolves. A line has one such address in practice -
    /// `call 0xBB5A` - and where it has more, the caret decides (the caller
    /// tries that first).
    fn firmware_number_on_line(
        line: &str
    ) -> Option<(
        String,
        i64,
        usize,
        &'static crate::common::firmware_docs::FirmwareDoc
    )> {
        let bytes = line.as_bytes();
        let mut at = 0usize;
        while at < bytes.len() {
            if let Some((text, value, start)) = super::hover::extract_number_at_position(line, at)
                && let Some(fw) = crate::common::firmware_docs::lookup_by_value(value)
            {
                return Some((text, value, start, fw));
            }
            at += 1;
        }
        None
    }

    pub(super) fn firmware_symbol_replacement_action(
        &self,
        document: &Document,
        range: Range
    ) -> Option<CodeAction> {
        let line_text = document.line(range.start.line as usize)?;
        let col = range.start.character as usize;

        // The caret is rarely *on* the number. A hover is driven by the mouse,
        // which is why hovering `0xBB5A` names `TXT_OUTPUT` while the lightbulb
        // stayed empty: the editor asks for actions wherever the caret happens
        // to be, and that is usually the end of the line or the mnemonic.
        //
        // So the cursor is tried first - it disambiguates a line carrying
        // several numbers - and the line is then scanned for one that names a
        // firmware routine. Anything that resolves to nothing is skipped rather
        // than offered.
        let (num_str, value, byte_start, fw) =
            super::hover::extract_number_at_position(&line_text, col)
                .and_then(|(text, value, start)| {
                    crate::common::firmware_docs::lookup_by_value(value)
                        .map(|fw| (text, value, start, fw))
                })
                .or_else(|| {
                    // The line-wide fallback follows the same rule as the
                    // warning it hangs off: a firmware address is a routine
                    // only where control is transferred to it.
                    crate::basm::diagnostics::targets_a_routine(&line_text)
                        .then(|| Self::firmware_number_on_line(&line_text))
                        .flatten()
                })?;
        let _ = value;

        let start_char = byte_offset_to_utf16_col(&line_text, byte_start) as u32;
        let end_char = byte_offset_to_utf16_col(&line_text, byte_start + num_str.len()) as u32;
        let literal_range = Range {
            start: Position {
                line: range.start.line,
                character: start_char
            },
            end: Position {
                line: range.start.line,
                character: end_char
            }
        };

        let mut edits = vec![TextEdit {
            range: literal_range,
            new_text: fw.symbol.clone()
        }];

        let already_included = self.parse_document(document).ok().is_some_and(|listing| {
            super::token::flatten_listing(listing.iter()).any(|t| {
                t.is_include()
                    && t.include_fname().is_string()
                    && t.include_fname().string() == fw.source_file
            })
        });
        if !already_included {
            edits.push(TextEdit {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0
                    },
                    end: Position {
                        line: 0,
                        character: 0
                    }
                },
                new_text: format!("include once \"{}\"\n", fw.source_file)
            });
        }

        Some(CodeAction {
            title: format!("Replace with firmware symbol '{}'", fw.symbol),
            kind: Some(CodeActionKind::QUICKFIX),
            edit: Some(single_file_multi_edit(document.uri.clone(), edits)),
            ..Default::default()
        })
    }

    /// Offer to remove an unused MACRO/FUNCTION parameter when the
    /// cursor/selection sits on its declaring definition's header line -
    /// same cheap, synchronous, same-file gating as
    /// `unused_repeat_counter_removal_action` above, reusing the
    /// already-detected `UnusedBinding.owner` data (which macro/function
    /// owns the parameter, and its 0-based position) - no cross-file work
    /// happens here.
    ///
    /// Unlike every other `CodeAction` in this file, this one carries a
    /// `command` and **no** synchronous `edit`: finding and rewriting every
    /// call site across the workspace is real I/O (parsing every candidate
    /// file), far too expensive to do on every `codeAction` request (which
    /// fires on essentially every cursor move in many clients) - the actual
    /// edit is only computed once the user picks this from the Quick Fix
    /// menu, in `execute_command` (`server/backend.rs`). The title
    /// therefore promises *behavior*, not a specific file/call-site count,
    /// since that isn't known yet at this point.
    pub(super) fn unused_macro_or_function_parameter_removal_action(
        &self,
        document: &Document,
        range: Range
    ) -> Option<CodeAction> {
        let listing = self.parse_document(document).ok()?;
        let (lo, hi) = (range.start.line, range.end.line);

        for binding in cpclib_asm::unused_bindings::find_unused_bindings(listing.iter()) {
            let kind = match binding.kind {
                UnusedBindingKind::MacroParameter => RemoveParameterKind::Macro,
                UnusedBindingKind::FunctionParameter => RemoveParameterKind::Function,
                _ => continue
            };
            let Some(owner) = &binding.owner
            else {
                continue;
            };
            let def_line = (binding.line as u32).saturating_sub(1);
            if def_line < lo || def_line > hi {
                continue;
            }
            return Some(CodeAction {
                title: format!(
                    "Remove unused parameter '{}' and update all call sites",
                    binding.name
                ),
                kind: Some(CodeActionKind::QUICKFIX),
                command: Some(remove_unused_parameter_command(
                    &document.uri,
                    kind,
                    &owner.name,
                    owner.param_index
                )),
                ..Default::default()
            });
        }
        None
    }

    pub(super) fn wrap_action(
        &self,
        document: &Document,
        lines: &[&str],
        start_line: usize,
        end_line: usize,
        header: &str,
        footer: &str,
        placeholder: &str,
        title: &str,
        kind: CodeActionKind
    ) -> CodeAction {
        // Detect minimum indentation of non-empty selected lines.
        let indent = lines[start_line..=end_line]
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);

        // MACRO/ENDM always at column 0; body gets an extra \t when unindented.
        let mut new_text = format!("{header}\n");
        for &line in &lines[start_line..=end_line] {
            if indent == 0 {
                new_text.push('\t');
            }
            new_text.push_str(line.trim_end());
            new_text.push('\n');
        }
        new_text.push_str(&format!("{footer}\n"));

        let edit_range = Range {
            start: Position {
                line: start_line as u32,
                character: 0
            },
            end: Position {
                line: end_line as u32 + 1,
                character: 0
            }
        };

        // Select the placeholder text in the header line once the edit is applied,
        // so the user can immediately type a replacement (e.g. macro name / count).
        let command = header.find(placeholder).map(|col| {
            let header_line = start_line as u32;
            let placeholder_range = Range {
                start: Position {
                    line: header_line,
                    character: col as u32
                },
                end: Position {
                    line: header_line,
                    character: (col + placeholder.len()) as u32
                }
            };
            select_range_command(&document.uri, placeholder_range)
        });

        CodeAction {
            title: title.to_string(),
            kind: Some(kind),
            edit: Some(single_file_edit(document.uri.clone(), edit_range, new_text)),
            command,
            ..Default::default()
        }
    }

    pub(super) fn join_lines_action(
        &self,
        document: &Document,
        lines: &[&str],
        start_line: usize,
        end_line: usize
    ) -> Option<CodeAction> {
        // Indentation taken from the first non-empty line.
        let first = lines[start_line..=end_line]
            .iter()
            .find(|l| !l.trim().is_empty())?;
        let indent_len = first.len() - first.trim_start().len();
        let indent = &first[..indent_len];

        // Strip inline comments before joining so they don't eat subsequent parts.
        let parts: Vec<&str> = lines[start_line..=end_line]
            .iter()
            .map(|l| strip_asm_comment(l).trim())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() < 2 {
            return None;
        }

        let joined = format!("{}{}\n", indent, parts.join(" : "));
        let edit_range = Range {
            start: Position {
                line: start_line as u32,
                character: 0
            },
            end: Position {
                line: end_line as u32 + 1,
                character: 0
            }
        };
        Some(CodeAction {
            title: "Join selected lines (separate with :)".to_string(),
            kind: Some(CodeActionKind::REFACTOR_REWRITE),
            edit: Some(single_file_edit(document.uri.clone(), edit_range, joined)),
            ..Default::default()
        })
    }

    pub(super) fn split_lines_action(
        &self,
        document: &Document,
        lines: &[&str],
        start_line: usize,
        end_line: usize
    ) -> Option<CodeAction> {
        let mut new_text = String::new();
        let mut any_split = false;

        for &line in &lines[start_line..=end_line] {
            let indent_len = line.len() - line.trim_start().len();
            let indent = &line[..indent_len];
            let parts = split_at_colon(line);
            if parts.len() > 1 {
                any_split = true;
            }
            for part in parts {
                new_text.push_str(indent);
                new_text.push_str(part.trim_start());
                new_text.push('\n');
            }
        }

        if !any_split {
            return None;
        }

        let edit_range = Range {
            start: Position {
                line: start_line as u32,
                character: 0
            },
            end: Position {
                line: end_line as u32 + 1,
                character: 0
            }
        };
        Some(CodeAction {
            title: "Split lines at : (one instruction per line)".to_string(),
            kind: Some(CodeActionKind::REFACTOR_REWRITE),
            edit: Some(single_file_edit(document.uri.clone(), edit_range, new_text)),
            ..Default::default()
        })
    }
}

/// Whether `token`'s operand(s) match one of a small set of known no-op or
/// trivially-improvable Z80 instruction shapes, and if so, what to do about
/// it: `(title, replacement)`, where an empty `replacement` means "delete
/// the instruction entirely", and a non-empty one is the real-instruction
/// text to replace it with. Every rule is a pure syntactic match on the
/// instruction's own operand *shape* (same register in both slots, or a
/// literal `0`) - never on a register's runtime *value*, so this needs no
/// value-tracking (`registers.rs`'s `register_state_at` is unrelated).
///
/// - `ld rr,rr` (any same register, 8- or 16-bit) and `add 0`/`add a,0`:
///   deletable - they provably change nothing.
/// - `ld a,0` → `xor a`, `adc de,de` → `rl de`, `add de,de` → `sla de`,
///   `sub de,de` → `ld de,0`, `sub hl,hl` → `ld hl,0`: each a real,
///   equivalent, strictly simpler instruction.
fn classify_no_op_or_improvable<T: ListingElement>(
    token: &T
) -> Option<(&'static str, &'static str)> {
    let mnemonic = token.mnemonic()?;
    let arg1 = token.mnemonic_arg1();
    let arg2 = token.mnemonic_arg2();

    let is_literal_zero = |da: &T::DataAccess| {
        da.get_expression()
            .is_some_and(|e| e.is_value() && e.value() == 0)
    };
    let is_same_register = |a: &T::DataAccess, b: &T::DataAccess| {
        (a.is_register8() || a.is_register16() || a.is_indexregister8() || a.is_indexregister16())
            && a.to_data_access() == b.to_data_access()
    };

    match mnemonic {
        Mnemonic::Ld => {
            let (a1, a2) = (arg1?, arg2?);
            if is_same_register(a1, a2) {
                return Some((
                    "Delete no-op instruction (same source and destination register)",
                    ""
                ));
            }
            if a1.is_register_a() && is_literal_zero(a2) {
                return Some(("Replace with the equivalent, shorter 'xor a'", "xor a"));
            }
            None
        },
        Mnemonic::Add => {
            let a2 = arg2?;
            if arg1.map(|a| a.is_register_a()).unwrap_or(true) && is_literal_zero(a2) {
                return Some(("Delete no-op instruction (adding 0 changes nothing)", ""));
            }
            let a1 = arg1?;
            if a1.is_register_de() && a2.is_register_de() {
                return Some((
                    "Replace with the equivalent, real instruction 'sla de'",
                    "sla de"
                ));
            }
            None
        },
        Mnemonic::Adc => {
            let (a1, a2) = (arg1?, arg2?);
            if a1.is_register_de() && a2.is_register_de() {
                return Some((
                    "Replace with the equivalent, real instruction 'rl de'",
                    "rl de"
                ));
            }
            None
        },
        Mnemonic::Sub => {
            let (a1, a2) = (arg1?, arg2?);
            if a1.is_register_de() && a2.is_register_de() {
                return Some((
                    "Replace with the equivalent, real instruction 'ld de,0'",
                    "ld de,0"
                ));
            }
            if a1.is_register_hl() && a2.is_register_hl() {
                return Some((
                    "Replace with the equivalent, real instruction 'ld hl,0'",
                    "ld hl,0"
                ));
            }
            None
        },
        _ => None
    }
}

/// `token`'s real-instruction expansion, straight from
/// `ListingElement::fake_to_listing_from_access` - the same function the
/// real assembler calls to expand a fake instruction - built from the
/// token's own operands, not from assembled bytes. `None` if `token` isn't
/// a fake instruction this function recognizes.
fn fake_instruction_real_expansion<T: ListingElement>(
    token: &T
) -> Option<Vec<(Mnemonic, Option<DataAccess>, Option<DataAccess>)>> {
    let mnemonic = *token.mnemonic()?;
    T::fake_to_listing_from_access(mnemonic, token.mnemonic_arg1(), token.mnemonic_arg2(), None)
}

/// Renders one `(mnemonic, arg1, arg2)` real-instruction tuple as text,
/// matching `Token`'s own `Display` convention (`"{mne} {arg1}, {arg2}"`),
/// case-folded to `upper` - but *only* the mnemonic and any register
/// keywords, never a user expression (see `render_data_access`'s own doc
/// comment for why: an embedded symbol's case must survive exactly, e.g.
/// `(iy+VAR)` must not become `(iy+var)`).
fn render_expanded_instruction(
    mnemonic: Mnemonic,
    arg1: &Option<DataAccess>,
    arg2: &Option<DataAccess>,
    upper: bool
) -> String {
    let mne = fold_case(&mnemonic.to_string(), upper);
    match (arg1, arg2) {
        (Some(a1), Some(a2)) => {
            format!(
                "{mne} {}, {}",
                render_data_access(a1, upper),
                render_data_access(a2, upper)
            )
        },
        (Some(a1), None) => format!("{mne} {}", render_data_access(a1, upper)),
        (None, Some(a2)) => format!("{mne} {}", render_data_access(a2, upper)),
        (None, None) => mne
    }
}

/// Renders one `DataAccess` operand, case-folded to `upper` - but *only*
/// the parts that are always a fixed keyword (register names, `I`/`R`,
/// `(C)`), never an embedded `Expr`. A fake instruction's expansion can
/// carry through an operand straight from the original source (e.g. the
/// displacement in `(iy+VAR)`), and that `Expr` may be an arbitrary
/// case-sensitive user symbol - folding its case would silently reference a
/// different (or nonexistent) symbol, a real correctness bug caught in
/// review (`ld bc,(iy+VAR)` must expand to `...(iy + VAR)`, not
/// `...(iy + var)`).
fn render_data_access(da: &DataAccess, upper: bool) -> String {
    match da {
        DataAccess::IndexRegister16WithIndex(reg, op, delta) => {
            format!(
                "({} {op} {})",
                fold_case(&reg.to_string(), upper),
                delta.to_simplified_string()
            )
        },
        DataAccess::IndexRegister16(reg) => fold_case(&reg.to_string(), upper),
        DataAccess::Register16(reg) => fold_case(&reg.to_string(), upper),
        DataAccess::IndexRegister8(reg) => fold_case(&reg.to_string(), upper),
        DataAccess::Register8(reg) => fold_case(&reg.to_string(), upper),
        DataAccess::MemoryRegister16(reg) => format!("({})", fold_case(&reg.to_string(), upper)),
        DataAccess::MemoryIndexRegister16(reg) => {
            format!("({})", fold_case(&reg.to_string(), upper))
        },
        DataAccess::Expression(exp) => exp.to_simplified_string(),
        DataAccess::Memory(exp) => format!("({})", exp.to_simplified_string()),
        DataAccess::FlagTest(test) => fold_case(&test.to_string(), upper),
        DataAccess::SpecialRegisterI => fold_case("I", upper),
        DataAccess::SpecialRegisterR => fold_case("R", upper),
        DataAccess::PortC => "(C)".to_string(),
        DataAccess::PortN(exp) => format!("({})", exp.to_simplified_string())
    }
}

fn fold_case(s: &str, upper: bool) -> String {
    if upper {
        s.to_uppercase()
    }
    else {
        s.to_lowercase()
    }
}

/// Whether `original`'s own first alphabetic character is uppercase - used
/// to pick the case convention for synthesized replacement keywords, so a
/// quickfix's output stays visually consistent with the surrounding source.
fn is_uppercase_style(original: &str) -> bool {
    original
        .chars()
        .find(|c| c.is_ascii_alphabetic())
        .is_some_and(|c| c.is_ascii_uppercase())
}

/// Renders `replacement` in the same upper/lowercase convention as
/// `original`'s own first alphabetic character. Only safe for a
/// `replacement` that is a fixed, static keyword string with no embedded
/// user expression/symbol (e.g. the no-op/improvable-instruction
/// quickfix's hardcoded replacements like `"xor a"`) - see
/// `render_expanded_instruction`/`render_data_access` for the
/// component-wise approach a *dynamic*, expression-carrying replacement
/// (the fake-instruction quickfix) must use instead.
fn match_case_like(original: &str, replacement: &str) -> String {
    fold_case(replacement, is_uppercase_style(original))
}

#[cfg(test)]
mod unused_repeat_counter_removal_tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///main.asm").unwrap(), text.to_string(), 1)
    }

    fn cursor(line: u32, character: u32) -> Range {
        Range {
            start: Position { line, character },
            end: Position { line, character }
        }
    }

    #[test]
    fn offers_the_quickfix_with_a_correct_removal_edit_for_a_cursor_on_the_header_line() {
        let d = doc("REPEAT 5, i\n    nop\nENDR\n");
        let analyzer = AssemblyAnalyzer::new();
        let action = analyzer
            .unused_repeat_counter_removal_action(&d, cursor(0, 3))
            .expect("expected the quickfix");
        assert_eq!(action.title, "Remove unused loop counter 'i'");
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        let edit = action.edit.expect("expected an edit");
        let text_edits = &edit.changes.expect("expected changes")[&d.uri];
        assert_eq!(text_edits.len(), 1);
        assert_eq!(text_edits[0].new_text, "");
        assert_eq!(
            text_edits[0].range,
            Range {
                start: Position {
                    line: 0,
                    character: 8
                },
                end: Position {
                    line: 0,
                    character: 11
                }
            }
        );
    }

    #[test]
    fn is_wired_into_code_actions() {
        let d = doc("REPEAT 5, i\n    nop\nENDR\n");
        let analyzer = AssemblyAnalyzer::new();
        let actions = analyzer.code_actions(&d, cursor(0, 3));
        assert!(actions.iter().any(|a| {
            a.title == "Remove unused loop counter 'i'" && a.kind == Some(CodeActionKind::QUICKFIX)
        }));
    }

    #[test]
    fn no_quickfix_when_the_repeat_counter_is_used() {
        let d = doc("REPEAT 5, i\n    db {i}\nENDR\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, cursor(0, 3))
                .is_none()
        );
    }

    #[test]
    fn no_quickfix_for_a_repeat_with_no_counter_at_all() {
        let d = doc("REPEAT 5\n    nop\nENDR\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, cursor(0, 3))
                .is_none()
        );
    }

    #[test]
    fn no_quickfix_when_the_repeat_counter_has_an_explicit_start_value() {
        // Unused, but not offered: removing just ", i" here would silently
        // change behavior (dropping the "10" start value along with it), so
        // this stays warning-only, matching `removable_clause`'s own
        // documented scope in cpclib-asm.
        let d = doc("REPEAT 5, i, 10\n    nop\nENDR\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, cursor(0, 3))
                .is_none()
        );
    }

    #[test]
    fn no_quickfix_for_an_unused_iterate_counter() {
        let d = doc("ITERATE i, [1, 2, 3]\n    nop\nENDITERATE\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, cursor(0, 3))
                .is_none()
        );
    }

    #[test]
    fn no_quickfix_for_an_unused_for_counter() {
        let d = doc("FOR i, 0, 5, 1\n    nop\nENDFOR\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, cursor(0, 3))
                .is_none()
        );
    }

    #[test]
    fn no_quickfix_for_an_unused_macro_parameter() {
        let d = doc("MACRO foo, a, b, c\n    ld a, {a}\n    ld b, {b}\nENDM\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, cursor(0, 3))
                .is_none()
        );
    }

    #[test]
    fn no_quickfix_for_an_unused_function_parameter() {
        let d = doc("FUNCTION f, a, b\n    RETURN {a}\nENDFUNCTION\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, cursor(0, 3))
                .is_none()
        );
    }

    #[test]
    fn does_not_trigger_when_the_cursor_is_on_an_unrelated_line() {
        let d = doc("REPEAT 5, i\n    nop\nENDR\n\nld a, b\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, cursor(4, 0))
                .is_none()
        );
    }

    #[test]
    fn triggers_for_a_selection_spanning_the_header_line_not_just_a_collapsed_cursor() {
        let d = doc("REPEAT 5, i\n    nop\nENDR\n");
        let analyzer = AssemblyAnalyzer::new();
        let range = Range {
            start: Position {
                line: 0,
                character: 0
            },
            end: Position {
                line: 2,
                character: 4
            }
        };
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, range)
                .is_some()
        );
    }
}

#[cfg(test)]
mod unused_macro_or_function_parameter_removal_tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///main.asm").unwrap(), text.to_string(), 1)
    }

    fn cursor(line: u32, character: u32) -> Range {
        Range {
            start: Position { line, character },
            end: Position { line, character }
        }
    }

    #[test]
    fn offers_the_action_for_an_unused_macro_parameter_under_the_cursor() {
        let d = doc("MACRO foo, a, b, c\n    ld a, {a}\n    ld b, {b}\nENDM\n");
        let analyzer = AssemblyAnalyzer::new();
        let action = analyzer
            .unused_macro_or_function_parameter_removal_action(&d, cursor(0, 3))
            .expect("expected the action");
        assert_eq!(
            action.title,
            "Remove unused parameter 'c' and update all call sites"
        );
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        assert!(action.edit.is_none(), "the edit isn't known synchronously");
        let command = action.command.expect("expected a command");
        assert_eq!(command.command, "cpclib.removeUnusedParameter");
        let args = &command.arguments.expect("expected arguments")[0];
        assert_eq!(args["kind"], "macro");
        assert_eq!(args["ownerName"], "foo");
        assert_eq!(args["paramIndex"], 2);
    }

    #[test]
    fn offers_the_action_for_an_unused_function_parameter_under_the_cursor() {
        let d = doc("FUNCTION f, a, b\n    RETURN {a}\nENDFUNCTION\n");
        let analyzer = AssemblyAnalyzer::new();
        let action = analyzer
            .unused_macro_or_function_parameter_removal_action(&d, cursor(0, 3))
            .expect("expected the action");
        assert_eq!(
            action.title,
            "Remove unused parameter 'b' and update all call sites"
        );
        let command = action.command.expect("expected a command");
        let args = &command.arguments.expect("expected arguments")[0];
        assert_eq!(args["kind"], "function");
        assert_eq!(args["ownerName"], "f");
        assert_eq!(args["paramIndex"], 1);
    }

    #[test]
    fn no_action_when_every_macro_parameter_is_used() {
        let d = doc("MACRO foo, a, b\n    ld a, {a}\n    ld b, {b}\nENDM\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_macro_or_function_parameter_removal_action(&d, cursor(0, 3))
                .is_none()
        );
    }

    #[test]
    fn no_action_when_the_cursor_is_on_an_unrelated_line() {
        let d = doc("MACRO foo, a, b, c\n    ld a, {a}\n    ld b, {b}\nENDM\n\nld a, b\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_macro_or_function_parameter_removal_action(&d, cursor(5, 0))
                .is_none()
        );
    }

    #[test]
    fn no_action_for_an_unused_repeat_counter_it_stays_routed_through_the_other_action() {
        let d = doc("REPEAT 5, i\n    nop\nENDR\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .unused_macro_or_function_parameter_removal_action(&d, cursor(0, 3))
                .is_none()
        );
        // ...but the REPEAT-specific action still offers it.
        assert!(
            analyzer
                .unused_repeat_counter_removal_action(&d, cursor(0, 3))
                .is_some()
        );
    }

    #[test]
    fn is_wired_into_code_actions() {
        let d = doc("MACRO foo, a, b, c\n    ld a, {a}\n    ld b, {b}\nENDM\n");
        let analyzer = AssemblyAnalyzer::new();
        let actions = analyzer.code_actions(&d, cursor(0, 3));
        assert!(actions.iter().any(|a| {
            a.title == "Remove unused parameter 'c' and update all call sites"
                && a.kind == Some(CodeActionKind::QUICKFIX)
        }));
    }
}

#[cfg(test)]
mod redundant_accumulator_prefix_removal_tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///main.asm").unwrap(), text.to_string(), 1)
    }

    fn cursor(line: u32, character: u32) -> Range {
        Range {
            start: Position { line, character },
            end: Position { line, character }
        }
    }

    #[test]
    fn offers_the_quickfix_with_a_correct_removal_edit_for_cp() {
        let d = doc("org 0x4000\ncp a, c\nret\n");
        let analyzer = AssemblyAnalyzer::new();
        let action = analyzer
            .redundant_accumulator_prefix_removal_action(&d, cursor(1, 3))
            .expect("expected the quickfix");
        assert_eq!(
            action.title,
            "Remove redundant explicit 'A,' accumulator prefix"
        );
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        let edit = action.edit.expect("expected an edit");
        let text_edits = &edit.changes.expect("expected changes")[&d.uri];
        assert_eq!(text_edits.len(), 1);
        assert_eq!(text_edits[0].new_text, "");
        assert_eq!(
            text_edits[0].range,
            Range {
                start: Position {
                    line: 1,
                    character: 3
                },
                end: Position {
                    line: 1,
                    character: 6
                }
            }
        );
    }

    #[test]
    fn works_for_sub_and_add_too() {
        let analyzer = AssemblyAnalyzer::new();
        for src in ["org 0x4000\nsub a, c\nret\n", "org 0x4000\nadd a, c\nret\n"] {
            let d = doc(src);
            assert!(
                analyzer
                    .redundant_accumulator_prefix_removal_action(&d, cursor(1, 3))
                    .is_some(),
                "{src:?}"
            );
        }
    }

    #[test]
    fn no_quickfix_for_the_bare_implicit_accumulator_form() {
        let d = doc("org 0x4000\ncp c\nret\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .redundant_accumulator_prefix_removal_action(&d, cursor(1, 3))
                .is_none()
        );
    }

    #[test]
    fn no_quickfix_for_a_genuine_fake_instruction() {
        // `sub hl, bc` is the real fake-16-bit-form warning, not a redundant
        // accumulator prefix - must not be offered this quickfix.
        let d = doc("org 0x4000\nsub hl, bc\nret\n");
        let analyzer = AssemblyAnalyzer::new();
        assert!(
            analyzer
                .redundant_accumulator_prefix_removal_action(&d, cursor(1, 3))
                .is_none()
        );
    }

    #[test]
    fn is_wired_into_code_actions() {
        let d = doc("org 0x4000\ncp a, c\nret\n");
        let analyzer = AssemblyAnalyzer::new();
        let actions = analyzer.code_actions(&d, cursor(1, 3));
        assert!(actions.iter().any(|a| {
            a.title == "Remove redundant explicit 'A,' accumulator prefix"
                && a.kind == Some(CodeActionKind::QUICKFIX)
        }));
    }

    #[test]
    fn applying_the_edit_produces_the_bare_form() {
        let d = doc("org 0x4000\ncp a, c\nret\n");
        let analyzer = AssemblyAnalyzer::new();
        let action = analyzer
            .redundant_accumulator_prefix_removal_action(&d, cursor(1, 3))
            .expect("expected the quickfix");
        let edit = action.edit.expect("expected an edit");
        let text_edits = &edit.changes.expect("expected changes")[&d.uri];
        let line = "cp a, c";
        let r = &text_edits[0].range;
        let new_line = format!(
            "{}{}{}",
            &line[..r.start.character as usize],
            text_edits[0].new_text,
            &line[r.end.character as usize..]
        );
        assert_eq!(new_line, "cp c");
    }
}

#[cfg(test)]
mod no_op_or_improvable_instruction_tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///main.asm").unwrap(), text.to_string(), 1)
    }

    fn cursor(line: u32, character: u32) -> Range {
        Range {
            start: Position { line, character },
            end: Position { line, character }
        }
    }

    fn action_for(text: &str, line: u32) -> Option<CodeAction> {
        let d = doc(text);
        AssemblyAnalyzer::new().no_op_or_improvable_instruction_action(&d, cursor(line, 3))
    }

    fn applied(text: &str, line: u32) -> String {
        let action = action_for(text, line).expect("expected the quickfix");
        let edit = action.edit.expect("expected an edit");
        let uri = Url::parse("file:///main.asm").unwrap();
        let text_edits = &edit.changes.expect("expected changes")[&uri];
        assert_eq!(text_edits.len(), 1);
        let line_text = text.lines().nth(line as usize).unwrap();
        let r = &text_edits[0].range;
        format!(
            "{}{}{}",
            &line_text[..r.start.character as usize],
            text_edits[0].new_text,
            &line_text[r.end.character as usize..]
        )
    }

    #[test]
    fn ld_same_8bit_register_is_deletable() {
        let text = "org 0x4000\nld a, a\nret\n";
        let action = action_for(text, 1).expect("expected the quickfix");
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        assert!(action.title.contains("Delete"), "{}", action.title);
        assert_eq!(applied(text, 1), "");
    }

    #[test]
    fn ld_same_16bit_register_is_deletable() {
        assert_eq!(applied("org 0x4000\nld bc, bc\nret\n", 1), "");
        assert_eq!(applied("org 0x4000\nld de, de\nret\n", 1), "");
        assert_eq!(applied("org 0x4000\nld hl, hl\nret\n", 1), "");
    }

    #[test]
    fn ld_a_0_becomes_xor_a() {
        assert_eq!(applied("org 0x4000\nld a, 0\nret\n", 1), "xor a");
    }

    #[test]
    fn add_0_implicit_a_is_deletable() {
        assert_eq!(applied("org 0x4000\nadd 0\nret\n", 1), "");
    }

    #[test]
    fn add_a_0_explicit_is_deletable() {
        assert_eq!(applied("org 0x4000\nadd a, 0\nret\n", 1), "");
    }

    #[test]
    fn adc_de_de_becomes_rl_de() {
        assert_eq!(applied("org 0x4000\nadc de, de\nret\n", 1), "rl de");
    }

    #[test]
    fn add_de_de_becomes_sla_de() {
        assert_eq!(applied("org 0x4000\nadd de, de\nret\n", 1), "sla de");
    }

    #[test]
    fn sub_de_de_becomes_ld_de_0() {
        assert_eq!(applied("org 0x4000\nsub de, de\nret\n", 1), "ld de,0");
    }

    #[test]
    fn sub_hl_hl_becomes_ld_hl_0() {
        assert_eq!(applied("org 0x4000\nsub hl, hl\nret\n", 1), "ld hl,0");
    }

    #[test]
    fn replacement_case_follows_the_original_instruction() {
        assert_eq!(applied("org 0x4000\nLD A, 0\nret\n", 1), "XOR A");
        assert_eq!(applied("org 0x4000\nSUB HL, HL\nret\n", 1), "LD HL,0");
    }

    #[test]
    fn add_hl_hl_is_a_real_instruction_not_flagged() {
        // A genuine, common doubling idiom (add hl,hl only exists as a real
        // 16-bit ADD - unlike add de,de/adc de,de/sub de,de/sub hl,hl, which
        // are all fake instructions on real Z80).
        assert!(action_for("org 0x4000\nadd hl, hl\nret\n", 1).is_none());
    }

    #[test]
    fn ld_different_registers_is_not_flagged() {
        assert!(action_for("org 0x4000\nld a, c\nret\n", 1).is_none());
    }

    #[test]
    fn add_a_nonzero_is_not_flagged() {
        assert!(action_for("org 0x4000\nadd a, 5\nret\n", 1).is_none());
    }

    #[test]
    fn is_wired_into_code_actions() {
        let d = doc("org 0x4000\nld a, a\nret\n");
        let analyzer = AssemblyAnalyzer::new();
        let actions = analyzer.code_actions(&d, cursor(1, 3));
        assert!(
            actions
                .iter()
                .any(|a| a.kind == Some(CodeActionKind::QUICKFIX) && a.title.contains("Delete"))
        );
    }
}

#[cfg(test)]
mod fake_instruction_to_real_tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///main.asm").unwrap(), text.to_string(), 1)
    }

    fn cursor(line: u32, character: u32) -> Range {
        Range {
            start: Position { line, character },
            end: Position { line, character }
        }
    }

    fn applied(text: &str, line: u32) -> String {
        let d = doc(text);
        let action = AssemblyAnalyzer::new()
            .fake_instruction_to_real_action(&d, cursor(line, 3))
            .expect("expected the quickfix");
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        let edit = action.edit.expect("expected an edit");
        let uri = Url::parse("file:///main.asm").unwrap();
        let text_edits = &edit.changes.expect("expected changes")[&uri];
        assert_eq!(text_edits.len(), 1);
        let line_text = text.lines().nth(line as usize).unwrap();
        let r = &text_edits[0].range;
        format!(
            "{}{}{}",
            &line_text[..r.start.character as usize],
            text_edits[0].new_text,
            &line_text[r.end.character as usize..]
        )
    }

    #[test]
    fn sub_hl_bc_expands_to_its_real_instructions_joined_by_colon() {
        let real = applied("org 0x4000\nsub hl, bc\nret\n", 1);
        // Two real instructions, colon-joined on one line (basm's real
        // statement separator) - not `disassemble_snippet`'s `" ; "` join,
        // which would silently comment out everything after the first.
        assert!(!real.contains(';'), "{real}");
        let parts: Vec<&str> = real.split(':').collect();
        assert_eq!(parts.len(), 2, "{real:?}");
        assert!(parts[0].to_lowercase().contains("or"), "{real}");
        assert!(parts[1].to_lowercase().contains("sbc"), "{real}");
    }

    #[test]
    fn replacement_case_follows_the_original_instruction() {
        let real = applied("org 0x4000\nSUB HL, BC\nret\n", 1);
        assert_eq!(real, real.to_uppercase(), "{real}");
    }

    /// Regression test for a user report: a symbol embedded in a fake
    /// instruction's expansion (here, the `(iy+VAR)` displacement) must
    /// keep its own original case exactly - only the synthesized mnemonic/
    /// register keywords should follow the surrounding lowercase style.
    /// `ld bc,(iy+VAR)` must expand to `...(iy + VAR)`, never
    /// `...(iy + var)` (a different, and likely nonexistent, symbol).
    #[test]
    fn embedded_symbol_case_is_preserved_even_when_keywords_are_lowercased() {
        let real = applied("org 0x4000\nld bc,(iy+VAR)\nret\n", 1);
        assert!(real.contains("VAR"), "{real}");
        assert!(!real.contains("var"), "{real}");
        // Keywords (mnemonic/registers) still follow the lowercase source style.
        assert!(real.contains("ld"), "{real}");
        assert!(real.contains("iy"), "{real}");
    }

    #[test]
    fn embedded_symbol_case_is_preserved_in_uppercase_style_source_too() {
        let real = applied("ORG 0x4000\nLD BC,(IY+var)\nRET\n", 1);
        assert!(real.contains("var"), "{real}");
        assert!(!real.contains("VAR"), "{real}");
        assert!(real.contains("LD"), "{real}");
        assert!(real.contains("IY"), "{real}");
    }

    #[test]
    fn not_offered_for_a_real_non_fake_instruction() {
        let d = doc("org 0x4000\nld a, c\nret\n");
        assert!(
            AssemblyAnalyzer::new()
                .fake_instruction_to_real_action(&d, cursor(1, 3))
                .is_none()
        );
    }

    #[test]
    fn not_offered_for_a_redundant_accumulator_prefix_it_is_not_a_fake_instruction() {
        let d = doc("org 0x4000\ncp a, c\nret\n");
        assert!(
            AssemblyAnalyzer::new()
                .fake_instruction_to_real_action(&d, cursor(1, 3))
                .is_none()
        );
    }

    #[test]
    fn is_wired_into_code_actions() {
        let d = doc("org 0x4000\nsub hl, bc\nret\n");
        let analyzer = AssemblyAnalyzer::new();
        let actions = analyzer.code_actions(&d, cursor(1, 3));
        assert!(
            actions.iter().any(|a| {
                a.kind == Some(CodeActionKind::QUICKFIX)
                    && a.title == "Replace fake instruction with its real instruction(s)"
            }),
            "{actions:?}"
        );
    }
}

#[cfg(test)]
mod firmware_symbol_replacement_tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///main.asm").unwrap(), text.to_string(), 1)
    }

    fn cursor(line: u32, character: u32) -> Range {
        Range {
            start: Position { line, character },
            end: Position { line, character }
        }
    }

    #[test]
    fn offers_the_quickfix_for_a_known_firmware_address() {
        let d = doc("org 0x4000\ncall 0xBB5A\nret\n");
        // Cursor on the literal (after "call ").
        let action = AssemblyAnalyzer::new()
            .firmware_symbol_replacement_action(&d, cursor(1, 7))
            .expect("expected the quickfix");
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        assert_eq!(action.title, "Replace with firmware symbol 'TXT_OUTPUT'");
    }

    #[test]
    fn replaces_the_literal_and_prepends_include_once_when_missing() {
        let d = doc("org 0x4000\ncall 0xBB5A\nret\n");
        let action = AssemblyAnalyzer::new()
            .firmware_symbol_replacement_action(&d, cursor(1, 7))
            .expect("expected the quickfix");
        let edit = action.edit.expect("expected an edit");
        let text_edits = &edit.changes.expect("expected changes")[&d.uri];
        assert_eq!(text_edits.len(), 2, "{text_edits:?}");

        let literal_edit = text_edits
            .iter()
            .find(|e| e.range.start.line == 1)
            .expect("expected the literal replacement edit");
        assert_eq!(literal_edit.new_text, "TXT_OUTPUT");
        let line = "call 0xBB5A";
        let new_line = format!(
            "{}{}{}",
            &line[..literal_edit.range.start.character as usize],
            literal_edit.new_text,
            &line[literal_edit.range.end.character as usize..]
        );
        assert_eq!(new_line, "call TXT_OUTPUT");

        let include_edit = text_edits
            .iter()
            .find(|e| e.range.start.line == 0 && e.range.start.character == 0)
            .expect("expected the include-once edit");
        assert!(
            include_edit.new_text.contains("include once"),
            "{}",
            include_edit.new_text
        );
        assert!(
            include_edit.new_text.contains("inner://firmware/"),
            "{}",
            include_edit.new_text
        );
    }

    #[test]
    fn does_not_duplicate_an_already_present_include() {
        // TXT_OUTPUT is documented in txtvdu.asm - confirmed via
        // `firmware_docs::lookup_by_value(0xBB5A).source_file`.
        let d = doc("include once \"inner://firmware/txtvdu.asm\"\norg 0x4000\ncall 0xBB5A\nret\n");
        let action = AssemblyAnalyzer::new()
            .firmware_symbol_replacement_action(&d, cursor(2, 7))
            .expect("expected the quickfix");
        let edit = action.edit.expect("expected an edit");
        let text_edits = &edit.changes.expect("expected changes")[&d.uri];
        assert_eq!(
            text_edits.len(),
            1,
            "no include edit should be added when already present: {text_edits:?}"
        );
        assert_eq!(text_edits[0].new_text, "TXT_OUTPUT");
    }

    #[test]
    fn not_offered_for_an_unrecognized_literal() {
        let d = doc("org 0x4000\nld a, 0x1234\nret\n");
        assert!(
            AssemblyAnalyzer::new()
                .firmware_symbol_replacement_action(&d, cursor(1, 6))
                .is_none()
        );
    }

    #[test]
    fn is_wired_into_code_actions() {
        let d = doc("org 0x4000\ncall 0xBB5A\nret\n");
        let analyzer = AssemblyAnalyzer::new();
        let actions = analyzer.code_actions(&d, cursor(1, 7));
        assert!(
            actions.iter().any(|a| {
                a.kind == Some(CodeActionKind::QUICKFIX)
                    && a.title == "Replace with firmware symbol 'TXT_OUTPUT'"
            }),
            "{actions:?}"
        );
    }

    /// The caret is rarely *on* the number.
    ///
    /// A hover is driven by the mouse, so hovering `0xBB5A` named `TXT_OUTPUT`
    /// while the lightbulb stayed empty - the editor asks for actions wherever
    /// the caret is, which is the end of the line or the mnemonic. Reported
    /// from real use.
    #[test]
    fn the_quickfix_is_offered_from_anywhere_on_the_line() {
        let d = doc("\tcall 0xBB5A\n");
        let analyzer = AssemblyAnalyzer::new();

        // Caret at the start of the line, on the mnemonic, and past the end of
        // the number - every place a caret actually sits.
        for character in [0u32, 2, 4, 11] {
            let action = analyzer
                .firmware_symbol_replacement_action(&d, cursor(0, character))
                .unwrap_or_else(|| panic!("no quickfix with the caret at {character}"));
            assert!(action.title.contains("TXT_OUTPUT"), "{}", action.title);
        }
    }

    /// ...and it is reachable through the editor's own entry point, with the
    /// caret where the editor really puts it.
    #[test]
    fn the_line_wide_quickfix_is_wired_into_code_actions() {
        let d = doc("\tcall 0xBB5A\n");
        let actions = AssemblyAnalyzer::new().code_actions(&d, cursor(0, 0));
        assert!(
            actions.iter().any(|a| {
                a.title
                    .contains("Replace with firmware symbol 'TXT_OUTPUT'")
            }),
            "{actions:?}"
        );
    }

    /// A line with no firmware address offers nothing, rather than the first
    /// number it can find.
    #[test]
    fn an_ordinary_number_is_not_offered_as_firmware() {
        let d = doc("\tld a, 0x12\n");
        assert!(
            AssemblyAnalyzer::new()
                .firmware_symbol_replacement_action(&d, cursor(0, 0))
                .is_none()
        );
    }
}

/// Byte range of the number in the first `$ - <number>` at or after `from`.
///
/// Used to rewrite an offset in place rather than the whole `equ`, so the
/// author's spacing and any trailing comment survive the fix.
fn offset_literal_span(line: &str, from: usize) -> Option<(usize, usize)> {
    let dollar = from + line.get(from..)?.find('$')?;
    let after = line.get(dollar + 1..)?;
    let minus = after.find('-')?;
    if !after[..minus].chars().all(char::is_whitespace) {
        return None;
    }
    let digits_from = dollar + 1 + minus + 1;
    let rest = line.get(digits_from..)?;
    let leading = rest.len() - rest.trim_start().len();
    let start = digits_from + leading;
    let digits = line.get(start..)?;
    let len = digits
        .find(|c: char| !c.is_ascii_digit())?
        .min(digits.len());
    if len == 0 {
        return None;
    }
    Some((start, start + len))
}
