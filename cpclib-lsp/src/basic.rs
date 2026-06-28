use std::collections::HashMap;

use cpclib_basic::BasicProgram;
use tower_lsp::lsp_types::*;

use crate::document::Document;

pub struct BasicAnalyzer;

impl BasicAnalyzer {
    pub fn new() -> Self { Self }

    pub fn analyze(&self, document: &Document) -> Vec<Diagnostic> {
        match BasicProgram::parse(document.text()) {
            Ok(_) => vec![],
            Err(e) => vec![Diagnostic {
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end:   Position { line: 0, character: 1 },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                message: e.to_string(),
                source: Some("cpclib-lsp".into()),
                ..Default::default()
            }],
        }
    }

    pub fn document_symbols(&self, document: &Document) -> Vec<DocumentSymbol> {
        // Return first-assignment location for every variable found in the program.
        let mut seen: HashMap<String, u32> = HashMap::new();
        for (line_idx, line) in document.text().lines().enumerate() {
            let rest = strip_line_number(line);
            for var in extract_assigned_vars(rest) {
                let key = var.to_uppercase();
                seen.entry(key).or_insert(line_idx as u32);
            }
        }

        let mut entries: Vec<(String, u32)> = seen.into_iter().collect();
        entries.sort_by_key(|(_, ln)| *ln);

        entries
            .into_iter()
            .map(|(name, line_idx)| {
                let end_char = name.len() as u32;
                let pos = Range {
                    start: Position { line: line_idx, character: 0 },
                    end:   Position { line: line_idx, character: end_char },
                };
                #[allow(deprecated)]
                DocumentSymbol {
                    name,
                    detail: None,
                    kind: SymbolKind::VARIABLE,
                    tags: None,
                    deprecated: None,
                    range: pos,
                    selection_range: pos,
                    children: None,
                }
            })
            .collect()
    }

    pub fn goto_definition(&self, document: &Document, position: Position) -> Option<Location> {
        let source_line = document.line(position.line as usize)?;
        let col = position.character as usize;

        // Word under cursor must be a decimal integer.
        let word = digit_word_at(source_line.trim_end_matches(|c| c == '\n' || c == '\r'), col)?;
        let target_num: u16 = word.parse().ok()?;

        // Verify the number is a GOTO/GOSUB/RESTORE/RESUME/RUN target on this line.
        if !is_line_number_target(&source_line, col) {
            return None;
        }

        // Use cpclib-basic to parse the program and find the target line index.
        let prog = BasicProgram::parse(document.text()).ok()?;
        let (target_idx, _) = prog
            .lines()
            .iter()
            .enumerate()
            .find(|(_, l)| l.line_number() == target_num)?;

        let tgt = target_idx as u32;
        Some(Location {
            uri: document.uri.clone(),
            range: Range {
                start: Position { line: tgt, character: 0 },
                end:   Position { line: tgt, character: 0 },
            },
        })
    }

    pub fn hover(&self, document: &Document, position: Position) -> Option<Hover> {
        let source_line = document.line(position.line as usize)?;
        let col = position.character as usize;
        let line = source_line.trim_end_matches(|c| c == '\n' || c == '\r');

        // Try longest match first so e.g. "INKEY$" beats "INKEY".
        let word_upper = alpha_word_at(line, col)?.to_uppercase();
        let doc = KEYWORD_DOCS
            .iter()
            .find(|(kw, _)| kw.to_uppercase() == word_upper)
            .map(|(_, d)| *d)?;

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: doc.to_string(),
            }),
            range: None,
        })
    }

    pub fn completion(&self, _document: &Document, _position: Position) -> Vec<CompletionItem> {
        KEYWORD_DOCS
            .iter()
            .map(|(kw, doc)| CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: doc.to_string(),
                })),
                ..Default::default()
            })
            .collect()
    }

    pub fn find_references(&self, _document: &Document, _position: Position) -> Vec<Location> {
        vec![]
    }

    pub fn semantic_tokens(&self, _document: &Document) -> Vec<SemanticToken> {
        vec![]
    }

    pub fn code_lens(&self, _document: &Document) -> Vec<CodeLens> {
        vec![]
    }
}

// ── Text helpers ──────────────────────────────────────────────────────────────

/// Strip the BASIC line-number prefix (`"100 "` → `"PRINT …"`).
fn strip_line_number(line: &str) -> &str {
    let s = line.trim_start_matches(|c: char| c.is_ascii_digit());
    s.trim_start_matches(' ')
}

/// Return the sequence of ASCII digits that spans column `col` in `line`.
fn digit_word_at(line: &str, col: usize) -> Option<&str> {
    let bytes = line.as_bytes();
    if col >= bytes.len() || !bytes[col].is_ascii_digit() {
        return None;
    }
    let start = (0..col).rev().take_while(|&i| bytes[i].is_ascii_digit()).last().unwrap_or(col);
    let end   = (col..bytes.len()).take_while(|&i| bytes[i].is_ascii_digit()).last().unwrap_or(col) + 1;
    Some(&line[start..end])
}

/// Return the alphabetic/alphanumeric word (including `$` and `%` suffixes) at column `col`.
fn alpha_word_at(line: &str, col: usize) -> Option<String> {
    let bytes = line.as_bytes();
    if col >= bytes.len() {
        return None;
    }
    let is_word_char = |b: u8| b.is_ascii_alphanumeric() || b == b'$' || b == b'%' || b == b'_';
    if !is_word_char(bytes[col]) {
        return None;
    }
    let start = (0..col).rev().take_while(|&i| bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_').last().unwrap_or(col);
    let mut end = col;
    while end < bytes.len() && is_word_char(bytes[end]) {
        end += 1;
    }
    Some(line[start..end].to_string())
}

/// True when column `col` (which holds a digit) is a line-number argument of
/// GOTO / GOSUB / ON…GOTO / ON…GOSUB / RESTORE / RESUME / RUN.
fn is_line_number_target(line: &str, col: usize) -> bool {
    // Walk backwards from col, skip digits and spaces, then check for keyword.
    let bytes = line.as_bytes();
    let mut i = col;
    // skip current number and any leading spaces
    while i > 0 && (bytes[i - 1].is_ascii_digit() || bytes[i - 1] == b' ') {
        i -= 1;
    }
    // skip commas in ON…GOTO lists
    if i > 0 && bytes[i - 1] == b',' {
        // could be ON n GOTO x,y – just check for GOTO/GOSUB somewhere earlier
    }
    let prefix = line[..i].trim_end().to_uppercase();
    prefix.ends_with("GOTO")
        || prefix.ends_with("GOSUB")
        || prefix.ends_with("RESTORE")
        || prefix.ends_with("RESUME")
        || prefix.ends_with("RUN")
        || prefix.ends_with("MERGE")
        || prefix.ends_with("CHAIN")
        || prefix.ends_with("DELETE")
        || prefix.ends_with("LIST")
        || prefix.ends_with("RENUM")
        || prefix.ends_with("AUTO")
}

// ── Variable assignment extraction ───────────────────────────────────────────

/// Walk through the statement-level text of a BASIC line (after stripping the
/// line number) and return variable names that are being assigned.
fn extract_assigned_vars(line: &str) -> Vec<String> {
    let mut results = Vec::new();
    for stmt in split_statements(line) {
        let s = stmt.trim();
        if let Some(rest) = prefix_ci(s, "FOR") {
            if let Some(var) = var_before_eq(rest.trim_start()) {
                results.push(var);
            }
        } else if let Some(rest) = prefix_ci(s, "INPUT") {
            results.extend(vars_after_prompt(rest.trim_start()));
        } else if let Some(rest) = prefix_ci(s, "LINE INPUT") {
            results.extend(vars_after_prompt(rest.trim_start()));
        } else if let Some(rest) = prefix_ci(s, "READ") {
            results.extend(comma_separated_vars(rest.trim_start()));
        } else {
            // LET (optional) var = …
            let s2 = prefix_ci(s, "LET").map(|r| r.trim_start()).unwrap_or(s);
            if let Some(var) = var_before_eq(s2) {
                results.push(var);
            }
        }
    }
    results
}

/// Split a BASIC statement-block at `:` separators, respecting string literals.
fn split_statements(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut in_str = false;
    for (i, ch) in s.char_indices() {
        match ch {
            '"' => in_str = !in_str,
            ':' if !in_str => {
                parts.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&s[start..]);
    parts
}

/// If `s` starts with `prefix` (case-insensitive, whole-word), return the remainder.
fn prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() < prefix.len() {
        return None;
    }
    let candidate = &s[..prefix.len()];
    if !candidate.eq_ignore_ascii_case(prefix) {
        return None;
    }
    // Must be followed by a space, `(`, `=`, or end-of-string.
    let rest = &s[prefix.len()..];
    if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t') {
        Some(rest)
    } else {
        None
    }
}

/// Extract a variable name immediately before a `=` sign (assignment).
fn var_before_eq(s: &str) -> Option<String> {
    // var name: letter followed by letters/digits, then optional `$` or `%`,
    // then optional array index `(…)`, then optional spaces then `=`.
    let s = s.trim_start();
    let var = read_var_name(s)?;
    let after = s[var.len()..].trim_start();
    // Skip optional array subscript.
    let after = if after.starts_with('(') {
        skip_parens(after)
    } else {
        after
    };
    if after.trim_start().starts_with('=') { Some(var) } else { None }
}

/// Extract variable name(s) from an INPUT argument list (skip optional prompt).
fn vars_after_prompt(s: &str) -> Vec<String> {
    // Optional stream: `#n,`
    let s = skip_stream_prefix(s);
    // Optional prompt ending with `;` or `,`
    let s = if s.starts_with('"') {
        let end = s[1..].find('"').map(|i| i + 2).unwrap_or(s.len());
        let rest = &s[end..].trim_start();
        if rest.starts_with(';') || rest.starts_with(',') { &rest[1..] } else { rest }
    } else {
        s
    };
    comma_separated_vars(s.trim_start())
}

/// Extract comma-separated variable names.
fn comma_separated_vars(s: &str) -> Vec<String> {
    let mut vars = Vec::new();
    let mut rest = s.trim_start();
    loop {
        if let Some(var) = read_var_name(rest) {
            let after = rest[var.len()..].trim_start();
            let after = if after.starts_with('(') { skip_parens(after) } else { after };
            vars.push(var);
            rest = after.trim_start();
            if rest.starts_with(',') {
                rest = rest[1..].trim_start();
            } else {
                break;
            }
        } else {
            break;
        }
    }
    vars
}

/// Read a variable name from the start of `s`.  Returns `None` if `s` doesn't
/// start with a valid variable name.
fn read_var_name(s: &str) -> Option<String> {
    let mut chars = s.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    let mut name = String::from(first);
    for ch in chars {
        if ch.is_ascii_alphanumeric() {
            name.push(ch);
        } else if ch == '$' || ch == '%' {
            name.push(ch);
            break;
        } else {
            break;
        }
    }
    Some(name)
}

fn skip_parens(s: &str) -> &str {
    let mut depth = 0usize;
    let mut in_str = false;
    for (i, ch) in s.char_indices() {
        match ch {
            '"' => in_str = !in_str,
            '(' if !in_str => depth += 1,
            ')' if !in_str => {
                depth -= 1;
                if depth == 0 {
                    return &s[i + 1..];
                }
            }
            _ => {}
        }
    }
    s
}

fn skip_stream_prefix(s: &str) -> &str {
    if !s.starts_with('#') {
        return s;
    }
    let rest = s[1..].trim_start_matches(|c: char| c.is_ascii_digit());
    if rest.starts_with(',') { &rest[1..] } else { s }
}

// ── Keyword documentation ─────────────────────────────────────────────────────

pub static KEYWORD_DOCS: &[(&str, &str)] = &[
    // ── Control flow ──────────────────────────────────────────────────────────
    ("GOTO",    "**GOTO** *line*\n\nUnconditional jump to *line*."),
    ("GOSUB",   "**GOSUB** *line*\n\nCall subroutine at *line*. Execution continues after the matching `RETURN`."),
    ("RETURN",  "**RETURN**\n\nReturn from a `GOSUB` subroutine to the statement after the matching `GOSUB`."),
    ("IF",      "**IF** *expr* **THEN** *statements* [**ELSE** *statements*]\n\nConditional execution. If *expr* is non-zero (true) the THEN branch is taken; otherwise the optional ELSE branch."),
    ("THEN",    "**THEN** — keyword used after `IF` to introduce the true branch."),
    ("ELSE",    "**ELSE** — optional branch of an `IF` statement, taken when the condition is false."),
    ("FOR",     "**FOR** *var* = *start* **TO** *end* [**STEP** *inc*]\n\nBegin a counted loop. *var* runs from *start* to *end*, incrementing by *inc* (default 1) each iteration."),
    ("TO",      "**TO** — separator between start and end values in a `FOR` statement."),
    ("STEP",    "**STEP** *n* — optional increment for a `FOR` loop (default 1, may be negative)."),
    ("NEXT",    "**NEXT** [*var*]\n\nEnd of a `FOR` loop. Increments *var* and repeats if the limit has not been reached."),
    ("WHILE",   "**WHILE** *expr*\n\nBegin a condition-tested loop. Repeats as long as *expr* is non-zero."),
    ("WEND",    "**WEND**\n\nEnd of a `WHILE` loop."),
    ("ON",      "**ON** *expr* **GOTO**/**GOSUB** *line*[,*line*,...]\n\nComputed jump or call. Jumps/calls to the *n*-th line in the list where *n* = *expr*."),
    ("ON BREAK",    "**ON BREAK** **CONT**/**STOP**/**GOSUB** *line*\n\nSet the break-key handler."),
    ("ON ERROR GOTO","**ON ERROR GOTO** *line*\n\nInstall an error handler at *line*. Use `RESUME` to return."),
    ("RESUME",  "**RESUME** [*line*]\n\nResume execution after an error handler. Without *line*, resumes at the statement that caused the error; with *line*, resumes at that line."),
    ("ERROR",   "**ERROR** *n*\n\nSimulate error number *n*."),
    ("STOP",    "**STOP**\n\nStop program execution (can be resumed with `CONT`)."),
    ("END",     "**END**\n\nEnd program execution."),
    ("CONT",    "**CONT**\n\nContinue execution after `STOP`."),
    ("RUN",     "**RUN** [*line* | *filename$*]\n\nRun the program from the specified line (default: first line), or load and run a file."),
    // ── Assignment / data ─────────────────────────────────────────────────────
    ("LET",     "**LET** *var* = *expr*\n\nAssign *expr* to *var*. `LET` is optional in Locomotive BASIC."),
    ("DATA",    "**DATA** *value*[,*value*,...]\n\nDefine constant data values read by `READ`."),
    ("READ",    "**READ** *var*[,*var*,...]\n\nRead the next value(s) from the `DATA` list into *var*."),
    ("RESTORE", "**RESTORE** [*line*]\n\nReset the `DATA` pointer to *line* (or to the first `DATA` statement if omitted)."),
    ("DIM",     "**DIM** *var*(*size*[,*size*,...])[,*var*...]\n\nDimension an array. Indices are 0-based by default."),
    ("ERASE",   "**ERASE** *var*[,*var*,...]\n\nFree the memory used by an array."),
    ("SWAP",    "**SWAP** *var1*, *var2*\n\nExchange the values of two variables of the same type."),
    ("MID$",    "**MID$**(*string$*, *start*[, *len*]) = *str$*\n\nReplace a substring within *string$* with *str$*."),
    // ── I/O ───────────────────────────────────────────────────────────────────
    ("PRINT",   "**PRINT** [**#***stream*,] [*expr*[;|,] ...]\n\nPrint expressions to the screen (or *stream*). `;` suppresses the trailing newline; `,` moves to the next print zone."),
    ("INPUT",   "**INPUT** [**#***stream*,] [*prompt$*;] *var*[,*var*,...]\n\nRead user input into *var*(s). An optional prompt is displayed before waiting."),
    ("LINE INPUT","**LINE INPUT** [**#***stream*,] [*prompt$*;] *var$*\n\nRead an entire line of input (including spaces) into *var$*."),
    ("WRITE",   "**WRITE** [**#***stream*,] *expr*[,*expr*,...]\n\nWrite values in comma-delimited format (strings quoted). Useful for machine-readable output."),
    ("OPENIN",  "**OPENIN** *filename$*\n\nOpen a file on tape/disc for sequential reading."),
    ("OPENOUT", "**OPENOUT** *filename$*\n\nOpen a file on tape/disc for sequential writing."),
    ("CLOSEIN", "**CLOSEIN**\n\nClose the currently open input file."),
    ("CLOSEOUT","**CLOSEOUT**\n\nClose the currently open output file."),
    ("OUT",     "**OUT** *port*, *value*\n\nWrite *value* to hardware I/O port *port*."),
    ("KEY",     "**KEY** *n*, *string$*\n\nAssign *string$* to function key *n* (0–9 for f0–f9, 128–138 for shifted)."),
    ("LOCATE",  "**LOCATE** [**#***stream*,] *col*, *row*\n\nPosition the text cursor at column *col*, row *row* (1-based)."),
    ("TAB",     "**TAB**(*n*)\n\nMove print position to column *n* in a `PRINT` statement."),
    ("SPC",     "**SPC**(*n*)\n\nInsert *n* spaces in a `PRINT` statement."),
    ("AT",      "**AT**(*row*, *col*) — position specifier for `PRINT`."),
    ("USING",   "**USING** *format$*; *expr*[,...] — formatted numeric output in `PRINT`."),
    ("WIDTH",   "**WIDTH** **#***stream*, *n*\n\nSet the line width of *stream* to *n* characters."),
    ("ZONE",    "**ZONE** *n*\n\nSet the print zone width (used by `,` separator in `PRINT`)."),
    ("WINDOW",  "**WINDOW** [**#***stream*,] *left*, *right*, *top*, *bottom*\n\nDefine a text window (viewport) on the screen."),
    // ── Screen / graphics ─────────────────────────────────────────────────────
    ("MODE",    "**MODE** *n*\n\nSet screen mode: 0 = 160×200 16-colour, 1 = 320×200 4-colour, 2 = 640×200 2-colour."),
    ("CLS",     "**CLS** [**#***stream*]\n\nClear the screen or text window of *stream*."),
    ("CLG",     "**CLG** [*ink*]\n\nClear the graphics screen to *ink* colour (default: current graphics paper)."),
    ("INK",     "**INK** *pen*, *color1*[, *color2*]\n\nSet ink pen *pen* to *color1* (and optionally flash between *color1* and *color2*). Colours 0–26."),
    ("PAPER",   "**PAPER** [**#***stream*,] *color*\n\nSet the background (paper) colour for text output."),
    ("PEN",     "**PEN** [**#***stream*,] *color*[, *bg*]\n\nSet the foreground (pen) colour for text output."),
    ("BORDER",  "**BORDER** *color1*[, *color2*]\n\nSet the screen border colour (with optional flash)."),
    ("PLOT",    "**PLOT** *x*, *y*[, *ink*]\n\nPlot a pixel at absolute graphics coordinates (*x*, *y*)."),
    ("PLOTR",   "**PLOTR** *dx*, *dy*[, *ink*]\n\nPlot a pixel at coordinates relative to the current graphics position."),
    ("DRAW",    "**DRAW** *x*, *y*[, *ink*]\n\nDraw a line from the current graphics position to (*x*, *y*)."),
    ("DRAWR",   "**DRAWR** *dx*, *dy*[, *ink*]\n\nDraw a line relative to the current graphics position."),
    ("MOVE",    "**MOVE** *x*, *y*[, *ink*[, *paper*]]\n\nMove the graphics cursor to (*x*, *y*) without drawing."),
    ("MOVER",   "**MOVER** *dx*, *dy*[, *ink*[, *paper*]]\n\nMove the graphics cursor relative to the current position without drawing."),
    ("ORIGIN",  "**ORIGIN** *x*, *y*[, *xmin*, *xmax*, *ymin*, *ymax*]\n\nSet the graphics origin and optional clipping rectangle."),
    ("FILL",    "**FILL** [*ink*,] *x*, *y*\n\nFlood-fill the enclosed area containing (*x*, *y*) with *ink*."),
    ("MASK",    "**MASK** [*int*[, *first*]]\n\nSet the graphics plotting mask for sprite effects."),
    ("GRAPHICS","**GRAPHICS** **PAPER** *color* / **GRAPHICS** **PEN** [*color*]\n\nSet the graphics paper or pen colour used by CLG/DRAW etc."),
    ("TAG",     "**TAG** [**#***stream*]\n\nEnable graphics-mode text output (characters follow the graphics cursor)."),
    ("TAGOFF",  "**TAGOFF** [**#***stream*]\n\nDisable graphics-mode text output."),
    ("CURSOR",  "**CURSOR** [*visibility*[, *blink*]]\n\nControl the text cursor visibility and blink rate."),
    ("SYMBOL",  "**SYMBOL** *n*, *row0*[, *row1*, ..., *row7*]\n\nRedefine character code *n* with 8-byte pixel pattern."),
    ("SYMBOL AFTER", "**SYMBOL AFTER** *n*\n\nSet the first character code (*n*) that can be redefined with `SYMBOL`."),
    // ── Sound ─────────────────────────────────────────────────────────────────
    ("SOUND",   "**SOUND** *channel*, *period*[, *duration*[, *volume*[, *vol_env*[, *tone_env*[, *noise*]]]]]\n\nPlay a sound. *channel* is a bitmask (1=A, 2=B, 4=C). *period* controls pitch (0–4095)."),
    ("NOISE",   "(part of `SOUND`) — noise period parameter."),
    ("ENV",     "**ENV** *n*, *steps*, *step_size*, *period*[,...]\n\nDefine volume envelope *n* for use with `SOUND`."),
    ("ENT",     "**ENT** *n*, *steps*, *step_size*, *period*[,...]\n\nDefine tone envelope *n* for use with `SOUND`."),
    ("RELEASE", "**RELEASE** *channel*\n\nRelease the sound queue for *channel* (1=A, 2=B, 4=C)."),
    // ── Timing / interrupts ───────────────────────────────────────────────────
    ("AFTER",   "**AFTER** *n* [**,***timer*] **GOSUB** *line*\n\nSchedule a one-shot interrupt after *n* centiseconds. Timers 0–3 are available."),
    ("EVERY",   "**EVERY** *n* [**,***timer*] **GOSUB** *line*\n\nSchedule a repeating interrupt every *n* centiseconds. Timers 0–3 are available."),
    ("WAIT",    "**WAIT** *n*\n\nPause execution for *n* centiseconds."),
    ("FRAME",   "**FRAME**\n\nWait for the next 50 Hz screen frame sync (≈20 ms)."),
    // ── Memory / system ───────────────────────────────────────────────────────
    ("POKE",    "**POKE** *address*, *value*\n\nWrite *value* (0–255) to memory *address*."),
    ("CALL",    "**CALL** *address*[, *param*, ...]\n\nCall a machine-code subroutine at *address*. Parameters are passed on the stack."),
    ("MEMORY",  "**MEMORY** *address*\n\nSet the top of user memory to *address*, protecting machine code/data above it."),
    ("DI",      "**DI**\n\nDisable hardware interrupts (Z80 DI instruction)."),
    ("EI",      "**EI**\n\nEnable hardware interrupts (Z80 EI instruction)."),
    // ── Program management ────────────────────────────────────────────────────
    ("NEW",     "**NEW**\n\nErase the current program and all variables."),
    ("LIST",    "**LIST** [*start*[-*end*]]\n\nList program lines to the screen."),
    ("AUTO",    "**AUTO** [*start*[, *step*]]\n\nEnable automatic line numbering. Default start 10, step 10."),
    ("DELETE",  "**DELETE** *line*[-*line*]\n\nDelete a range of program lines."),
    ("EDIT",    "**EDIT** *line*\n\nOpen *line* in the built-in editor."),
    ("RENUM",   "**RENUM** [*new*[, *old*[, *step*]]]\n\nRenumber program lines."),
    ("REM",     "**REM** *comment* — remark; the rest of the line is ignored."),
    ("TRON",    "**TRON**\n\nTrace on: display line numbers as they are executed."),
    ("TROFF",   "**TROFF**\n\nTrace off: disable line-number tracing."),
    // ── Tape / disc ───────────────────────────────────────────────────────────
    ("LOAD",    "**LOAD** *filename$*[, *address*]\n\nLoad a BASIC program (or binary data to *address*) from tape/disc."),
    ("SAVE",    "**SAVE** *filename$*[, **B**, *address*, *length*[, *entry*]]\n\nSave the program (or a block of memory) to tape/disc."),
    ("MERGE",   "**MERGE** *filename$*\n\nMerge a BASIC program from tape/disc into the current program."),
    ("CHAIN",   "**CHAIN** *filename$*[, *line*]\n\nLoad and run a BASIC program, optionally starting at *line*."),
    ("CAT",     "**CAT**\n\nDisplay the disc directory (catalogue)."),
    // ── Definitions ───────────────────────────────────────────────────────────
    ("DEF",     "**DEF FN** *name*(*args*) = *expr*\n\nDefine a user function callable as `FN name(args)`."),
    ("FN",      "**FN** *name*(*args*)\n\nCall a user-defined function (defined with `DEF FN`)."),
    ("DEFINT",  "**DEFINT** *letter*[-*letter*]\n\nDeclare that all variables starting with the given letter(s) are integers."),
    ("DEFREAL", "**DEFREAL** *letter*[-*letter*]\n\nDeclare that all variables starting with the given letter(s) are real numbers."),
    ("DEFSTR",  "**DEFSTR** *letter*[-*letter*]\n\nDeclare that all variables starting with the given letter(s) are strings."),
    ("DEG",     "**DEG**\n\nSet angle mode to degrees for trigonometric functions."),
    ("RAD",     "**RAD**\n\nSet angle mode to radians for trigonometric functions (default)."),
    ("RANDOMIZE","**RANDOMIZE** [*n*]\n\nSeed the random number generator. Without *n*, uses the real-time clock."),
    ("SPEED",   "**SPEED** **INK** *n1*, *n2* / **SPEED** **KEY** *n1*, *n2* / **SPEED** **WRITE** *n*\n\nSet ink flash speed, keyboard auto-repeat speed, or tape write speed."),
    // ── Built-in functions ────────────────────────────────────────────────────
    ("ABS",     "**ABS**(*n*) → number\n\nAbsolute value of *n*."),
    ("ASC",     "**ASC**(*a$*) → integer\n\nASCII code of the first character of *a$*."),
    ("ATN",     "**ATN**(*n*) → number\n\nArctangent of *n* (result in current angle mode)."),
    ("BIN$",    "**BIN$**(*n*[, *digits*]) → string\n\nConvert integer *n* to binary string, zero-padded to *digits*."),
    ("CHR$",    "**CHR$**(*n*) → string\n\nCharacter whose ASCII code is *n*."),
    ("CINT",    "**CINT**(*n*) → integer\n\nRound *n* to the nearest integer (returns a real value)."),
    ("COPYCHR$","**COPYCHR$**(**#***stream*) → string\n\nRead the character at the current cursor position on *stream*."),
    ("COS",     "**COS**(*n*) → number\n\nCosine of *n* (in current angle mode)."),
    ("CREAL",   "**CREAL**(*n*) → real\n\nConvert *n* to a real (floating-point) number."),
    ("DEC$",    "**DEC$**(*n*, *format$*) → string\n\nFormat *n* as a decimal string using *format$* (e.g. `\"###.##\"`)."),
    ("DERR",    "**DERR** → integer\n\nDisc error number from the most recent disc operation."),
    ("EOF",     "**EOF** → integer\n\n-1 if the input file is at end-of-file, 0 otherwise."),
    ("ERR",     "**ERR** → integer\n\nError number of the last error encountered."),
    ("ERL",     "**ERL** → integer\n\nLine number where the last error occurred."),
    ("EXP",     "**EXP**(*n*) → number\n\n*e* raised to the power *n* (≈ 2.71828ⁿ)."),
    ("FIX",     "**FIX**(*n*) → integer\n\nTruncate *n* toward zero (remove fractional part)."),
    ("FRE",     "**FRE**(*n* | *var$*) → integer\n\nFree memory in bytes. Pass any number or string argument."),
    ("HEX$",    "**HEX$**(*n*[, *digits*]) → string\n\nConvert integer *n* to hexadecimal string, zero-padded to *digits*."),
    ("HIMEM",   "**HIMEM** → integer\n\nHighest address of user memory (set with `MEMORY`)."),
    ("INKEY",   "**INKEY**(*n*) → integer\n\nReturn -1 if key *n* is currently pressed, 0 if not. Non-blocking."),
    ("INKEY$",  "**INKEY$** → string\n\nRead one character from the keyboard buffer; returns `\"\"` if no key is waiting."),
    ("INP",     "**INP**(*port*) → integer\n\nRead a byte from hardware I/O port *port*."),
    ("INSTR",   "**INSTR**([*start*,] *string$*, *search$*) → integer\n\nPosition of *search$* within *string$* (1-based), starting at *start* (default 1). Returns 0 if not found."),
    ("INT",     "**INT**(*n*) → integer\n\nLargest integer ≤ *n* (floor function)."),
    ("JOY",     "**JOY**(*n*) → integer\n\nRead joystick *n* (0 = right port, 1 = left port). Returns a bitmask: bit 0=up, 1=down, 2=left, 3=right, 4=fire 1, 5=fire 2."),
    ("LEFT$",   "**LEFT$**(*string$*, *n*) → string\n\nLeftmost *n* characters of *string$*."),
    ("LEN",     "**LEN**(*string$*) → integer\n\nLength of *string$* in characters."),
    ("LOG",     "**LOG**(*n*) → number\n\nNatural logarithm (base *e*) of *n*."),
    ("LOG10",   "**LOG10**(*n*) → number\n\nBase-10 logarithm of *n*."),
    ("LOWER$",  "**LOWER$**(*string$*) → string\n\nConvert *string$* to lower case."),
    ("MAX",     "**MAX**(*n1*, *n2*[, ...]) → number\n\nLargest of the given values."),
    ("MID$",    "**MID$**(*string$*, *start*[, *len*]) → string\n\nSubstring of *string$* starting at *start* (1-based) with length *len* (or to end)."),
    ("MIN",     "**MIN**(*n1*, *n2*[, ...]) → number\n\nSmallest of the given values."),
    ("PEEK",    "**PEEK**(*address*) → integer\n\nRead the byte at memory *address*."),
    ("PI",      "**PI** → number\n\nThe constant π (3.14159265…)."),
    ("POS",     "**POS**(**#***stream*) → integer\n\nHorizontal cursor column (1-based) on *stream*."),
    ("REMAIN",  "**REMAIN**(*timer*) → integer\n\nCancel the `AFTER`/`EVERY` *timer* (0–3) and return the remaining centiseconds."),
    ("RIGHT$",  "**RIGHT$**(*string$*, *n*) → string\n\nRightmost *n* characters of *string$*."),
    ("RND",     "**RND**[(*n*)] → number\n\nRandom number. If *n* < 0: re-seed and return; *n* = 0: repeat last; *n* > 0 or omitted: return new value in [0, 1)."),
    ("ROUND",   "**ROUND**(*n*[, *decimals*]) → number\n\nRound *n* to *decimals* decimal places (default 0)."),
    ("SIGN",    "**SIGN**(*n*) → integer\n\nSign of *n*: -1 if negative, 0 if zero, 1 if positive."),
    ("SIN",     "**SIN**(*n*) → number\n\nSine of *n* (in current angle mode)."),
    ("SPACE$",  "**SPACE$**(*n*) → string\n\nString of *n* space characters."),
    ("SQ",      "**SQ**(*channel*) → integer\n\nSound queue status for *channel* (1=A, 2=B, 4=C). Returns number of entries free."),
    ("SQR",     "**SQR**(*n*) → number\n\nSquare root of *n*."),
    ("STR$",    "**STR$**(*n*) → string\n\nConvert number *n* to its string representation."),
    ("STRING$", "**STRING$**(*n*, *char$* | *code*) → string\n\nString of *n* repetitions of a character (given as a one-character string or ASCII code)."),
    ("TAN",     "**TAN**(*n*) → number\n\nTangent of *n* (in current angle mode)."),
    ("TEST",    "**TEST**(*x*, *y*) → integer\n\nColour (ink number) of the pixel at absolute graphics coordinates (*x*, *y*)."),
    ("TESTSTR", "**TESTSTR**(*dx*, *dy*) → integer\n\nColour of the pixel at graphics coordinates relative to the current position."),
    ("TIME",    "**TIME** → integer\n\nElapsed time in centiseconds since power-on (wraps at 2 147 483 647)."),
    ("UNT",     "**UNT**(*n*) → integer\n\nInterpret a 16-bit two's-complement value as an unsigned integer."),
    ("UPPER$",  "**UPPER$**(*string$*) → string\n\nConvert *string$* to upper case."),
    ("VAL",     "**VAL**(*string$*) → number\n\nConvert the numeric string *string$* to a number. Returns 0 if not a valid number."),
    ("VPOS",    "**VPOS**(**#***stream*) → integer\n\nVertical cursor row (1-based) on *stream*."),
    ("XPOS",    "**XPOS** → integer\n\nCurrent graphics cursor X coordinate."),
    ("YPOS",    "**YPOS** → integer\n\nCurrent graphics cursor Y coordinate."),
    // ── Operators (for hover when cursor is on them) ───────────────────────────
    ("AND",     "**AND** — Bitwise/logical AND operator. Also used to combine `SOUND` channel masks."),
    ("OR",      "**OR** — Bitwise/logical OR operator."),
    ("XOR",     "**XOR** — Bitwise exclusive-OR operator."),
    ("NOT",     "**NOT** *expr* — Bitwise/logical NOT (one's complement)."),
    ("MOD",     "*a* **MOD** *b* — Integer modulo: remainder of integer division of *a* by *b*."),
];
