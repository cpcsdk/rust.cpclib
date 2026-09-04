//! Register value tracking for hover: walks forward from the nearest
//! control-flow boundary to the hovered position, using `cpclib_z80emu`'s
//! `track::TrackedState`/`track::apply` (a partial-knowledge, `Option`-based
//! instruction-effect simulator - see that crate for the actual per-
//! instruction rules). This module is the LSP-specific integration layer:
//! finding the right token to stop at, deciding where a walk may safely
//! start, parsing the project's own `IN:`/`OUT:` register-contract comment
//! convention, and formatting the result for hover markdown.

use cpclib_asm::assembler::Env;
use cpclib_asm::parser::obtained::{LocatedDataAccess, LocatedListing, LocatedToken, MayHaveSpan};
use cpclib_tokens::{IndexRegister16, ListingElement, Mnemonic, Register8, Register16};
use cpclib_z80emu::track::{self, TrackedState};
use tower_lsp::lsp_types::Position;

use super::token::flatten_listing;

/// 0-based `(line, column)` of `token`'s own span start.
fn token_position<T: MayHaveSpan>(token: &T) -> (u32, u32) {
    let (line_1based, col_1based) = token.span().relative_line_and_column();
    (
        line_1based.saturating_sub(1) as u32,
        col_1based.saturating_sub(1) as u32
    )
}

/// Whether entering/crossing `token` should reset all tracked state to
/// fully unknown - a label (any label: it could be a jump target reached
/// from elsewhere), a control-transfer instruction, a macro/struct
/// invocation (arbitrary unknown code, can write any register), or entry
/// into a `REPEAT`/`REPEAT UNTIL`/`WHILE`/`FOR`/`ITERATE` block (compile-
/// time unrolled - the same source position represents N different runtime
/// instructions with potentially different entry state each iteration, no
/// single correct answer without per-iteration simulation).
fn is_boundary(token: &LocatedToken) -> bool {
    if token.is_warning() {
        return is_boundary(token.warning_token());
    }
    if token.is_label()
        || token.is_repeat()
        || token.is_repeat_until()
        || token.is_while()
        || token.is_for()
        || token.is_iterate()
        || token.is_call_macro_or_build_struct()
    {
        return true;
    }
    // Asked of `cpclib-z80flow`'s own table rather than of a list kept here.
    // The list this replaces named JP/JR/CALL/RET/RETI/RETN/DJNZ/RST and
    // omitted `JQ` (basm's "assembler picks JR or JP" form). That omission
    // happened not to be observable - `track::apply` clears its state for any
    // instruction it does not model, so `JQ` reset the walk anyway - but it
    // was one of three copies of this question in the workspace, each
    // complete in a different way, and only one of them can be right about
    // the next instruction someone adds.
    cpclib_z80flow::diverts_control(token.mnemonic().copied())
}

/// Tracked state at `position` for the register named `hovered_register_upper`.
///
/// The general rule is the value *before* whatever instruction the cursor
/// sits on/in would execute. One deliberate exception: when the token at
/// `position` is an `LD dst, src` and the hovered register is exactly
/// `dst`, the natural reading of "what does this register hold" is *after*
/// this line runs (e.g. hovering `B` in `ld bc, 6*256+7` should show the
/// value `LD` is about to give it, not whatever `B` held beforehand) - so
/// that one instruction's own effect is applied before returning. A source
/// operand (or any other instruction) keeps the "before" reading.
///
/// Finds the last token (in `flatten_listing`'s source-order stream) whose
/// span starts at or before `position`, walks backward to the nearest
/// boundary, then walks forward from there applying `track::apply` to a
/// fresh, all-unknown state.
///
/// Single forward pass over `flatten_listing`'s lazy stream (never
/// materializes the whole document into a `Vec`, and stops as soon as it
/// passes `position`): `target` holds the latest token seen whose position
/// is `<= position` - the *previous* held value, once superseded by a later
/// one, is definitely not the final target, so it's folded into `state`
/// right then (reset on a boundary, else `track::apply`), exactly mirroring
/// the old two-pass `start_idx..target_idx` walk. The final `target` itself
/// is deliberately never folded in this loop - only conditionally applied
/// after, via the LD-destination exception below - matching the old code
/// excluding `target_idx` from the `start_idx..target_idx` range.
pub(super) fn register_state_at(
    listing: &LocatedListing,
    env: &mut Env,
    position: Position,
    hovered_register_upper: &str
) -> TrackedState {
    let target_pos = (position.line, position.character);

    let mut state = TrackedState::default();
    let mut target: Option<&LocatedToken> = None;

    for token in flatten_listing(listing.iter()) {
        if token_position(token) > target_pos {
            break;
        }
        if let Some(prev) = target.replace(token) {
            if is_boundary(prev) {
                state = TrackedState::default();
            }
            else {
                track::apply(&mut state, prev, env);
            }
        }
    }

    let Some(target) = target
    else {
        return TrackedState::default();
    };

    if is_ld_destination(target, hovered_register_upper) {
        track::apply(&mut state, target, env);
    }

    state
}

/// Whether `token` is `LD dst, ...` with `dst` exactly the register named
/// `word_upper` (case already normalized by the caller).
fn is_ld_destination(token: &LocatedToken, word_upper: &str) -> bool {
    if token.is_warning() {
        return is_ld_destination(token.warning_token(), word_upper);
    }
    if !matches!(token.mnemonic(), Some(Mnemonic::Ld)) {
        return false;
    }
    match token.mnemonic_arg1() {
        Some(LocatedDataAccess::Register8(r, _)) => r.to_string() == word_upper,
        Some(LocatedDataAccess::Register16(r, _)) => r.to_string() == word_upper,
        Some(LocatedDataAccess::IndexRegister8(r, _)) => r.to_string() == word_upper,
        Some(LocatedDataAccess::IndexRegister16(r, _)) => r.to_string() == word_upper,
        _ => false
    }
}

// ─── comment-contract parsing ───────────────────────────────────────────────

/// A function's documented register contract, parsed from the real
/// `IN:`/`OUT:` comment convention already in use in real project code
/// (`;IN:    HL = Address of the song.`, `;OUT:   IX, IY = unmodified.`,
/// case/spacing variations like `; Input: HL = palette address` and
/// `; input : A channel to read` all accepted). Register names may carry a
/// trailing `'` marking the *shadow* register (`DE'`/`BC'`, also seen in
/// real code) - kept as part of the stored name for display, stripped only
/// for matching against a hovered (necessarily non-shadow-spelled, since
/// `'` isn't valid basm syntax) register name.
pub(super) struct FunctionContract {
    pub inputs: Vec<(String, String)>,
    // Parsed and test-verified (round-tripped through `parse_function_contract`),
    // but hover only ever surfaces `inputs` (a hovered register's *known
    // value* comes from tracked state, not from what a function documents
    // it writes) - kept for parity with `inputs` and because a future
    // hover/signature enrichment is a natural use for it.
    #[allow(dead_code)]
    pub outputs: Vec<(String, String)>
}

enum Section {
    In,
    Out
}

/// Parse the contiguous `;`-comment block directly above `label_line`
/// (0-based), if any, into a `FunctionContract`. Returns `None` when
/// there's no comment block, or it contains no `IN:`/`OUT:` section at all
/// (most functions in most codebases won't be annotated, and this feature
/// must degrade gracefully rather than require it).
pub(super) fn parse_function_contract(text: &str, label_line: u32) -> Option<FunctionContract> {
    let lines: Vec<&str> = text.lines().collect();
    let mut comment_lines: Vec<&str> = Vec::new();
    let mut i = label_line as i64 - 1;
    while i >= 0 {
        let trimmed = lines[i as usize].trim_start();
        if !trimmed.starts_with(';') {
            break;
        }
        comment_lines.push(trimmed);
        i -= 1;
    }
    comment_lines.reverse();

    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut current: Option<Section> = None;

    for raw in comment_lines {
        let content = raw.trim_start_matches(';').trim();
        if let Some((section, rest)) = match_section_keyword(content) {
            parse_register_entries(rest, &section, &mut inputs, &mut outputs);
            current = Some(section);
        }
        else if let Some(section) = &current {
            parse_register_entries(content, section, &mut inputs, &mut outputs);
        }
        // else: prose before any IN:/OUT: keyword was found - ignored.
    }

    if inputs.is_empty() && outputs.is_empty() {
        None
    }
    else {
        Some(FunctionContract { inputs, outputs })
    }
}

/// If `content` starts with an `IN`/`INPUT`/`OUT`/`OUTPUT` section keyword
/// (case-insensitive, optional space before `:`), the section and the
/// remainder of the line after the `:`. Longer keywords (`input`/`output`)
/// are checked before their prefixes (`in`/`out`) so `"Input: ..."` isn't
/// misread as keyword `"in"` + remainder `"put: ..."`.
/// A section keyword and the constructor for the `Section` it introduces.
type SectionKeyword = (&'static str, fn() -> Section);

fn match_section_keyword(content: &str) -> Option<(Section, &str)> {
    const KEYWORDS: &[SectionKeyword] = &[
        ("input", || Section::In),
        ("in", || Section::In),
        ("output", || Section::Out),
        ("out", || Section::Out)
    ];
    for (kw, make_section) in KEYWORDS {
        if content.len() < kw.len() || !content[..kw.len()].eq_ignore_ascii_case(kw) {
            continue;
        }
        let after_kw = content[kw.len()..].trim_start();
        if let Some(rest) = after_kw.strip_prefix(':') {
            return Some((make_section(), rest.trim()));
        }
    }
    None
}

fn parse_register_entries(
    rest: &str,
    section: &Section,
    inputs: &mut Vec<(String, String)>,
    outputs: &mut Vec<(String, String)>
) {
    // Two sub-conventions both seen in real project code: `HL = Address of
    // the song.` (explicit `=`) and, without one, `A channel to read`
    // (first word is the register, the rest is free-text description).
    let (reg_list, desc) = match rest.split_once('=') {
        Some((regs, desc)) => (regs.to_string(), desc.trim().to_string()),
        None => {
            match rest.split_once(char::is_whitespace) {
                Some((reg, desc)) => (reg.to_string(), desc.trim().to_string()),
                None => (rest.to_string(), String::new())
            }
        },
    };
    let target = match section {
        Section::In => &mut *inputs,
        Section::Out => &mut *outputs
    };
    for reg in reg_list.split(',') {
        let reg_upper = reg.trim().to_ascii_uppercase();
        if is_tracked_register_name(&reg_upper) {
            target.push((reg_upper, desc.clone()));
        }
    }
}

fn is_tracked_register_name(name: &str) -> bool {
    let base = name.trim_end_matches('\'');
    matches!(
        base,
        "A" | "B" | "C" | "D" | "E" | "H" | "L" | "BC" | "DE" | "HL" | "IX" | "IY" | "SP"
    )
}

fn contract_input_for<'a>(
    contract: &'a FunctionContract,
    word_upper: &str
) -> Option<&'a (String, String)> {
    contract
        .inputs
        .iter()
        .find(|(name, _)| name.trim_end_matches('\'') == word_upper)
}

// ─── all-registers status bar ───────────────────────────────────────────────

/// Every tracked register's hex value at a position - `None` per field when
/// that register's value isn't statically known there. The same 13
/// registers `tracked_value_hex` recognizes (`AF`/`PC`/`I`/`R`/flags aren't
/// tracked, same as the single-register hover).
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct AllRegisters {
    pub a: Option<String>,
    pub b: Option<String>,
    pub c: Option<String>,
    pub d: Option<String>,
    pub e: Option<String>,
    pub h: Option<String>,
    pub l: Option<String>,
    pub bc: Option<String>,
    pub de: Option<String>,
    pub hl: Option<String>,
    pub ix: Option<String>,
    pub iy: Option<String>,
    pub sp: Option<String>
}

/// All 13 tracked registers' values at `position` in one pass - same
/// underlying `register_state_at` walk the single-register hover already
/// uses, called once with an empty `hovered_register_upper` (never matches
/// a real register name, so the "LD destination" after-this-instruction
/// exception never triggers - every register uniformly reads "value before
/// this instruction", the same reading `register_state_at`'s own tests use
/// for anything other than the one register actually being asked about).
pub(super) fn all_tracked_registers_at(
    listing: &LocatedListing,
    env: &mut Env,
    position: Position
) -> AllRegisters {
    let state = register_state_at(listing, env, position, "");
    let get = |name: &str| tracked_value_hex(name, &state).flatten();
    AllRegisters {
        a: get("A"),
        b: get("B"),
        c: get("C"),
        d: get("D"),
        e: get("E"),
        h: get("H"),
        l: get("L"),
        bc: get("BC"),
        de: get("DE"),
        hl: get("HL"),
        ix: get("IX"),
        iy: get("IY"),
        sp: get("SP")
    }
}

// ─── hover formatting ───────────────────────────────────────────────────────

fn tracked_value_hex(word_upper: &str, state: &TrackedState) -> Option<Option<String>> {
    let hex8 = |v: Option<u8>| v.map(|v| format!("0x{v:02X}"));
    let hex16 = |v: Option<i32>| v.map(|v| format!("0x{v:04X}"));
    Some(match word_upper {
        "A" => hex8(state.get8(Register8::A)),
        "B" => hex8(state.get8(Register8::B)),
        "C" => hex8(state.get8(Register8::C)),
        "D" => hex8(state.get8(Register8::D)),
        "E" => hex8(state.get8(Register8::E)),
        "H" => hex8(state.get8(Register8::H)),
        "L" => hex8(state.get8(Register8::L)),
        "BC" => hex16(state.get16(Register16::Bc)),
        "DE" => hex16(state.get16(Register16::De)),
        "HL" => hex16(state.get16(Register16::Hl)),
        "IX" => hex16(state.get_index16(IndexRegister16::Ix)),
        "IY" => hex16(state.get_index16(IndexRegister16::Iy)),
        "SP" => hex16(state.sp()),
        _ => return None
    })
}

/// Markdown line to append to the existing `register_description` hover
/// text, or `None` when `word_upper` isn't a tracked register at all
/// (`AF`/`PC`/`I`/`R`/condition codes - the existing description stays
/// untouched for those). Explicit uncertainty: an untracked value never
/// silently omits this line - it says "not statically known" (optionally
/// explaining *why*, when a documented function contract names it as an
/// input) rather than staying quiet, since silence would be
/// indistinguishable from a feature that simply forgot to answer.
pub(super) fn format_known_value(
    word_upper: &str,
    state: &TrackedState,
    contract: Option<&FunctionContract>
) -> Option<String> {
    let value = tracked_value_hex(word_upper, state)?;
    Some(match value {
        Some(hex) => format!("**Known value at this point:** `{hex}`"),
        None => {
            match contract.and_then(|c| contract_input_for(c, word_upper)) {
                Some((name, desc)) => {
                    format!("Not tracked numerically — documented input: *`{name} = {desc}`*")
                },
                None => "Value not statically known at this point.".to_string()
            }
        },
    })
}

#[cfg(test)]
mod tests {
    use cpclib_asm::parser::context::ParserContextBuilder;
    use tower_lsp::lsp_types::Position;

    use super::*;

    fn state_at(text: &str, line: u32, character: u32) -> TrackedState {
        // "" never matches a real register name, so this never triggers
        // the LD-destination "after" exception - tests exercising that
        // specifically use `state_at_hovering` below.
        state_at_hovering(text, line, character, "")
    }

    fn state_at_hovering(
        text: &str,
        line: u32,
        character: u32,
        hovered_register_upper: &str
    ) -> TrackedState {
        let builder = ParserContextBuilder::default().set_quiet(true);
        let listing = LocatedListing::new_complete_source(text, builder)
            .unwrap_or_else(|_| panic!("expected {text:?} to parse cleanly"));
        let mut env = Env::default();
        register_state_at(
            &listing,
            &mut env,
            Position { line, character },
            hovered_register_upper
        )
    }

    #[test]
    fn tracks_forward_within_a_straight_line_sequence() {
        let s = state_at("    ld a,5\n    ld b,10\n    nop\n", 2, 4);
        assert_eq!(s.get8(Register8::A), Some(5));
        assert_eq!(s.get8(Register8::B), Some(10));
    }

    #[test]
    fn resets_after_a_label() {
        let s = state_at("    ld a,5\nfoo:\n    nop\n", 2, 4);
        assert_eq!(s.get8(Register8::A), None);
    }

    #[test]
    fn resets_after_a_jp() {
        let s = state_at(
            "    ld a,5\n    jp somewhere\n    nop\nsomewhere:\n    nop\n",
            2,
            4
        );
        assert_eq!(s.get8(Register8::A), None);
    }

    #[test]
    fn resets_after_a_call() {
        let s = state_at("    ld a,5\n    call foo\n    nop\nfoo:\n    ret\n", 2, 4);
        assert_eq!(s.get8(Register8::A), None);
    }

    #[test]
    fn resets_after_a_ret() {
        let s = state_at("foo:\n    ld a,5\n    ret\n    nop\n", 3, 4);
        assert_eq!(s.get8(Register8::A), None);
    }

    #[test]
    fn resets_after_a_rst() {
        let s = state_at("    ld a,5\n    rst 0x08\n    nop\n", 2, 4);
        assert_eq!(s.get8(Register8::A), None);
    }

    #[test]
    fn resets_on_entering_a_repeat_block() {
        let s = state_at("    ld a,5\n    repeat 3\n    nop\n    endrepeat\n", 2, 4);
        assert_eq!(s.get8(Register8::A), None);
    }

    /// Regression test for the single-pass streaming rewrite of
    /// `register_state_at`: a boundary mid-sequence, followed by *several*
    /// tracked instructions before the hovered position - stresses that
    /// resetting `state` when a superseded candidate turns out to be a
    /// boundary doesn't interfere with correctly folding the *subsequent*
    /// (post-boundary) candidates one at a time as later tokens keep
    /// superseding them.
    #[test]
    fn resets_at_a_mid_sequence_boundary_then_tracks_forward_across_several_instructions() {
        let text = "    ld a,1\n    ld c,2\nfoo:\n    ld b,10\n    ld d,20\n    nop\n";
        let s = state_at(text, 5, 4);
        assert_eq!(s.get8(Register8::A), None);
        assert_eq!(s.get8(Register8::C), None);
        assert_eq!(s.get8(Register8::B), Some(10));
        assert_eq!(s.get8(Register8::D), Some(20));
    }

    /// Every control transfer resets the walk, `jq` included.
    ///
    /// Note what this does *not* prove: `jq` passed here even before
    /// `is_boundary` knew about it, because `track::apply` clears its state
    /// for any instruction it does not model. The property is worth pinning
    /// regardless - it is the behaviour users see - but the reason the
    /// boundary check now comes from one shared table is
    /// single-source-of-truth, not a bug this test would have caught.
    #[test]
    fn a_jq_resets_the_walk_like_every_other_control_transfer() {
        for transfer in [
            "jq elsewhere",
            "jr elsewhere",
            "jp elsewhere",
            "call elsewhere"
        ] {
            let text = format!("    ld a,1\n    {transfer}\n    nop\n");
            let s = state_at(&text, 2, 4);
            assert_eq!(
                s.get8(Register8::A),
                None,
                "`{transfer}` must reset the tracked state"
            );
        }

        // The control: an ordinary instruction in the same position does not
        // reset anything, so the test above is not passing for a trivial
        // reason.
        let s = state_at("    ld a,1\n    ld c,2\n    nop\n", 2, 4);
        assert_eq!(s.get8(Register8::A), Some(1));
    }

    #[test]
    fn hovering_an_ld_destination_shows_the_value_after_this_line_not_before() {
        // "ld bc, 6*256 + 7" - B/C were never set before this line, but
        // hovering B (the destination) should show what this LD is about
        // to give it (6), not "unknown".
        let text = "    ld bc, 6*256 + 7\n";
        let col_b = text.lines().next().unwrap().find('b').unwrap() as u32; // "bc" - the b
        let s = state_at_hovering(text, 0, col_b, "BC");
        assert_eq!(s.get8(Register8::B), Some(6));
        assert_eq!(s.get8(Register8::C), Some(7));
    }

    #[test]
    fn hovering_an_ld_source_still_shows_the_value_before_this_line() {
        let text = "    ld a,5\n    ld b,a\n";
        let col_a = text.lines().nth(1).unwrap().find('a').unwrap() as u32;
        // Hovering the source "a" in "ld b,a" - not a destination, so the
        // ordinary "before this line" rule applies (still correctly 5,
        // since A was already known from the previous line either way,
        // but crucially via the *walk*, not the after-this-instruction
        // exception).
        let s = state_at_hovering(text, 1, col_a, "A");
        assert_eq!(s.get8(Register8::A), Some(5));
    }

    /// Regression test for the reported bug, end to end through
    /// `register_state_at` (not just `cpclib_z80emu::track::apply` in
    /// isolation, see that crate's own `implicit_accumulator_shorthand_is_not_a_no_op`
    /// test): `add d` (implicit-accumulator shorthand) must invalidate A,
    /// so hovering D as the destination of the following `ld d, a` must not
    /// still report the pre-`add` value of A.
    #[test]
    fn add_shorthand_invalidates_a_before_a_following_ld_destination() {
        let text = "    ld a, 0x8\n    add d\n    ld d, a\n";
        let col_d = text.lines().nth(2).unwrap().find('d').unwrap() as u32;
        let s = state_at_hovering(text, 2, col_d, "D");
        assert_eq!(s.get8(Register8::D), None);
    }

    #[test]
    fn hovering_the_ld_destination_of_an_unresolvable_source_still_reports_unknown() {
        // The destination-after exception must not silently guess when the
        // instruction's own effect is itself unknown (e.g. source depends
        // on another never-set register).
        let text = "    ld a,c\n";
        let col_a = text.lines().next().unwrap().find('a').unwrap() as u32;
        let s = state_at_hovering(text, 0, col_a, "A");
        assert_eq!(s.get8(Register8::A), None);
    }

    // ─── comment-contract parsing ──────────────────────────────────────────

    #[test]
    fn parses_the_in_colon_convention() {
        let text = ";IN:    IX = Data block of the Track.\n;       DE'= Instrument table. Do not modify!\n;       BC'= Note index table. Do not modify!\nfoo:\n";
        let contract = parse_function_contract(text, 3).expect("expected a contract");
        assert!(
            contract
                .inputs
                .iter()
                .any(|(n, d)| n == "IX" && d == "Data block of the Track.")
        );
        assert!(
            contract
                .inputs
                .iter()
                .any(|(n, d)| n == "DE'" && d.starts_with("Instrument table"))
        );
        assert!(contract.inputs.iter().any(|(n, _)| n == "BC'"));
    }

    #[test]
    fn parses_input_output_spelling_and_spacing_variants() {
        let text = "; Input: HL = palette address\nfoo:\n";
        let contract = parse_function_contract(text, 1).expect("expected a contract");
        assert_eq!(contract.inputs[0].0, "HL");

        let text2 = "; input : A channel to read\n; output: A volume\nfoo:\n";
        let contract2 = parse_function_contract(text2, 2).expect("expected a contract");
        assert_eq!(contract2.inputs[0].0, "A");
        assert_eq!(contract2.outputs[0].0, "A");
    }

    #[test]
    fn parses_comma_separated_registers_and_unmodified_output() {
        let text = ";OUT:   IX, IY = unmodified.\nfoo:\n";
        let contract = parse_function_contract(text, 1).expect("expected a contract");
        assert!(contract.outputs.iter().any(|(n, _)| n == "IX"));
        assert!(contract.outputs.iter().any(|(n, _)| n == "IY"));
    }

    #[test]
    fn no_contract_when_no_comment_block_present() {
        let text = "foo:\n    nop\n";
        assert!(parse_function_contract(text, 0).is_none());
    }

    #[test]
    fn no_contract_when_comment_block_has_no_in_out_keyword() {
        let text = ";Just a plain description, no contract.\nfoo:\n";
        assert!(parse_function_contract(text, 1).is_none());
    }

    // ─── hover formatting ───────────────────────────────────────────────────

    #[test]
    fn format_known_value_shows_hex_when_known() {
        let state = state_at("    ld a,5\n    nop\n", 1, 4);
        let md = format_known_value("A", &state, None).unwrap();
        assert_eq!(md, "**Known value at this point:** `0x05`");
    }

    #[test]
    fn format_known_value_reports_unknown_explicitly() {
        let state = TrackedState::default();
        let md = format_known_value("A", &state, None).unwrap();
        assert_eq!(md, "Value not statically known at this point.");
    }

    #[test]
    fn format_known_value_enriches_unknown_with_a_documented_contract() {
        let state = TrackedState::default();
        let contract = FunctionContract {
            inputs: vec![("HL".to_string(), "Address of the song".to_string())],
            outputs: vec![]
        };
        let md = format_known_value("HL", &state, Some(&contract)).unwrap();
        assert!(md.contains("documented input"));
        assert!(md.contains("Address of the song"));
    }

    #[test]
    fn format_known_value_is_none_for_an_untracked_word() {
        let state = TrackedState::default();
        assert!(format_known_value("AF", &state, None).is_none());
        assert!(format_known_value("NZ", &state, None).is_none());
    }

    // ─── all-registers status bar ───────────────────────────────────────────

    fn all_registers_at(text: &str, line: u32, character: u32) -> AllRegisters {
        let builder = ParserContextBuilder::default().set_quiet(true);
        let listing = LocatedListing::new_complete_source(text, builder)
            .unwrap_or_else(|_| panic!("expected {text:?} to parse cleanly"));
        let mut env = Env::default();
        all_tracked_registers_at(&listing, &mut env, Position { line, character })
    }

    #[test]
    fn all_tracked_registers_at_reports_every_known_value_at_once() {
        let regs = all_registers_at("    ld a,5\n    ld bc,0x1234\n    nop\n", 2, 4);
        assert_eq!(regs.a.as_deref(), Some("0x05"));
        assert_eq!(regs.bc.as_deref(), Some("0x1234"));
        // B/C individually reflect the same BC load.
        assert_eq!(regs.b.as_deref(), Some("0x12"));
        assert_eq!(regs.c.as_deref(), Some("0x34"));
        // Never touched - unknown, not silently zero.
        assert_eq!(regs.hl, None);
        assert_eq!(regs.sp, None);
    }

    #[test]
    fn all_tracked_registers_at_resets_after_a_boundary_like_the_single_register_walk() {
        let regs = all_registers_at("    ld a,5\nfoo:\n    nop\n", 2, 4);
        assert_eq!(regs.a, None);
    }

    /// Unlike the single-register hover, this never applies the
    /// "LD destination" after-this-instruction exception to any register -
    /// every field consistently reads "value before this instruction".
    #[test]
    fn all_tracked_registers_at_never_applies_the_ld_destination_exception() {
        let regs = all_registers_at("    ld bc, 6*256 + 7\n", 0, 4);
        assert_eq!(regs.bc, None);
        assert_eq!(regs.b, None);
        assert_eq!(regs.c, None);
    }
}
