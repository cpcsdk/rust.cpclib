use super::Formatter;

impl<'src> Formatter<'src> {
    pub(super) fn indent(&self, depth: usize) -> String {
        " ".repeat(depth * self.indent_size)
    }

    pub(super) fn emit_interstitial(&mut self, target_line: usize) {
        // Counts a run of consecutive blank lines within *this* call only -
        // real content (code or a comment) resets it, and so does returning
        // from this function, so `max_consecutive_blank_lines` only ever
        // collapses blank lines it can see clustered together in one walk.
        // That covers the common case (several blank lines in a row between
        // two statements) without needing to track a blank run as formatter
        // state across the many separate call sites that reach this.
        let mut blank_run = 0usize;
        while self.current_line < target_line {
            let src = self
                .source_lines
                .get(self.current_line)
                .copied()
                .unwrap_or("");
            let trimmed = src.trim();
            if trimmed.is_empty() {
                blank_run += 1;
                let keep = self
                    .max_consecutive_blank_lines
                    .is_none_or(|max| blank_run <= max);
                if keep {
                    self.output.push_str(src);
                    self.output.push('\n');
                }
            }
            else if trimmed.starts_with(';') {
                blank_run = 0;
                self.output.push_str(src);
                self.output.push('\n');
            }
            else {
                blank_run = 0;
            }
            self.current_line += 1;
        }
    }

    // Copy source lines through completely unchanged, from wherever we left
    // off up to and including `end_line_0` - the mechanism behind `; fmt: off`
    // .. `; fmt: on` (see `pragma`'s own doc comment). A no-op if we're
    // already past `end_line_0` (an earlier token inside the same disabled
    // range already covered it).
    pub(super) fn emit_verbatim_through(&mut self, end_line_0: usize) {
        while self.current_line <= end_line_0 {
            if let Some(src) = self.source_lines.get(self.current_line) {
                self.output.push_str(src);
                self.output.push('\n');
            }
            self.current_line += 1;
        }
    }

    pub(super) fn emit_line(&mut self, depth: usize, content: &str, comment: Option<&str>) {
        let indent = self.indent(depth);
        self.output.push_str(&indent);
        self.output.push_str(content);
        if let Some(c) = comment {
            let current_col = indent.len() + content.len();
            let padding = self.comment_column.saturating_sub(current_col).max(2);
            self.output.push_str(&" ".repeat(padding));
            self.output.push_str(c);
        }
        self.output.push('\n');
    }

    // Emit a source line with reformatted indentation and directive_case on the first word.
    // Used for block headers and closers (REPEAT … ENDREPEAT, IF … ENDIF, etc.)
    pub(super) fn emit_source_line_indented(&mut self, depth: usize, line_0: usize) {
        let src = self.source_lines.get(line_0).copied().unwrap_or("");
        let (content, comment) = Self::split_comment(src.trim());
        let formatted = Self::apply_case_to_first_word(content, self.directive_case);
        let formatted = self.reformat_numeric_literals(&formatted);
        let formatted = self.apply_comma_and_quote_style(&formatted);
        self.emit_line(depth, &formatted, comment);
    }

    pub(super) fn find_closer_start(&self, keywords: &[&str]) -> usize {
        let kws: Vec<String> = keywords.iter().map(|k| k.to_ascii_uppercase()).collect();
        for i in self.current_line..self.source_lines.len() {
            let t = self.source_lines[i].trim().to_ascii_uppercase();
            for kw in &kws {
                if t == kw.as_str()
                    || t.starts_with(&format!("{kw} "))
                    || t.starts_with(&format!("{kw}\t"))
                    || t.starts_with(&format!("{kw};"))
                    || t.starts_with(&format!("{kw}//"))
                {
                    return i;
                }
            }
        }
        self.current_line
    }

    pub(super) fn emit_closer(&mut self, depth: usize, keywords: &[&str]) {
        let line = self.find_closer_start(keywords);
        self.emit_interstitial(line);
        self.emit_source_line_indented(depth, line);
        self.current_line = line + 1;
    }
}
