use cpclib_asm::{ListingElement, LocatedToken, MayHaveSpan};

use super::Formatter;
use crate::options::LabelPostfix;

impl<'src> Formatter<'src> {
    /// The indentation depth an assignment/EQU should actually render at.
    ///
    /// `=`/`EQU` define a symbol, and outside a FUNCTION that symbol is
    /// always globally visible - IF/REPEAT/WHILE/... bodies never scope
    /// anything, they only decide whether/how many times their body
    /// assembles at all (confirmed live: a variable set inside a plain `IF`
    /// or `REPEAT` is still readable straight after it). So a definition
    /// textually inside one of those is, semantically, exactly as global as
    /// a top-level one, and keeps that convention's column 0 regardless of
    /// how deep the surrounding IF/REPEAT nesting happens to be - matching
    /// real project style found this way (skyline's own source keeps such
    /// definitions flush left even nested several IFs deep).
    ///
    /// A FUNCTION body is different: it has real local scope (confirmed
    /// live: `Unknown symbol` for a name assigned inside one, if referenced
    /// after it), so an assignment there is an ordinary local statement, not
    /// a declaration - it keeps the surrounding block's own depth like its
    /// sibling statements (`RETURN`, ...), the same way real hand-written
    /// code in this style already aligns it (confirmed against a real
    /// project's own source, not just guessed). `function_nesting` stays set
    /// through any further IF/REPEAT/... wrapping *within* that FUNCTION
    /// body, since those don't open a scope of their own either - the
    /// assignment is still local to the enclosing FUNCTION.
    fn assign_depth(&self, depth: usize) -> usize {
        if self.function_nesting > 0 { depth } else { 0 }
    }

    pub fn format_tokens(&mut self, tokens: &[LocatedToken], depth: usize) {
        for token in tokens {
            let (line_1, _) = token.span().relative_line_and_column();
            let line_0 = line_1.saturating_sub(1);
            self.emit_interstitial(line_0);
            self.format_token(token, depth, line_0);
        }
    }

    // Whatever the real parser says comes right after `own_text` (this token's
    // own already-trimmed content, e.g. a label's name or an instruction's own
    // span) on its source line: a genuine trailing `;comment` if that really is
    // all that remains (a leading `:` separator, if any, is skipped first, since
    // it belongs to whatever follows, not to this token) - or `None` if further
    // non-comment text follows, meaning a *different* token on this same line
    // owns that trailing comment instead, whenever its own turn comes.
    //
    // Finds `own_text` by searching for it in the source line (from the start of
    // the line, matching the leftmost occurrence) rather than trusting a byte
    // offset computed from `relative_line_and_column()`: `MayHaveSpan` spans in
    // this codebase are the substring the parser actually matched, which is
    // reliable content but not always reliably positioned relative to the raw
    // source line's own bytes once macro/struct expansion is involved. A
    // substring search is slightly more conservative but never misattributes a
    // comment to the wrong token, which a wrong offset could.
    fn trailing_comment_for_span(&self, own_text: &str, line_0: usize) -> Option<String> {
        let src_line = self.source_lines.get(line_0).copied()?;
        let start = src_line.find(own_text)?;
        let after = &src_line[start + own_text.len()..];
        let after = after.trim_start();
        let after = after.strip_prefix(':').unwrap_or(after).trim_start();
        let (rest, comment) = Self::split_comment(after);
        if rest.trim().is_empty() { comment.map(str::to_string) } else { None }
    }

    fn format_token(&mut self, token: &LocatedToken, depth: usize, line_0: usize) {
        // `; fmt: off` .. `; fmt: on` - pass the whole range through verbatim
        // and skip every token whose line falls in it entirely (comments,
        // labels, instructions, blocks alike - see `pragma`'s own doc
        // comment for why). `current_line` already past `end` here just
        // means an earlier token in the same disabled range already handled
        // it - the loop below then does nothing, so a range is never emitted
        // twice no matter how many tokens sit inside it.
        if let Some(&(_, end)) = self.disabled_ranges.iter().find(|&&(start, end)| line_0 >= start && line_0 <= end) {
            self.emit_verbatim_through(end);
            return;
        }

        if token.is_warning() {
            self.format_token(token.warning_token(), depth, line_0);
            return;
        }

        // Standalone comment line → emit verbatim; trailing comment (same line as an
        // instruction already emitted) → skip to avoid duplication.
        if token.is_comment() {
            if line_0 < self.current_line {
                return;
            }
            let src = self.source_lines.get(line_0).copied().unwrap_or("");
            self.output.push_str(src);
            self.output.push('\n');
            self.current_line = line_0 + 1;
            return;
        }

        if token.is_label() {
            self.format_label(token, depth, line_0);
        }
        else if token.is_if() {
            self.format_if(token, depth, line_0);
        }
        else if token.is_repeat() {
            self.format_block(
                token.repeat_listing(),
                depth,
                line_0,
                &["ENDREPEAT", "ENDREPT", "ENDREP", "ENDR", "REND"]
            );
        }
        else if token.is_while() {
            self.format_block(
                token.while_listing(),
                depth,
                line_0,
                &["ENDWHILE", "ENDW", "WEND"]
            );
        }
        else if token.is_for() {
            self.format_block(
                token.for_listing(),
                depth,
                line_0,
                &["ENDFOR", "FEND", "ENDF"]
            );
        }
        else if token.is_module() {
            self.format_block(token.module_listing(), depth, line_0, &["ENDMODULE"]);
        }
        else if token.is_confined() {
            self.format_block(
                token.confined_listing(),
                depth,
                line_0,
                &["ENDCONFINED", "CEND", "ENDC"]
            );
        }
        else if token.is_repeat_until() {
            self.format_repeat_until(token, depth, line_0);
        }
        else if token.is_iterate() {
            self.format_block(
                token.iterate_listing(),
                depth,
                line_0,
                &["ENDITERATE", "ENDITER", "ENDI", "IEND"]
            );
        }
        else if token.is_rorg() {
            self.format_block(
                token.rorg_listing(),
                depth,
                line_0,
                &["DEPHASE", "REND", "ENDR"]
            );
        }
        else if token.is_crunched_section() {
            self.format_block(
                token.crunched_section_listing(),
                depth,
                line_0,
                &["LZCLOSE"]
            );
        }
        else if token.is_function_definition() {
            // A FUNCTION body has real local scope in cpclib-asm/basm (confirmed
            // live: a variable assigned inside one is `Unknown symbol` outside
            // it) - unlike IF/REPEAT/WHILE/..., whose bodies never scope a
            // symbol at all, only conditionally/repeatedly assemble it. That
            // distinction is what `assign_depth` keys off.
            self.function_nesting += 1;
            self.format_block(
                token.function_definition_inner(),
                depth,
                line_0,
                &["ENDFUNCTION", "ENDF"]
            );
            self.function_nesting -= 1;
        }
        else if token.is_switch() {
            self.format_switch(token, depth, line_0);
        }
        else if token.is_macro_definition() {
            self.format_macro_def(token, depth, line_0);
        }
        else {
            // Simple instruction, directive, or macro call.
            self.format_simple(token, depth, line_0);
        }
    }

    fn format_label(&mut self, token: &LocatedToken, depth: usize, line_0: usize) {
        let name = token.label_symbol();

        // Determine whether to emit the trailing ':' based on the postfix option.
        let src_line = self.source_lines.get(line_0).copied().unwrap_or("");
        let original_had_colon = src_line
            .trim_start()
            .strip_prefix(name)
            .is_some_and(|rest| rest.trim_start().starts_with(':'));
        let emit_colon = match self.label_definition_postfix_with_column {
            LabelPostfix::WithColumn => true,
            LabelPostfix::NoColumn => false,
            LabelPostfix::Untouched => original_had_colon
        };
        let label_str = if emit_colon {
            format!("{name}:")
        }
        else {
            name.to_string()
        };

        if self.one_instruction_per_line {
            // A trailing instruction sharing this source line (`myloop: ld a,0`,
            // or even `myloop ld a,0` with no `:` at all - the real parser can
            // tell those apart from a label's own name whether or not a colon is
            // there) is simply the *next* token the real parser already produced
            // - it will be visited on its own right after this call returns, and
            // rendered by `format_simple` from its own span, landing on a new
            // output line automatically. Nothing to extract or re-inject here.
            let comment = self.trailing_comment_for_span(name, line_0);
            self.emit_line(0, &label_str, comment.as_deref());
        }
        else {
            let src = self.source_lines.get(line_0).copied().unwrap_or("");
            let (content_no_comment, comment) = Self::split_comment(src.trim());
            // Extract any instruction content that follows the label name on the same source line.
            let after_label = content_no_comment
                .trim_start()
                .strip_prefix(name)
                .map(|rest| rest.trim_start_matches(':').trim())
                .unwrap_or("");
            if after_label.is_empty() {
                self.emit_line(0, &label_str, comment);
            }
            else {
                // Label and instruction share a line in the source; split them out.
                // Emit the instruction content verbatim to avoid misidentifying
                // struct/macro names as mnemonics and applying the wrong case.
                self.emit_line(0, &label_str, None);
                let after = Self::normalize_colon_spacing(after_label, self.space_around_column);
                let after = self.apply_comma_and_quote_style(&after);
                self.emit_line(depth, &after, comment);
            }
        }
        self.current_line = line_0 + 1;
    }

    // Format a non-block, non-label token.
    fn format_simple(&mut self, token: &LocatedToken, depth: usize, line_0: usize) {
        let (content, comment) = if self.one_instruction_per_line {
            // The token's own span *is* its content - the real parser already
            // resolved every ambiguity a text scan would otherwise have to
            // guess at (a bare `NOP` before `:` is the mnemonic, not a label; a
            // numeric operand butting against `:` is not a label either; an
            // identifier operand that happens to be spelled like one isn't a
            // label just because nothing but the type checker could tell the
            // difference from text alone). No separate split pass, no segment
            // cache - each token here renders from exactly the text the parser
            // says belongs to it.
            let raw: &str = token.span().as_ref();
            let content = raw.lines().next().unwrap_or(raw).trim().to_string();
            let comment = if raw.contains('\n') {
                // A multi-line token's trailing comment (if any) belongs on its
                // last physical line, handled with the verbatim continuation
                // lines below, not here.
                None
            }
            else {
                self.trailing_comment_for_span(&content, line_0)
            };
            (content, comment)
        }
        else {
            // Without splitting: skip tokens that land on an already-emitted source line.
            if line_0 < self.current_line {
                return;
            }
            let src = self.source_lines.get(line_0).copied().unwrap_or("");
            let (c, cmt) = Self::split_comment(src.trim());
            // Reformat instruction-separator spacing if requested.
            let c = Self::normalize_colon_spacing(c, self.space_around_column);
            (c, cmt.map(str::to_string))
        };

        if token.mnemonic().is_some() {
            let out = Self::apply_mnemonic_case(&content, self.mnemonic_case, self.register_case);
            let out = self.reformat_numeric_literals(&out);
            let out = self.apply_comma_and_quote_style(&out);
            self.emit_line(depth, &out, comment.as_deref());
        }
        else if token.is_call_macro_or_build_struct() {
            // Macro names are user-defined: preserve casing; only reformat numeric literals.
            let out = self.reformat_numeric_literals(&content);
            let out = self.apply_comma_and_quote_style(&out);
            self.emit_line(depth, &out, comment.as_deref());
        }
        else if token.is_assign() {
            // Symbol assignment (label = value, label += value, etc.) - see
            // `assign_depth`'s own doc comment for the FUNCTION-vs-everything-
            // else distinction this depends on.
            let out = Self::normalize_assignment_spacing(&content, self.space_around_assignment);
            let out = self.reformat_numeric_literals(&out);
            let out = self.apply_comma_and_quote_style(&out);
            self.emit_line(self.assign_depth(depth), &out, comment.as_deref());
        }
        else if token.is_equ() {
            // "symbol EQU value": same reasoning as `is_assign` above; apply
            // directive_case only to the keyword (second word).
            let out = Self::apply_case_to_second_word(&content, self.directive_case);
            let out = self.reformat_numeric_literals(&out);
            let out = self.apply_comma_and_quote_style(&out);
            self.emit_line(self.assign_depth(depth), &out, comment.as_deref());
        }
        else {
            // Directives where a user-defined symbol precedes the keyword (like SETN/NEXT)
            // must not have that symbol name case-converted.
            // All directives where a user-defined symbol precedes the keyword:
            // SETN/NEXT (set-next symbol), FIELD/# (MAP entry allocation).
            const LABEL_FIRST_KWS: &[&str] = &["SETN", "SETNX", "NEXT", "FIELD", "#"];
            let second_word_upper = content
                .split_ascii_whitespace()
                .nth(1)
                .map(|w| w.trim_start_matches(':').to_ascii_uppercase())
                .unwrap_or_default();
            if LABEL_FIRST_KWS.contains(&second_word_upper.as_str())
                || second_word_upper.starts_with('#')
            {
                // Label-first directives: label is a top-level symbol name → column 0.
                let out = Self::apply_case_to_second_word(&content, self.directive_case);
                let out = self.reformat_numeric_literals(&out);
                let out = self.apply_comma_and_quote_style(&out);
                self.emit_line(0, &out, comment.as_deref());
            }
            else {
                // All other directives: keyword is the first word.
                let out = Self::apply_case_to_first_word(&content, self.directive_case);
                let out = self.reformat_numeric_literals(&out);
                let out = self.apply_comma_and_quote_style(&out);
                self.emit_line(depth, &out, comment.as_deref());
            }
        }

        // If the token spans multiple source lines (e.g. a multi-line expression),
        // emit the continuation lines verbatim so the assembler sees the complete syntax.
        let span_lines: usize = {
            let s: &str = token.span().as_ref();
            s.lines().count().max(1)
        };
        if span_lines > 1 {
            for i in 1..span_lines {
                if let Some(src) = self.source_lines.get(line_0 + i).copied() {
                    self.output.push_str(src);
                    self.output.push('\n');
                }
            }
            self.current_line = self.current_line.max(line_0 + span_lines);
        }
        else if line_0 >= self.current_line {
            self.current_line = line_0 + 1;
        }
    }

    fn format_block(
        &mut self,
        inner: &[LocatedToken],
        depth: usize,
        line_0: usize,
        closers: &[&str]
    ) {
        if line_0 >= self.current_line {
            self.emit_source_line_indented(depth, line_0);
            self.current_line = line_0 + 1;
        }
        if self.is_block_inline(inner, line_0) {
            return;
        }
        self.format_tokens(inner, depth + 1);
        self.emit_closer(depth, closers);
    }

    // Returns true if a block whose header is at `line_0` has its first inner token on the
    // same source line (i.e. the whole block is inline: `if x : body : endif`).
    fn is_block_inline(&self, inner: &[LocatedToken], line_0: usize) -> bool {
        inner
            .first()
            .map(|t| t.span().relative_line_and_column().0.saturating_sub(1) == line_0)
            // Empty body but closer might still be on the same line (e.g. `if x : endif`).
            .unwrap_or_else(|| {
                self.source_lines
                    .get(line_0)
                    .map(|l| {
                        let u = l.to_ascii_uppercase();
                        u.contains("ENDIF") || u.contains("ENDM") || u.contains("ENDREPEAT")
                    })
                    .unwrap_or(false)
            })
    }

    fn format_if(&mut self, token: &LocatedToken, depth: usize, line_0: usize) {
        let nb_tests = token.if_nb_tests();
        let (_, inner_0) = token.if_test(0);

        if line_0 >= self.current_line {
            self.emit_source_line_indented(depth, line_0);
            self.current_line = line_0 + 1;
        }

        // Inline IF (all on one source line): header already emitted; skip body/closer.
        if self.is_block_inline(inner_0, line_0) {
            return;
        }

        self.format_tokens(inner_0, depth + 1);

        for i in 1..nb_tests {
            // Determine the ELSEIF* header line from the first body token's source position
            // (the header is always the line immediately before the body).
            // This handles ELSEIFDEF/ELSEIFNDEF/ELSEIF etc. without keyword-specific searches.
            let (_, inner_i) = token.if_test(i);
            let header = inner_i
                .first()
                .map(|t| t.span().relative_line_and_column().0.saturating_sub(2))
                .unwrap_or_else(|| self.find_closer_start(&["ELSEIF", "ELSE"]));
            self.emit_interstitial(header);
            self.emit_source_line_indented(depth, header);
            self.current_line = header + 1;
            self.format_tokens(inner_i, depth + 1);
        }

        if let Some(else_inner) = token.if_else() {
            let else_line = self.find_closer_start(&["ELSE"]);
            self.emit_interstitial(else_line);
            self.emit_source_line_indented(depth, else_line);
            self.current_line = else_line + 1;
            self.format_tokens(else_inner, depth + 1);
        }

        self.emit_closer(depth, &["ENDIF"]);
    }

    fn format_repeat_until(&mut self, token: &LocatedToken, depth: usize, line_0: usize) {
        if line_0 >= self.current_line {
            self.emit_source_line_indented(depth, line_0);
            self.current_line = line_0 + 1;
        }
        self.format_tokens(token.repeat_until_listing(), depth + 1);
        self.emit_closer(depth, &["UNTIL"]);
    }

    fn format_macro_def(&mut self, token: &LocatedToken, depth: usize, line_0: usize) {
        // Emit the macro header applying directive_case only to the MACRO keyword, not the
        // user-defined name. For name-first syntax (`name MACRO`) the first word is the name
        // and the second word is the keyword. For keyword-first (`MACRO name`) it is reversed.
        let src = self.source_lines.get(line_0).copied().unwrap_or("");
        let (content, comment) = Self::split_comment(src.trim());
        let macro_name = token.macro_definition_name();
        let first_word = content.split_ascii_whitespace().next().unwrap_or("");
        let first_upper = first_word.to_ascii_uppercase();
        let name_upper = macro_name.to_ascii_uppercase();
        let is_name_first = first_upper == name_upper || first_upper == format!("{}:", name_upper);
        let formatted = if is_name_first {
            // name MACRO[(params)]: case only the leading alpha chars of the keyword word,
            // leaving the macro name AND any parameter list unchanged.
            let name_end = content
                .find(|c: char| c.is_ascii_whitespace())
                .unwrap_or(content.len());
            let ws_and_rest = &content[name_end..];
            let rest_start = ws_and_rest
                .find(|c: char| !c.is_ascii_whitespace())
                .unwrap_or(ws_and_rest.len());
            let kw_and_tail = &ws_and_rest[rest_start..];
            let kw_end = kw_and_tail
                .find(|c: char| !c.is_ascii_alphabetic() && c != '_')
                .unwrap_or(kw_and_tail.len());
            format!(
                "{}{}{}{}",
                &content[..name_end],
                &ws_and_rest[..rest_start],
                Self::apply_case(&kw_and_tail[..kw_end], self.directive_case),
                &kw_and_tail[kw_end..],
            )
        }
        else {
            // MACRO name [...]: case the first word (the keyword) as usual.
            Self::apply_case_to_first_word(content, self.directive_case)
        };
        self.emit_line(depth, &formatted, comment);
        self.current_line = line_0 + 1;
        let body = token.macro_definition_code();
        // The body content captured by the parser ends just before the ENDM keyword.
        // For name-first macros (`name MACRO`) the newline of the header line is
        // included at the start of the body. For all macros the indentation prefix
        // of the ENDM line appears as a trailing whitespace-only fragment.
        // Strip both so current_line stays aligned with the ENDM source line.
        let mut lines: Vec<&str> = body.lines().collect();
        let first_content = lines
            .iter()
            .position(|l| !l.trim().is_empty())
            .unwrap_or(lines.len());
        lines.drain(..first_content);
        if lines.last().is_some_and(|l| l.trim().is_empty()) {
            lines.pop();
        }
        let line_count = lines.len();
        // `lines.join("\n")` alone does not round-trip a genuine trailing
        // blank line (`lines` ending in `""`) back through a later `.lines()`
        // call - `["a", ""].join("\n")` is `"a\n"`, and `.lines()` on *that*
        // yields just `["a"]` again, silently losing the blank. One more
        // trailing `\n` than `join` alone produces fixes it for every case
        // (a body with no trailing blank is unaffected: `.lines()` never
        // creates a phantom empty element from a lone final newline) and is
        // exactly what surfaced this: reformatting a macro body twice used
        // to quietly drop a blank line sitting right before `ENDM`.
        let trimmed_body = format!("{}\n", lines.join("\n"));

        // A macro body is captured as raw text (it is genuinely re-parsed fresh
        // on every call, with parameter substitution, so it was never part of
        // the single token tree the rest of this formatter walks) - but that
        // text is still *ordinary Z80 source with an occasional `{param}`
        // placeholder*, and `{param}` is real, already-recognised syntax
        // wherever an expression can appear, not something that needs an
        // actual call to resolve. Confirmed against two real projects: ~97%
        // of their macro bodies parse completely fine standalone. So: try the
        // real parser first, and only fall back to a verbatim copy - the only
        // thing possible before this - for the rare body whose placeholder use
        // genuinely isn't valid syntax on its own (e.g. a `{reg}` standing in
        // for a whole register operand rather than a value).
        let formatted_body = (!trimmed_body.trim().is_empty())
            .then(|| cpclib_asm::parser::parse_z80_str(&trimmed_body).ok())
            .flatten()
            .map(|body_listing| super::format_listing(&body_listing, &trimmed_body, depth + 1, &self.opt));

        match formatted_body {
            Some(formatted) => self.output.push_str(&formatted),
            None => {
                for line in &lines {
                    self.output.push_str(line);
                    self.output.push('\n');
                }
            }
        }
        self.current_line += line_count;

        self.emit_closer(depth, &["ENDM", "ENDMACRO", "MEND"]);
    }

    fn format_switch(&mut self, token: &LocatedToken, depth: usize, line_0: usize) {
        if line_0 >= self.current_line {
            self.emit_source_line_indented(depth, line_0);
            self.current_line = line_0 + 1;
        }

        // Collect cases so we can inspect them without consuming the iterator twice.
        let switch_cases: Vec<_> = token.switch_cases().collect();

        for (_, case_inner, has_break) in &switch_cases {
            let case_line = self.find_closer_start(&["CASE"]);
            // If no CASE line exists past current position (e.g. entirely inline switch), stop.
            if case_line >= self.source_lines.len() {
                break;
            }
            self.emit_interstitial(case_line);
            self.emit_source_line_indented(depth, case_line);
            self.current_line = case_line + 1;

            // If the first body token is on the same source line as the CASE header, the
            // entire case clause is inline (e.g. `case 1: db 1 : break`) and was already
            // emitted by emit_source_line_indented above.
            let body_inline = case_inner
                .first()
                .map(|t| t.span().relative_line_and_column().0.saturating_sub(1) <= case_line)
                .unwrap_or(false);
            if !body_inline {
                self.format_tokens(case_inner, depth + 1);
                if *has_break {
                    self.emit_closer(depth + 1, &["BREAK"]);
                }
            }
        }

        if let Some(default_inner) = token.switch_default() {
            let default_line = self.find_closer_start(&["DEFAULT"]);
            if default_line < self.source_lines.len() {
                self.emit_interstitial(default_line);
                self.emit_source_line_indented(depth, default_line);
                self.current_line = default_line + 1;
                let body_inline = default_inner
                    .first()
                    .map(|t| {
                        t.span().relative_line_and_column().0.saturating_sub(1) <= default_line
                    })
                    .unwrap_or(false);
                if !body_inline {
                    self.format_tokens(default_inner, depth + 1);
                }
            }
        }

        let endswitch_line = self.find_closer_start(&["ENDSWITCH", "ENDS"]);
        if endswitch_line < self.source_lines.len() {
            self.emit_closer(depth, &["ENDSWITCH", "ENDS"]);
        }
    }
}
