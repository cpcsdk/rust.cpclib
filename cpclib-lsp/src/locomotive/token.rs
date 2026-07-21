//! Lexical data and helpers for Locomotive BASIC: keyword documentation
//! table, semantic-token indices, and small token-stream scanners shared by
//! the feature modules.

use std::collections::HashMap;

use cpclib_basic::located::{LocatedBasicProgram, LocatedBasicToken, LocatedTokenKind};
use cpclib_basic::tokens::BasicTokenNoPrefix;
use tower_lsp::lsp_types::Position;

use super::diagnostics::{collect_comma_separated_vars, collect_vars_after_input, record_var};

// ─── Semantic-token type indices (must match the basm module legend) ─────────
pub(super) const TT_KEYWORD: u32 = 0;
pub(super) const TT_FUNCTION: u32 = 2;
pub(super) const TT_VARIABLE: u32 = 4;
pub(super) const TT_NUMBER: u32 = 5;
pub(super) const TT_STRING: u32 = 6;
pub(super) const TT_COMMENT: u32 = 7;
pub(super) const TT_OPERATOR: u32 = 8;

// ─── Token navigation helpers ─────────────────────────────────────────────────

/// Skip space tokens starting at index `from` and return the first non-space
/// token if it is a Variable.
pub(super) fn skip_spaces_then_var(
    toks: &[LocatedBasicToken],
    from: usize
) -> Option<&LocatedBasicToken> {
    let mut i = from;
    while i < toks.len() {
        match &toks[i].kind {
            LocatedTokenKind::Space => {
                i += 1;
            },
            LocatedTokenKind::Variable(_) => return Some(&toks[i]),
            _ => return None
        }
    }
    None
}

/// Skip spaces starting at `from` and return the number text if a Number token follows.
pub(super) fn skip_spaces_then_number<'a>(
    toks: &'a [LocatedBasicToken],
    from: usize
) -> Option<&'a str> {
    let mut i = from;
    while i < toks.len() {
        match &toks[i].kind {
            LocatedTokenKind::Space => {
                i += 1;
            },
            LocatedTokenKind::Number(n) => return Some(n.as_str()),
            _ => return None
        }
    }
    None
}

/// Skip spaces starting at `from` and return the variable name if a Variable token follows.
pub(super) fn skip_spaces_then_var_name(toks: &[LocatedBasicToken], from: usize) -> Option<String> {
    skip_spaces_then_var(toks, from).and_then(|t| {
        if let LocatedTokenKind::Variable(n) = &t.kind {
            Some(n.clone())
        }
        else {
            None
        }
    })
}

/// Returns true if, starting at index `from` (skipping spaces), the first
/// meaningful token is `=`.
pub(super) fn is_followed_by_eq(toks: &[LocatedBasicToken], from: usize) -> bool {
    let mut i = from;
    while i < toks.len() {
        match &toks[i].kind {
            LocatedTokenKind::Space => {
                i += 1;
            },
            LocatedTokenKind::Other('(') => {
                // Skip subscript `(…)` then check for `=`.
                i += 1;
                let mut depth = 1usize;
                while i < toks.len() && depth > 0 {
                    match &toks[i].kind {
                        LocatedTokenKind::Other('(') => {
                            depth += 1;
                            i += 1;
                        },
                        LocatedTokenKind::Other(')') => {
                            depth -= 1;
                            i += 1;
                        },
                        _ => {
                            i += 1;
                        }
                    }
                }
                // After closing paren, look for `=`.
            },
            LocatedTokenKind::Operator(BasicTokenNoPrefix::Equal) => return true,
            _ => return false
        }
    }
    false
}

/// The token at `position` (document/block-relative, whichever `prog` was
/// parsed against), if any. Shared by `definition.rs` (rename/find-
/// references) and `hover.rs` (line/token byte hover) - previously
/// duplicated inline in each.
pub(super) fn token_at_position(
    prog: &LocatedBasicProgram,
    position: Position
) -> Option<&LocatedBasicToken> {
    let bline = prog.lines.iter().find(|l| l.source_line == position.line)?;
    bline
        .tokens
        .iter()
        .find(|t| t.span.col <= position.character && position.character < t.span.col + t.span.len)
}

/// The token's own encoded byte(s), plus a short display label, for kinds
/// whose encoding is an unambiguous, direct function of the token itself:
/// keywords, prefixed ("additional") functions, operators, and the
/// structural space/statement-separator/passthrough-character kinds. Shared
/// by both hover entry points (`hover.rs`) so a hovered token — whether it's
/// a keyword or just a space or a `:` — shows the same bytes as its own row
/// in the line-level byte breakdown.
///
/// `Variable`/`Number`/`StringLit`/`Comment`/`LineNumber` need fuller
/// multi-byte encoders (or already have their own dedicated hover) and are
/// out of scope here.
pub(super) fn token_bytes(kind: &LocatedTokenKind) -> Option<(String, Vec<u8>)> {
    match kind {
        LocatedTokenKind::Keyword(k) => Some((k.to_string(), vec![k.value()])),
        LocatedTokenKind::Function(f) => {
            Some((
                f.to_string(),
                vec![BasicTokenNoPrefix::AdditionalTokenMarker.value(), f.value()]
            ))
        },
        LocatedTokenKind::Operator(op) => {
            Some((op.to_string().trim().to_string(), vec![op.value()]))
        },
        LocatedTokenKind::Space => {
            Some((
                "(space)".to_string(),
                vec![BasicTokenNoPrefix::CharSpace.value()]
            ))
        },
        LocatedTokenKind::Separator => {
            Some((":".to_string(), vec![BasicTokenNoPrefix::CharColon.value()]))
        },
        LocatedTokenKind::Other(c) => {
            let mut buf = [0u8; 4];
            Some((c.to_string(), c.encode_utf8(&mut buf).as_bytes().to_vec()))
        },
        _ => None
    }
}

// ─── Variable collection ───────────────────────────────────────────────────────

/// Scan every line of `prog` for variable "definition" occurrences — the
/// same notion of "assignment context" used by the outline: `LET`/`FOR`
/// targets, `INPUT`/`READ` targets, and bare `NAME = ...` assignments.
/// Returns each variable keyed by its uppercased name, first-occurrence line
/// and column preserved for callers that need a location (the outline);
/// completion only needs the names.
pub(super) fn collect_variable_occurrences(
    prog: &LocatedBasicProgram
) -> HashMap<String, (String, u32, u32)> {
    let mut seen: HashMap<String, (String, u32, u32)> = HashMap::new();

    for bline in &prog.lines {
        let toks = &bline.tokens;
        let n = toks.len();
        let mut i = 0;

        while i < n {
            let tok = &toks[i];
            match &tok.kind {
                LocatedTokenKind::Keyword(BasicTokenNoPrefix::Let) => {
                    if let Some(var_tok) = skip_spaces_then_var(toks, i + 1) {
                        record_var(&mut seen, var_tok);
                    }
                    i += 1;
                },
                LocatedTokenKind::Keyword(BasicTokenNoPrefix::For) => {
                    if let Some(var_tok) = skip_spaces_then_var(toks, i + 1) {
                        record_var(&mut seen, var_tok);
                    }
                    i += 1;
                },
                LocatedTokenKind::Keyword(BasicTokenNoPrefix::Input) => {
                    collect_vars_after_input(toks, i + 1, &mut seen);
                    i += 1;
                },
                LocatedTokenKind::Keyword(BasicTokenNoPrefix::Read) => {
                    collect_comma_separated_vars(toks, i + 1, &mut seen);
                    i += 1;
                },
                LocatedTokenKind::Variable(name) => {
                    // Bare assignment: var followed by optional spaces then `=`.
                    if is_followed_by_eq(toks, i + 1) {
                        let key = name.to_uppercase();
                        seen.entry(key)
                            .or_insert_with(|| (name.clone(), tok.span.line, tok.span.col));
                    }
                    i += 1;
                },
                _ => {
                    i += 1;
                }
            }
        }
    }

    seen
}

// ─── Number helpers ───────────────────────────────────────────────────────────

/// Parse a BASIC integer literal to i64.
/// Handles `&XX` (hex), `&HXX` (explicit hex), `&X...` (binary), and decimal.
/// Returns None for floats or unrecognised forms.
pub(super) fn parse_basic_integer(text: &str) -> Option<i64> {
    if let Some(rest) = text.strip_prefix('&') {
        if let Some(bin_str) = rest.strip_prefix(['X', 'x']) {
            i64::from_str_radix(bin_str, 2).ok()
        }
        else if let Some(hex_str) = rest.strip_prefix(['H', 'h']) {
            i64::from_str_radix(hex_str, 16).ok()
        }
        else {
            i64::from_str_radix(rest, 16).ok()
        }
    }
    else {
        text.parse::<i64>().ok()
    }
}

// ─── Text helpers (hover still works on raw text) ─────────────────────────────

/// Return the alphabetic/alphanumeric word (including `$` and `%` suffixes) at column `col`.
pub(super) fn alpha_word_at(line: &str, col: usize) -> Option<String> {
    let bytes = line.as_bytes();
    if col >= bytes.len() {
        return None;
    }
    let is_word_char = |b: u8| b.is_ascii_alphanumeric() || b == b'$' || b == b'%' || b == b'_';
    if !is_word_char(bytes[col]) {
        return None;
    }
    let start = (0..col)
        .rev()
        .take_while(|&i| bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_')
        .last()
        .unwrap_or(col);
    let mut end = col;
    while end < bytes.len() && is_word_char(bytes[end]) {
        end += 1;
    }
    Some(line[start..end].to_string())
}

// ─── Keyword documentation ────────────────────────────────────────────────────

pub(super) static KEYWORD_DOCS: &[(&str, &str)] = &[
    // ── Control flow ──────────────────────────────────────────────────────────
    ("GOTO", "**GOTO** *line*\n\nUnconditional jump to *line*."),
    (
        "GOSUB",
        "**GOSUB** *line*\n\nCall subroutine at *line*. Execution continues after the matching `RETURN`."
    ),
    (
        "RETURN",
        "**RETURN**\n\nReturn from a `GOSUB` subroutine to the statement after the matching `GOSUB`."
    ),
    (
        "IF",
        "**IF** *expr* **THEN** *statements* [**ELSE** *statements*]\n\nConditional execution. If *expr* is non-zero (true) the THEN branch is taken; otherwise the optional ELSE branch."
    ),
    (
        "THEN",
        "**THEN** — keyword used after `IF` to introduce the true branch."
    ),
    (
        "ELSE",
        "**ELSE** — optional branch of an `IF` statement, taken when the condition is false."
    ),
    (
        "FOR",
        "**FOR** *var* = *start* **TO** *end* [**STEP** *inc*]\n\nBegin a counted loop. *var* runs from *start* to *end*, incrementing by *inc* (default 1) each iteration."
    ),
    (
        "TO",
        "**TO** — separator between start and end values in a `FOR` statement."
    ),
    (
        "STEP",
        "**STEP** *n* — optional increment for a `FOR` loop (default 1, may be negative)."
    ),
    (
        "NEXT",
        "**NEXT** [*var*]\n\nEnd of a `FOR` loop. Increments *var* and repeats if the limit has not been reached."
    ),
    (
        "WHILE",
        "**WHILE** *expr*\n\nBegin a condition-tested loop. Repeats as long as *expr* is non-zero."
    ),
    ("WEND", "**WEND**\n\nEnd of a `WHILE` loop."),
    (
        "ON",
        "**ON** *expr* **GOTO**/**GOSUB** *line*[,*line*,...]\n\nComputed jump or call. Jumps/calls to the *n*-th line in the list where *n* = *expr*."
    ),
    (
        "ON BREAK",
        "**ON BREAK** **CONT**/**STOP**/**GOSUB** *line*\n\nSet the break-key handler."
    ),
    (
        "ON ERROR GOTO",
        "**ON ERROR GOTO** *line*\n\nInstall an error handler at *line*. Use `RESUME` to return."
    ),
    (
        "RESUME",
        "**RESUME** [*line*]\n\nResume execution after an error handler. Without *line*, resumes at the statement that caused the error; with *line*, resumes at that line."
    ),
    ("ERROR", "**ERROR** *n*\n\nSimulate error number *n*."),
    (
        "STOP",
        "**STOP**\n\nStop program execution (can be resumed with `CONT`)."
    ),
    ("END", "**END**\n\nEnd program execution."),
    ("CONT", "**CONT**\n\nContinue execution after `STOP`."),
    (
        "RUN",
        "**RUN** [*line* | *filename$*]\n\nRun the program from the specified line (default: first line), or load and run a file."
    ),
    // ── Assignment / data ─────────────────────────────────────────────────────
    (
        "LET",
        "**LET** *var* = *expr*\n\nAssign *expr* to *var*. `LET` is optional in Locomotive BASIC."
    ),
    (
        "DATA",
        "**DATA** *value*[,*value*,...]\n\nDefine constant data values read by `READ`."
    ),
    (
        "READ",
        "**READ** *var*[,*var*,...]\n\nRead the next value(s) from the `DATA` list into *var*."
    ),
    (
        "RESTORE",
        "**RESTORE** [*line*]\n\nReset the `DATA` pointer to *line* (or to the first `DATA` statement if omitted)."
    ),
    (
        "DIM",
        "**DIM** *var*(*size*[,*size*,...])[,*var*...]\n\nDimension an array. Indices are 0-based by default."
    ),
    (
        "ERASE",
        "**ERASE** *var*[,*var*,...]\n\nFree the memory used by an array."
    ),
    (
        "SWAP",
        "**SWAP** *var1*, *var2*\n\nExchange the values of two variables of the same type."
    ),
    (
        "MID$",
        "**MID$**(*string$*, *start*[, *len*]) = *str$*\n\nReplace a substring within *string$* with *str$*."
    ),
    // ── I/O ───────────────────────────────────────────────────────────────────
    (
        "PRINT",
        "**PRINT** [**#***stream*,] [*expr*[;|,] ...]\n\nPrint expressions to the screen (or *stream*). `;` suppresses the trailing newline; `,` moves to the next print zone."
    ),
    (
        "INPUT",
        "**INPUT** [**#***stream*,] [*prompt$*;] *var*[,*var*,...]\n\nRead user input into *var*(s). An optional prompt is displayed before waiting."
    ),
    (
        "LINE INPUT",
        "**LINE INPUT** [**#***stream*,] [*prompt$*;] *var$*\n\nRead an entire line of input (including spaces) into *var$*."
    ),
    (
        "WRITE",
        "**WRITE** [**#***stream*,] *expr*[,*expr*,...]\n\nWrite values in comma-delimited format (strings quoted). Useful for machine-readable output."
    ),
    (
        "OPENIN",
        "**OPENIN** *filename$*\n\nOpen a file on tape/disc for sequential reading."
    ),
    (
        "OPENOUT",
        "**OPENOUT** *filename$*\n\nOpen a file on tape/disc for sequential writing."
    ),
    (
        "CLOSEIN",
        "**CLOSEIN**\n\nClose the currently open input file."
    ),
    (
        "CLOSEOUT",
        "**CLOSEOUT**\n\nClose the currently open output file."
    ),
    (
        "OUT",
        "**OUT** *port*, *value*\n\nWrite *value* to hardware I/O port *port*."
    ),
    (
        "KEY",
        "**KEY** *n*, *string$*\n\nAssign *string$* to function key *n* (0–9 for f0–f9, 128–138 for shifted)."
    ),
    (
        "LOCATE",
        "**LOCATE** [**#***stream*,] *col*, *row*\n\nPosition the text cursor at column *col*, row *row* (1-based)."
    ),
    (
        "TAB",
        "**TAB**(*n*)\n\nMove print position to column *n* in a `PRINT` statement."
    ),
    (
        "SPC",
        "**SPC**(*n*)\n\nInsert *n* spaces in a `PRINT` statement."
    ),
    (
        "AT",
        "**AT**(*row*, *col*) — position specifier for `PRINT`."
    ),
    (
        "USING",
        "**USING** *format$*; *expr*[,...] — formatted numeric output in `PRINT`."
    ),
    (
        "WIDTH",
        "**WIDTH** **#***stream*, *n*\n\nSet the line width of *stream* to *n* characters."
    ),
    (
        "ZONE",
        "**ZONE** *n*\n\nSet the print zone width (used by `,` separator in `PRINT`)."
    ),
    (
        "WINDOW",
        "**WINDOW** [**#***stream*,] *left*, *right*, *top*, *bottom*\n\nDefine a text window (viewport) on the screen."
    ),
    // ── Screen / graphics ─────────────────────────────────────────────────────
    (
        "MODE",
        "**MODE** *n*\n\nSet screen mode: 0 = 160×200 16-colour, 1 = 320×200 4-colour, 2 = 640×200 2-colour."
    ),
    (
        "CLS",
        "**CLS** [**#***stream*]\n\nClear the screen or text window of *stream*."
    ),
    (
        "CLG",
        "**CLG** [*ink*]\n\nClear the graphics screen to *ink* colour (default: current graphics paper)."
    ),
    (
        "INK",
        "**INK** *pen*, *color1*[, *color2*]\n\nSet ink pen *pen* to *color1* (and optionally flash between *color1* and *color2*). Colours 0–26."
    ),
    (
        "PAPER",
        "**PAPER** [**#***stream*,] *color*\n\nSet the background (paper) colour for text output."
    ),
    (
        "PEN",
        "**PEN** [**#***stream*,] *color*[, *bg*]\n\nSet the foreground (pen) colour for text output."
    ),
    (
        "BORDER",
        "**BORDER** *color1*[, *color2*]\n\nSet the screen border colour (with optional flash)."
    ),
    (
        "PLOT",
        "**PLOT** *x*, *y*[, *ink*]\n\nPlot a pixel at absolute graphics coordinates (*x*, *y*)."
    ),
    (
        "PLOTR",
        "**PLOTR** *dx*, *dy*[, *ink*]\n\nPlot a pixel at coordinates relative to the current graphics position."
    ),
    (
        "DRAW",
        "**DRAW** *x*, *y*[, *ink*]\n\nDraw a line from the current graphics position to (*x*, *y*)."
    ),
    (
        "DRAWR",
        "**DRAWR** *dx*, *dy*[, *ink*]\n\nDraw a line relative to the current graphics position."
    ),
    (
        "MOVE",
        "**MOVE** *x*, *y*[, *ink*[, *paper*]]\n\nMove the graphics cursor to (*x*, *y*) without drawing."
    ),
    (
        "MOVER",
        "**MOVER** *dx*, *dy*[, *ink*[, *paper*]]\n\nMove the graphics cursor relative to the current position without drawing."
    ),
    (
        "ORIGIN",
        "**ORIGIN** *x*, *y*[, *xmin*, *xmax*, *ymin*, *ymax*]\n\nSet the graphics origin and optional clipping rectangle."
    ),
    (
        "FILL",
        "**FILL** [*ink*,] *x*, *y*\n\nFlood-fill the enclosed area containing (*x*, *y*) with *ink*."
    ),
    (
        "MASK",
        "**MASK** [*int*[, *first*]]\n\nSet the graphics plotting mask for sprite effects."
    ),
    (
        "GRAPHICS",
        "**GRAPHICS** **PAPER** *color* / **GRAPHICS** **PEN** [*color*]\n\nSet the graphics paper or pen colour used by CLG/DRAW etc."
    ),
    (
        "TAG",
        "**TAG** [**#***stream*]\n\nEnable graphics-mode text output (characters follow the graphics cursor)."
    ),
    (
        "TAGOFF",
        "**TAGOFF** [**#***stream*]\n\nDisable graphics-mode text output."
    ),
    (
        "CURSOR",
        "**CURSOR** [*visibility*[, *blink*]]\n\nControl the text cursor visibility and blink rate."
    ),
    (
        "SYMBOL",
        "**SYMBOL** *n*, *row0*[, *row1*, ..., *row7*]\n\nRedefine character code *n* with 8-byte pixel pattern."
    ),
    (
        "SYMBOL AFTER",
        "**SYMBOL AFTER** *n*\n\nSet the first character code (*n*) that can be redefined with `SYMBOL`."
    ),
    // ── Sound ─────────────────────────────────────────────────────────────────
    (
        "SOUND",
        "**SOUND** *channel*, *period*[, *duration*[, *volume*[, *vol_env*[, *tone_env*[, *noise*]]]]]\n\nPlay a sound. *channel* is a bitmask (1=A, 2=B, 4=C). *period* controls pitch (0–4095)."
    ),
    ("NOISE", "(part of `SOUND`) — noise period parameter."),
    (
        "ENV",
        "**ENV** *n*, *steps*, *step_size*, *period*[,...]\n\nDefine volume envelope *n* for use with `SOUND`."
    ),
    (
        "ENT",
        "**ENT** *n*, *steps*, *step_size*, *period*[,...]\n\nDefine tone envelope *n* for use with `SOUND`."
    ),
    (
        "RELEASE",
        "**RELEASE** *channel*\n\nRelease the sound queue for *channel* (1=A, 2=B, 4=C)."
    ),
    // ── Timing / interrupts ───────────────────────────────────────────────────
    (
        "AFTER",
        "**AFTER** *n* [**,***timer*] **GOSUB** *line*\n\nSchedule a one-shot interrupt after *n* centiseconds. Timers 0–3 are available."
    ),
    (
        "EVERY",
        "**EVERY** *n* [**,***timer*] **GOSUB** *line*\n\nSchedule a repeating interrupt every *n* centiseconds. Timers 0–3 are available."
    ),
    (
        "WAIT",
        "**WAIT** *n*\n\nPause execution for *n* centiseconds."
    ),
    (
        "FRAME",
        "**FRAME**\n\nWait for the next 50 Hz screen frame sync (≈20 ms)."
    ),
    // ── Memory / system ───────────────────────────────────────────────────────
    (
        "POKE",
        "**POKE** *address*, *value*\n\nWrite *value* (0–255) to memory *address*."
    ),
    (
        "CALL",
        "**CALL** *address*[, *param*, ...]\n\nCall a machine-code subroutine at *address*. Parameters are passed on the stack."
    ),
    (
        "MEMORY",
        "**MEMORY** *address*\n\nSet the top of user memory to *address*, protecting machine code/data above it."
    ),
    (
        "DI",
        "**DI**\n\nDisable hardware interrupts (Z80 DI instruction)."
    ),
    (
        "EI",
        "**EI**\n\nEnable hardware interrupts (Z80 EI instruction)."
    ),
    // ── Program management ────────────────────────────────────────────────────
    (
        "NEW",
        "**NEW**\n\nErase the current program and all variables."
    ),
    (
        "LIST",
        "**LIST** [*start*[-*end*]]\n\nList program lines to the screen."
    ),
    (
        "AUTO",
        "**AUTO** [*start*[, *step*]]\n\nEnable automatic line numbering. Default start 10, step 10."
    ),
    (
        "DELETE",
        "**DELETE** *line*[-*line*]\n\nDelete a range of program lines."
    ),
    (
        "EDIT",
        "**EDIT** *line*\n\nOpen *line* in the built-in editor."
    ),
    (
        "RENUM",
        "**RENUM** [*new*[, *old*[, *step*]]]\n\nRenumber program lines."
    ),
    (
        "REM",
        "**REM** *comment* — remark; the rest of the line is ignored."
    ),
    (
        "TRON",
        "**TRON**\n\nTrace on: display line numbers as they are executed."
    ),
    (
        "TROFF",
        "**TROFF**\n\nTrace off: disable line-number tracing."
    ),
    // ── Tape / disc ───────────────────────────────────────────────────────────
    (
        "LOAD",
        "**LOAD** *filename$*[, *address*]\n\nLoad a BASIC program (or binary data to *address*) from tape/disc."
    ),
    (
        "SAVE",
        "**SAVE** *filename$*[, **B**, *address*, *length*[, *entry*]]\n\nSave the program (or a block of memory) to tape/disc."
    ),
    (
        "MERGE",
        "**MERGE** *filename$*\n\nMerge a BASIC program from tape/disc into the current program."
    ),
    (
        "CHAIN",
        "**CHAIN** *filename$*[, *line*]\n\nLoad and run a BASIC program, optionally starting at *line*."
    ),
    ("CAT", "**CAT**\n\nDisplay the disc directory (catalogue)."),
    // ── Definitions ───────────────────────────────────────────────────────────
    (
        "DEF",
        "**DEF FN** *name*(*args*) = *expr*\n\nDefine a user function callable as `FN name(args)`."
    ),
    (
        "FN",
        "**FN** *name*(*args*)\n\nCall a user-defined function (defined with `DEF FN`)."
    ),
    (
        "DEFINT",
        "**DEFINT** *letter*[-*letter*]\n\nDeclare that all variables starting with the given letter(s) are integers."
    ),
    (
        "DEFREAL",
        "**DEFREAL** *letter*[-*letter*]\n\nDeclare that all variables starting with the given letter(s) are real numbers."
    ),
    (
        "DEFSTR",
        "**DEFSTR** *letter*[-*letter*]\n\nDeclare that all variables starting with the given letter(s) are strings."
    ),
    (
        "DEG",
        "**DEG**\n\nSet angle mode to degrees for trigonometric functions."
    ),
    (
        "RAD",
        "**RAD**\n\nSet angle mode to radians for trigonometric functions (default)."
    ),
    (
        "RANDOMIZE",
        "**RANDOMIZE** [*n*]\n\nSeed the random number generator. Without *n*, uses the real-time clock."
    ),
    (
        "SPEED",
        "**SPEED** **INK** *n1*, *n2* / **SPEED** **KEY** *n1*, *n2* / **SPEED** **WRITE** *n*\n\nSet ink flash speed, keyboard auto-repeat speed, or tape write speed."
    ),
    // ── Built-in functions ────────────────────────────────────────────────────
    ("ABS", "**ABS**(*n*) → number\n\nAbsolute value of *n*."),
    (
        "ASC",
        "**ASC**(*a$*) → integer\n\nASCII code of the first character of *a$*."
    ),
    (
        "ATN",
        "**ATN**(*n*) → number\n\nArctangent of *n* (result in current angle mode)."
    ),
    (
        "BIN$",
        "**BIN$**(*n*[, *digits*]) → string\n\nConvert integer *n* to binary string, zero-padded to *digits*."
    ),
    (
        "CHR$",
        "**CHR$**(*n*) → string\n\nCharacter whose ASCII code is *n*."
    ),
    (
        "CINT",
        "**CINT**(*n*) → integer\n\nRound *n* to the nearest integer (returns a real value)."
    ),
    (
        "COPYCHR$",
        "**COPYCHR$**(**#***stream*) → string\n\nRead the character at the current cursor position on *stream*."
    ),
    (
        "COS",
        "**COS**(*n*) → number\n\nCosine of *n* (in current angle mode)."
    ),
    (
        "CREAL",
        "**CREAL**(*n*) → real\n\nConvert *n* to a real (floating-point) number."
    ),
    (
        "DEC$",
        "**DEC$**(*n*, *format$*) → string\n\nFormat *n* as a decimal string using *format$* (e.g. `\"###.##\"`)."
    ),
    (
        "DERR",
        "**DERR** → integer\n\nDisc error number from the most recent disc operation."
    ),
    (
        "EOF",
        "**EOF** → integer\n\n-1 if the input file is at end-of-file, 0 otherwise."
    ),
    (
        "ERR",
        "**ERR** → integer\n\nError number of the last error encountered."
    ),
    (
        "ERL",
        "**ERL** → integer\n\nLine number where the last error occurred."
    ),
    (
        "EXP",
        "**EXP**(*n*) → number\n\n*e* raised to the power *n* (≈ 2.71828ⁿ)."
    ),
    (
        "FIX",
        "**FIX**(*n*) → integer\n\nTruncate *n* toward zero (remove fractional part)."
    ),
    (
        "FRE",
        "**FRE**(*n* | *var$*) → integer\n\nFree memory in bytes. Pass any number or string argument."
    ),
    (
        "HEX$",
        "**HEX$**(*n*[, *digits*]) → string\n\nConvert integer *n* to hexadecimal string, zero-padded to *digits*."
    ),
    (
        "HIMEM",
        "**HIMEM** → integer\n\nHighest address of user memory (set with `MEMORY`)."
    ),
    (
        "INKEY",
        "**INKEY**(*n*) → integer\n\nReturn -1 if key *n* is currently pressed, 0 if not. Non-blocking."
    ),
    (
        "INKEY$",
        "**INKEY$** → string\n\nRead one character from the keyboard buffer; returns `\"\"` if no key is waiting."
    ),
    (
        "INP",
        "**INP**(*port*) → integer\n\nRead a byte from hardware I/O port *port*."
    ),
    (
        "INSTR",
        "**INSTR**([*start*,] *string$*, *search$*) → integer\n\nPosition of *search$* within *string$* (1-based), starting at *start* (default 1). Returns 0 if not found."
    ),
    (
        "INT",
        "**INT**(*n*) → integer\n\nLargest integer ≤ *n* (floor function)."
    ),
    (
        "JOY",
        "**JOY**(*n*) → integer\n\nRead joystick *n* (0 = right port, 1 = left port). Returns a bitmask: bit 0=up, 1=down, 2=left, 3=right, 4=fire 1, 5=fire 2."
    ),
    (
        "LEFT$",
        "**LEFT$**(*string$*, *n*) → string\n\nLeftmost *n* characters of *string$*."
    ),
    (
        "LEN",
        "**LEN**(*string$*) → integer\n\nLength of *string$* in characters."
    ),
    (
        "LOG",
        "**LOG**(*n*) → number\n\nNatural logarithm (base *e*) of *n*."
    ),
    (
        "LOG10",
        "**LOG10**(*n*) → number\n\nBase-10 logarithm of *n*."
    ),
    (
        "LOWER$",
        "**LOWER$**(*string$*) → string\n\nConvert *string$* to lower case."
    ),
    (
        "MAX",
        "**MAX**(*n1*, *n2*[, ...]) → number\n\nLargest of the given values."
    ),
    (
        "MID$",
        "**MID$**(*string$*, *start*[, *len*]) → string\n\nSubstring of *string$* starting at *start* (1-based) with length *len* (or to end)."
    ),
    (
        "MIN",
        "**MIN**(*n1*, *n2*[, ...]) → number\n\nSmallest of the given values."
    ),
    (
        "PEEK",
        "**PEEK**(*address*) → integer\n\nRead the byte at memory *address*."
    ),
    ("PI", "**PI** → number\n\nThe constant π (3.14159265…)."),
    (
        "POS",
        "**POS**(**#***stream*) → integer\n\nHorizontal cursor column (1-based) on *stream*."
    ),
    (
        "REMAIN",
        "**REMAIN**(*timer*) → integer\n\nCancel the `AFTER`/`EVERY` *timer* (0–3) and return the remaining centiseconds."
    ),
    (
        "RIGHT$",
        "**RIGHT$**(*string$*, *n*) → string\n\nRightmost *n* characters of *string$*."
    ),
    (
        "RND",
        "**RND**[(*n*)] → number\n\nRandom number. If *n* < 0: re-seed and return; *n* = 0: repeat last; *n* > 0 or omitted: return new value in [0, 1)."
    ),
    (
        "ROUND",
        "**ROUND**(*n*[, *decimals*]) → number\n\nRound *n* to *decimals* decimal places (default 0)."
    ),
    (
        "SIGN",
        "**SIGN**(*n*) → integer\n\nSign of *n*: -1 if negative, 0 if zero, 1 if positive."
    ),
    (
        "SIN",
        "**SIN**(*n*) → number\n\nSine of *n* (in current angle mode)."
    ),
    (
        "SPACE$",
        "**SPACE$**(*n*) → string\n\nString of *n* space characters."
    ),
    (
        "SQ",
        "**SQ**(*channel*) → integer\n\nSound queue status for *channel* (1=A, 2=B, 4=C). Returns number of entries free."
    ),
    ("SQR", "**SQR**(*n*) → number\n\nSquare root of *n*."),
    (
        "STR$",
        "**STR$**(*n*) → string\n\nConvert number *n* to its string representation."
    ),
    (
        "STRING$",
        "**STRING$**(*n*, *char$* | *code*) → string\n\nString of *n* repetitions of a character (given as a one-character string or ASCII code)."
    ),
    (
        "TAN",
        "**TAN**(*n*) → number\n\nTangent of *n* (in current angle mode)."
    ),
    (
        "TEST",
        "**TEST**(*x*, *y*) → integer\n\nColour (ink number) of the pixel at absolute graphics coordinates (*x*, *y*)."
    ),
    (
        "TESTSTR",
        "**TESTSTR**(*dx*, *dy*) → integer\n\nColour of the pixel at graphics coordinates relative to the current position."
    ),
    (
        "TIME",
        "**TIME** → integer\n\nElapsed time in centiseconds since power-on (wraps at 2 147 483 647)."
    ),
    (
        "UNT",
        "**UNT**(*n*) → integer\n\nInterpret a 16-bit two's-complement value as an unsigned integer."
    ),
    (
        "UPPER$",
        "**UPPER$**(*string$*) → string\n\nConvert *string$* to upper case."
    ),
    (
        "VAL",
        "**VAL**(*string$*) → number\n\nConvert the numeric string *string$* to a number. Returns 0 if not a valid number."
    ),
    (
        "VPOS",
        "**VPOS**(**#***stream*) → integer\n\nVertical cursor row (1-based) on *stream*."
    ),
    (
        "XPOS",
        "**XPOS** → integer\n\nCurrent graphics cursor X coordinate."
    ),
    (
        "YPOS",
        "**YPOS** → integer\n\nCurrent graphics cursor Y coordinate."
    ),
    // ── Operators (for hover when cursor is on them) ───────────────────────────
    (
        "AND",
        "**AND** — Bitwise/logical AND operator. Also used to combine `SOUND` channel masks."
    ),
    ("OR", "**OR** — Bitwise/logical OR operator."),
    ("XOR", "**XOR** — Bitwise exclusive-OR operator."),
    (
        "NOT",
        "**NOT** *expr* — Bitwise/logical NOT (one's complement)."
    ),
    (
        "MOD",
        "*a* **MOD** *b* — Integer modulo: remainder of integer division of *a* by *b*."
    )
];
