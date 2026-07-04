use super::Formatter;

impl<'src> Formatter<'src> {
    pub(super) fn indent(&self, depth: usize) -> String {
        " ".repeat(depth * self.indent_size)
    }

    pub(super) fn emit_interstitial(&mut self, target_line: usize) {
        while self.current_line < target_line {
            let src = self.source_lines.get(self.current_line).copied().unwrap_or("");
            let trimmed = src.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with("//") {
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
