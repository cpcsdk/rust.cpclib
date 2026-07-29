//! Derives semantic tokens directly from the real parsed AST
//! (`LocatedListing`/`LocatedToken`/`LocatedExpr`/`LocatedDataAccess`)
//! instead of re-scanning raw text, for every category that has real,
//! reliable span data. Two categories genuinely have none anywhere in the
//! AST and are deliberately left unclaimed here, falling back to
//! `semantic_tokens.rs`'s existing byte-level scanner exactly as before:
//! block comments (`/* */`, discarded as pure whitespace during parsing —
//! never a real token) and macro-body `{param}` placeholder text (macro
//! bodies are stored as raw, unparsed text; `MacroSegment::Arg` records only
//! a parameter index, no span).
//!
//! Every function here is a pure tree->iterator transformation - nothing
//! threads a `&mut Vec` accumulator through the recursion. This mirrors
//! `token.rs`'s own `flatten_listing`/`flatten_one` idiom
//! (`Box<dyn Iterator<...>>`, boxed because each match arm returns a
//! differently-shaped concrete iterator type). Only the single outermost
//! entry point collects into an owned `Vec`.
//!
//! Deliberately conservative wherever a span's exact boundaries weren't
//! independently confirmed against real parser source: under-claiming a
//! category (leaving it to the old scanner) is always safe, since the old
//! scanner is still the byte-level fallback for every column this walker
//! doesn't claim - over-claiming (e.g. assuming a span's shape without
//! checking) is what would risk a real regression. See the doc comments on
//! `expr_tokens`/`operand_tokens` for the specific cases this ruled out.

use std::collections::HashMap;

use cpclib_asm::parser::obtained::{
    LocatedDataAccess, LocatedExpr, LocatedListing, LocatedToken, MayHaveSpan
};
use cpclib_asm::parser::source::Z80Span;
use cpclib_asm::preamble::SourceString;
use cpclib_tokens::ListingElement;

use super::token::{
    DIRECTIVE_SET, MOD_DECLARATION, MOD_READONLY, RawSemanticToken, TT_ENUM_MEMBER, TT_FUNCTION,
    TT_KEYWORD, TT_LABEL, TT_MACRO, TT_NAMESPACE, TT_NUMBER, TT_VARIABLE, flatten_listing,
    locate_name_in_statement
};

type Tokens<'a> = Box<dyn Iterator<Item = RawSemanticToken> + 'a>;

/// Entry point: every semantic token derivable from `listing`'s real spans,
/// in no particular order (the caller sorts alongside the rest of its own
/// shared accumulator).
pub(super) fn ast_semantic_tokens(listing: &LocatedListing) -> Vec<RawSemanticToken> {
    flatten_listing(listing.iter())
        .flat_map(statement_tokens)
        .collect()
}

/// Groups `raw`'s entries by line, as `(start_col, end_col)` ranges, for
/// `semantic_tokens.rs`'s own byte-level scanner to skip over.
pub(super) fn claimed_ranges_by_line(raw: &[RawSemanticToken]) -> HashMap<u32, Vec<(u32, u32)>> {
    let mut by_line: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
    for t in raw {
        by_line
            .entry(t.line)
            .or_default()
            .push((t.col, t.col + t.len));
    }
    by_line
}

fn statement_tokens(token: &LocatedToken) -> Tokens<'_> {
    // `any_delegate!`-generated accessors (mnemonic(), equ_symbol(), ...)
    // unconditionally `.left().unwrap()` and panic on a `WarningWrapper`
    // (e.g. the `LD HL, DE` fake-instruction shorthand). Skip AST
    // involvement entirely for such a statement - the old scanner already
    // colors its words correctly via plain INSTRUCTION_SET/REGISTER_SET
    // lookups, nothing is lost.
    if token.is_warning() {
        return Box::new(std::iter::empty());
    }

    if token.mnemonic().is_some() {
        return opcode_tokens(token);
    }
    if token.is_label() {
        return Box::new(
            named_token(token, token.label_symbol(), TT_LABEL, MOD_DECLARATION).into_iter()
        );
    }
    if token.is_equ() {
        return Box::new(
            named_token(token, token.equ_symbol(), TT_ENUM_MEMBER, MOD_READONLY)
                .into_iter()
                .chain(expr_tokens(token.equ_value()))
        );
    }
    if token.is_assign() {
        return Box::new(
            named_token(token, token.assign_symbol(), TT_ENUM_MEMBER, MOD_READONLY)
                .into_iter()
                .chain(expr_tokens(token.assign_value()))
        );
    }
    if token.is_macro_definition() {
        // Params/body deliberately unclaimed - stays on the old scanner
        // (macro bodies are raw, unparsed text; see module doc comment).
        return Box::new(
            keyword_token(token) // "MACRO"
                .into_iter()
                .chain(named_token(
                    token,
                    token.macro_definition_name(),
                    TT_FUNCTION,
                    MOD_DECLARATION
                ))
        );
    }
    if token.is_call_macro_or_build_struct() {
        // Call arguments deliberately unclaimed for this pass.
        return Box::new(named_token(token, token.macro_call_name(), TT_FUNCTION, 0).into_iter());
    }
    if token.is_module() {
        // Nested body is visited separately via flatten_listing's own recursion.
        return Box::new(
            keyword_token(token) // "MODULE"
                .into_iter()
                .chain(named_token(
                    token,
                    token.module_name(),
                    TT_NAMESPACE,
                    MOD_DECLARATION
                ))
        );
    }
    if token.is_comment() {
        return Box::new(std::iter::empty()); // `;`/`//` already 100% correct on the old scanner
    }

    // Generic fallback for every other directive kind (ORG/DB/DW/DEFS/IF/
    // REPEAT/ENDM/RORG/...): claim only the leading keyword.
    Box::new(keyword_token(token).into_iter())
}

/// Mnemonic width: always just the statement's own leading identifier run,
/// never bounded by "distance to the first operand's span". That
/// operand-span-subtraction approach was tried and found genuinely unsound:
/// several instructions have (or, in `CP`'s case, *had* - see below) an
/// optional prefix the parser discarded before `mnemonic_arg1()`'s span even
/// started - originally confirmed for `CP`, where `CP A, C`'s `arg1` used to
/// be the compared value "C" directly (span starting well after "CP "), so
/// subtracting would have silently swallowed "CP A, " as if it were all one
/// keyword token (caught by the differential test against `good_cp.rasm`,
/// not by the unit tests). `CP` itself is now fixed at the source
/// (`cpclib-asm`'s `parse_cp` was rewritten to match `ADD`/`ADC`'s own
/// established two-slot shape: `arg1` is the optional explicit `A,` prefix,
/// `arg2` is the mandatory compared value - the same "optional implicit-
/// accumulator prefix" shape already fixed once before for `ADD r`/`ADC r`
/// itself, see the Wave 1 register-tracking fix in `cpclib-z80emu`). This
/// walker deliberately keeps the always-leading-word-run approach anyway
/// rather than reverting to span-subtraction now that `CP` is fixed: it's
/// simpler, produces an identical result in every case, and protects
/// against any *other*, not-yet-found mnemonic with a similar quirk.
/// (`SUB`/`AND`/`OR`/`XOR` had the identical discarded-prefix bug and are
/// now also fixed at the source, the same way `CP` was.)
fn opcode_tokens(token: &LocatedToken) -> Tokens<'_> {
    let stmt_span = token.span();
    let stmt_text = stmt_span.as_str();
    let (line1, col1) = stmt_span.relative_line_and_column();
    let (line, col0) = (
        line1.saturating_sub(1) as u32,
        col1.saturating_sub(1) as u32
    );

    let first_line = stmt_text.lines().next().unwrap_or(stmt_text);
    let mnemonic_len: usize = first_line
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .map(|c| c.len_utf8())
        .sum();
    let mnemonic = (mnemonic_len > 0).then(|| {
        RawSemanticToken {
            line,
            col: col0,
            len: mnemonic_len as u32,
            token_type: TT_KEYWORD,
            modifiers: 0
        }
    });
    let arg1 = token.mnemonic_arg1();
    // The 4th tuple field of OpCode (an implicit Option<Register8> used by
    // a few undocumented-instruction encodings) has no span at all - left
    // unclaimed, old scanner's REGISTER_SET lookup already colors it.
    Box::new(
        mnemonic
            .into_iter()
            .chain(operand_tokens(arg1))
            .chain(operand_tokens(token.mnemonic_arg2()))
    )
}

/// Operands, matched on the concrete `LocatedDataAccess` enum directly (14
/// variants, verified exhaustively against real source).
fn operand_tokens(access: Option<&LocatedDataAccess>) -> Tokens<'_> {
    let Some(access) = access
    else {
        return Box::new(std::iter::empty());
    };
    match access {
        LocatedDataAccess::Register8(_, span)
        | LocatedDataAccess::Register16(_, span)
        | LocatedDataAccess::IndexRegister8(_, span)
        | LocatedDataAccess::IndexRegister16(_, span)
        | LocatedDataAccess::FlagTest(_, span)
        | LocatedDataAccess::SpecialRegisterI(span)
        | LocatedDataAccess::SpecialRegisterR(span) => {
            Box::new(span_token(span, TT_VARIABLE, 0).into_iter())
        },
        // "IX+5"-shaped: outer span covers register+op+expr combined. Only
        // recurse into the offset expression - "IX" itself stays unclaimed
        // (old scanner's REGISTER_SET already colors it identically).
        LocatedDataAccess::IndexRegister16WithIndex(_, _, expr, _) => expr_tokens(expr),
        // `(HL)`/`(IX)`/`(C)`-shaped: verified directly (`parse_hl_address`/
        // `parse_indexregister_address`/`parse_portc` in cpclib-asm) that
        // the span *deliberately* covers the WHOLE parenthesized/bracketed
        // addressing-mode form, parens included - this is intentional
        // design (the variant represents the whole addressing mode, not
        // just the register), not a parser defect to fix upstream, unlike
        // the `FlagTest` trailing-whitespace case (see `span_token`'s doc
        // comment) which genuinely was a parser bug and got fixed at the
        // source instead. Claiming the whole span as one TT_VARIABLE token
        // here would incorrectly recolor `(`/`)` (old scanner colors those
        // as TT_OPERATOR) - deliberately left unclaimed rather than
        // guessing where inside the span the register substring starts/ends.
        LocatedDataAccess::MemoryRegister16(..)
        | LocatedDataAccess::MemoryIndexRegister16(..)
        | LocatedDataAccess::PortC(_) => Box::new(std::iter::empty()),
        LocatedDataAccess::PortN(expr, _)
        | LocatedDataAccess::Expression(expr)
        | LocatedDataAccess::Memory(expr) => expr_tokens(expr)
    }
}

/// Expressions, matched on the concrete `LocatedExpr` enum directly (13
/// variants, verified exhaustively against real source). Spans are read via
/// the whole expression's own `MayHaveSpan::span()` (not by attempting to
/// destructure a variant's inner span field directly) since at least one
/// variant (`String(UnescapedString)`) stores its span behind a
/// crate-private field unreachable from here - `span()` handles that
/// destructuring from inside `cpclib-asm` itself.
fn expr_tokens(expr: &LocatedExpr) -> Tokens<'_> {
    match expr {
        LocatedExpr::Value(..) | LocatedExpr::Float(..) => {
            Box::new(span_token(expr.span(), TT_NUMBER, 0).into_iter())
        },
        // String/Char literal spans were verified directly (`parse_string`
        // in cpclib-asm) to cover only the inner content, EXCLUDING both
        // surrounding quote characters (the opening quote is consumed
        // before the `.with_taken()` capture starts, the closing one after
        // it ends). The old scanner's own string handling colors the
        // quotes too, so claiming just the inner span here would visually
        // split one string literal into an uncolored-quote + colored-
        // content pair - deliberately left unclaimed instead, matching the
        // "under-claim is safe" rule this whole module follows.
        LocatedExpr::Char(..) | LocatedExpr::String(..) => Box::new(std::iter::empty()),
        // A `Label` expression is just "a symbol reference" - it could
        // equally be a genuine jump-target label, or an EQU/ASSIGN/MACRO/
        // MODULE name being *used*, and only the old scanner's HashSet-based
        // check (still alive in `semantic_tokens.rs` specifically so it can
        // keep disambiguating this) can currently tell the difference; this
        // walker has no access to that context. Confirmed as a real
        // regression via the differential test: `CPT=CPT-1`'s right-hand
        // `CPT` reference (an ASSIGN name) was being claimed here as a
        // blanket TT_LABEL, when it should be TT_ENUM_MEMBER. Since the old
        // scanner already classifies every such reference correctly (that's
        // the entire reason those HashSets are still populated), leaving
        // this fully unclaimed loses nothing and fixes the regression.
        LocatedExpr::Label(_) | LocatedExpr::PrefixedLabel(..) => Box::new(std::iter::empty()),
        LocatedExpr::Paren(inner, _) | LocatedExpr::UnaryOperation(_, inner, _) => {
            expr_tokens(inner)
        },
        LocatedExpr::BinaryOperation(_, a, b, _) => Box::new(expr_tokens(a).chain(expr_tokens(b))),
        LocatedExpr::Ternary(c, t, f, _) => {
            Box::new(expr_tokens(c).chain(expr_tokens(t)).chain(expr_tokens(f)))
        },
        LocatedExpr::List(items, _) => Box::new(items.iter().flat_map(expr_tokens)),
        LocatedExpr::AnyFunction(_, args, _) => Box::new(args.iter().flat_map(expr_tokens)),
        // Bool/Rnd/RelativeDelta/UnaryTokenOperation: no clean old-scanner
        // parity target and/or rare in hand-written source - left unclaimed.
        LocatedExpr::Bool(..)
        | LocatedExpr::Rnd(_)
        | LocatedExpr::RelativeDelta(..)
        | LocatedExpr::UnaryTokenOperation(..) => Box::new(std::iter::empty())
    }
}

/// Builds a token from `span`'s own boundaries - `None` when that would be
/// invalid or misleading:
/// - a span containing a newline can never be a single valid semantic token
///   (this protocol's delta-encoding is inherently single-line; there is no
///   way to represent a multi-line token). Found to matter for real: a `$`
///   (current-address pseudo-variable) expression nested inside an
///   `IFNDEF` block in a real firmware asset (`deshrink.asm`) produced a
///   `LocatedExpr::Value` whose span - for reasons not chased down further,
///   since `$` is inherently unresolvable at parse time and this is
///   evidently some kind of parser-internal placeholder/quirk around it,
///   not something worth risking a change to a foundational, heavily-tested
///   crate over - covered almost the entire rest of the file. Caught by the
///   differential test against a real corpus, not by any hand-written test.
/// - an empty span (after trimming) has nothing to highlight.
///
/// Trailing whitespace is trimmed off the *end* of the span before
/// measuring length - found to matter for real: `LocatedDataAccess::
/// FlagTest`'s span for `ret z` is `"z\t\t"`, not just `"z"` (confirmed by
/// direct inspection, not documented anywhere) - claiming its raw length
/// would have swallowed trailing whitespace/tabs into the token, changing
/// the old scanner's correct single-character register-colored region into
/// a too-wide one. No leading-whitespace case has been observed in
/// practice, so only the end is trimmed - `relative_line_and_column()`
/// already points at the real start of every span checked so far.
fn span_token(span: &Z80Span, token_type: u32, modifiers: u32) -> Option<RawSemanticToken> {
    let text = span.as_str().trim_end();
    if text.is_empty() || text.contains(['\n', '\r']) {
        return None;
    }
    let (line1, col1) = span.relative_line_and_column();
    Some(RawSemanticToken {
        line: line1.saturating_sub(1) as u32,
        col: col1.saturating_sub(1) as u32,
        len: text.len() as u32,
        token_type,
        modifiers
    })
}

/// Name-bearing statements (label defs, EQU/ASSIGN/MACRO/MODULE names,
/// macro-call names) funnel through the already-existing, already-tested
/// `locate_name_in_statement` - a text-search within the statement's own
/// span rather than trusting any dedicated name-span accessor (none is
/// reachable from this crate; e.g. `Equ`'s internal `label: Z80Span` field
/// is `pub(crate)` to `cpclib-asm`). This also sidesteps needing to know
/// whether a label statement's span includes a trailing `:`: searching for
/// the bare name inside it finds the right offset/length either way.
/// Returns `Option`, not a boxed iterator, since it's always exactly 0 or 1
/// items and every call site already composes it via `.into_iter().chain(..)`.
fn named_token(
    token: &LocatedToken,
    name: &str,
    tt: u32,
    modifiers: u32
) -> Option<RawSemanticToken> {
    if name.is_empty() {
        return None;
    }
    let (line, col) = locate_name_in_statement(token, name);
    Some(RawSemanticToken {
        line,
        col,
        len: name.len() as u32,
        token_type: tt,
        modifiers
    })
}

/// The statement's leading keyword (`MACRO`/`MODULE`/`ORG`/`DB`/...),
/// claimed only when it's a real recognized directive - used both as the
/// generic catch-all for every directive kind this module doesn't
/// specifically handle, and for the `MACRO`/`MODULE` header keyword ahead
/// of their own name token.
fn keyword_token(token: &LocatedToken) -> Option<RawSemanticToken> {
    let span = token.span();
    let text = span.as_str();
    let first_line = text.lines().next().unwrap_or(text);
    let word_len: usize = first_line
        .chars()
        .take_while(|c| c.is_ascii_alphabetic() || *c == '_')
        .map(|c| c.len_utf8())
        .sum();
    if word_len == 0 {
        return None;
    }
    if !DIRECTIVE_SET.contains(first_line[..word_len].to_uppercase().as_str()) {
        return None;
    }
    let (line1, col1) = span.relative_line_and_column();
    Some(RawSemanticToken {
        line: line1.saturating_sub(1) as u32,
        col: col1.saturating_sub(1) as u32,
        len: word_len as u32,
        token_type: TT_MACRO,
        modifiers: 0
    })
}
