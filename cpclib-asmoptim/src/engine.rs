//! Matching an instruction stream against optimization patterns.
//!
//! The engine is generic over `ListingElement`, so it runs against whatever
//! token type the caller already has (the LSP passes `LocatedToken`s straight
//! out of its parsed listing). It never mutates anything and never decides
//! *whether* to apply an optimization - it only reports that a rule matched,
//! where, and what the replacement would be.

use std::collections::HashMap;

use cpclib_tokens::{DataAccessElem, ExprElement, ListingElement, Mnemonic};

use crate::constraints::{self, ConstraintContext, LivenessContext, RegionUse};
use crate::dependency::Dependency;
use crate::effects::effects_of;
use crate::liveness::{self, Usage};
use crate::stream::AnalysisStream;
use crate::smc;
use crate::dsl::{
    Constraint, InstrPattern, MnemonicPattern, NumberedInstr, OperandPattern, RepeatCount, Rule,
    RuleSet
};

/// How many instructions a `*` wildcard may span.
///
/// A peephole rule's `*` means "and some instructions in between" - a gap the
/// rule then constrains (`regsNotModified(1, HL, ?reg)` and friends). Those
/// constraints make a long gap essentially never satisfiable anyway: the
/// longer the gap, the more certain something in it disturbs the register the
/// rule wants to carry across.
///
/// Bounding it keeps matching linear in file length. The cost is a real if
/// narrow capability loss - a rule whose gap genuinely runs longer than this,
/// and whose constraints somehow still hold, is no longer found. 32 is
/// comfortably above any gap seen in the real corpus while keeping a
/// 45k-token generated file analysable at all.
const MAX_WILDCARD_SPAN: usize = 32;

/// Resolves the information address-aware constraints (`reachableByJr`) need,
/// for whatever token type the caller is matching.
///
/// The engine calls this once per matched instruction rather than once per
/// constraint check, so a real implementation (backed by an assembled `Env`)
/// pays for the lookup only when a rule actually needs it.
pub trait AddressResolver<T> {
    /// The real assembled address of `token`, if known.
    fn address_of(&self, token: &T) -> Option<u16>;
    /// The resolved value of a label, if known.
    fn value_of_label(&self, name: &str) -> Option<i64>;
}

/// An [`AddressResolver`] that knows nothing - every address-aware constraint
/// reports [`constraints::Verdict::Unknown`], so rules needing one never
/// match. What [`find_matches`] uses.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoResolver;

impl<T> AddressResolver<T> for NoResolver {
    fn address_of(&self, _token: &T) -> Option<u16> {
        None
    }

    fn value_of_label(&self, _name: &str) -> Option<i64> {
        None
    }
}

/// Adapts an [`AddressResolver`] plus one candidate match's line->token
/// positions into the [`ConstraintContext`] `constraints::evaluate` needs.
///
/// Built fresh per candidate match (never shared across matches): "which real
/// token did pattern line N match" is per-match information the resolver
/// itself cannot know.
struct MatchContext<'t, T, R> {
    tokens: &'t [&'t T],
    positions: HashMap<u32, std::ops::Range<usize>>,
    resolver: &'t R,
    /// The normalized instruction stream and its label index, borrowed from
    /// the one built per [`find_matches_with_resolver`] call - never rebuilt
    /// per candidate match, which with ~120 active rules would dominate the
    /// cost of matching entirely.
    analysis: &'t Analysis<'t, T>
}

/// The per-call analysis data every liveness question is answered from.
struct Analysis<'t, T> {
    stream: AnalysisStream<'t, T>,
    labels: HashMap<String, usize>
}

impl<T, R> ConstraintContext for MatchContext<'_, T, R>
where R: AddressResolver<T>
{
    fn address_of_line(&self, index: u32) -> Option<u16> {
        let pos = self.positions.get(&index)?.start;
        let token = *self.tokens.get(pos)?;
        self.resolver.address_of(token)
    }

    fn value_of_label(&self, name: &str) -> Option<i64> {
        self.resolver.value_of_label(name)
    }
}

impl<T, R> LivenessContext for MatchContext<'_, T, R>
where
    T: ListingElement,
    T::DataAccess: DataAccessElem
{
    fn is_used_after(&self, index: u32, dependency: Dependency) -> Option<Usage> {
        let range = self.positions.get(&index)?;
        // "After" means after everything the line matched, not after its first
        // token: a `*` or `[?n]` line covers a whole region, and starting the
        // walk inside it would treat the line's own instructions as if they
        // came afterwards.
        let start = match range.end.checked_sub(1) {
            // Start *past* the whole instruction, expansion included: a fake
            // instruction occupies several ops, and resuming inside one would
            // analyze a fragment of something the user wrote as a unit.
            Some(last) if range.start <= last => self.analysis.stream.after_token(last)?,
            // The line matched nothing at all (a wildcard taking zero
            // instructions), so "after it" is simply where it began.
            _ => self.analysis.stream.first_op_of_token(range.start)?
        };
        Some(liveness::is_used_after(
            &self.analysis.stream,
            &self.analysis.labels,
            start,
            dependency
        ))
    }

    fn region_use(&self, index: u32, dependency: Dependency) -> Option<RegionUse> {
        let range = self.positions.get(&index)?.clone();
        // A line that matched nothing touches nothing - a `*` taking zero
        // instructions trivially satisfies "these registers are not modified
        // here", which is exactly what the rules using it rely on.
        if range.is_empty() {
            return Some(RegionUse::default());
        }

        let ops = self.analysis.stream.ops();
        let first = self.analysis.stream.first_op_of_token(range.start)?;
        let last = self.analysis.stream.after_token(range.end - 1)?;

        let mut used = RegionUse::default();
        for op in ops.get(first..last)? {
            if op.is_label() {
                continue;
            }
            // Same policy as the forward walk: data, or anything carrying no
            // mnemonic that is not a label, is something whose effects cannot
            // be described - and "cannot describe" must never read as
            // "touches nothing".
            if op.is_data() || op.mnemonic().is_none() {
                return None;
            }
            let effects = effects_of(op)?;

            used.reads |= effects
                .reads
                .iter()
                .any(|r| dependency.matches(Dependency::Reg(*r)))
                || effects
                    .reads_flags
                    .iter()
                    .any(|f| dependency.matches(Dependency::Flag(*f)));
            used.writes |= effects
                .writes
                .iter()
                .any(|r| dependency.matches(Dependency::Reg(*r)))
                || effects
                    .writes_flags
                    .iter()
                    .any(|f| dependency.matches(Dependency::Flag(*f)));
        }
        Some(used)
    }
}

/// The operands bound to each `?variable` of a rule, plus the mnemonics bound
/// to any `?op` variable.
#[derive(Debug)]
pub struct Captures<'a, D> {
    operands: HashMap<String, &'a D>,
    /// Name-only bindings: a `?op` mnemonic, or a register name a constraint
    /// derived rather than matched (see [`Captures::bind_text`]).
    texts: HashMap<String, String>,
    counts: HashMap<String, u32>
}

impl<D> Default for Captures<'_, D> {
    fn default() -> Self {
        Self {
            operands: HashMap::new(),
            texts: HashMap::new(),
            counts: HashMap::new()
        }
    }
}

impl<D> Clone for Captures<'_, D> {
    fn clone(&self) -> Self {
        Self {
            operands: self.operands.clone(),
            texts: self.texts.clone(),
            counts: self.counts.clone()
        }
    }
}

impl<'a, D: DataAccessElem> Captures<'a, D> {
    /// The operand bound to `name`, if it was captured from an operand slot.
    pub fn operand_of(&self, name: &str) -> Option<&'a D> {
        self.operands.get(name).copied()
    }

    /// The text bound to `name`, whether it came from an operand, a `?op`
    /// mnemonic slot, or a constraint that derived it.
    pub fn text_of(&self, name: &str) -> Option<String> {
        if let Some(text) = self.texts.get(name) {
            return Some(text.clone());
        }
        self.operands.get(name).map(|op| op.to_string())
    }

    /// Whether `name` is bound to something that is safe to case-fold when
    /// rendering a replacement.
    ///
    /// Registers, condition flags and mnemonics are keywords - their case is
    /// a style choice. A label or expression operand is **not**: rewriting
    /// `jp SomeLabel` as `jr somelabel` would produce a reference to a
    /// different (probably nonexistent) symbol.
    fn is_keyword_binding(&self, name: &str) -> bool {
        if self.texts.contains_key(name) {
            // Derived by a constraint (a register half) or captured from a
            // mnemonic slot - keyword either way.
            return true;
        }
        self.operands.get(name).is_some_and(|op| {
            op.is_register8()
                || op.is_register16()
                || op.is_indexregister8()
                || op.is_indexregister16()
                || op.is_flag_test()
        })
    }

    /// The repetition count bound to a `[?var]` prefix.
    pub fn count_of(&self, name: &str) -> Option<u32> {
        self.counts.get(name).copied()
    }

    /// [`Self::text_of`], case-folded to canonical lower case for keyword
    /// bindings (registers, flags, mnemonics), left untouched otherwise.
    ///
    /// The one text-producing entry point every user-visible rendering
    /// (replacement lines, the substituted diagnostic message) must go
    /// through: without it, the *same* captured register could render as
    /// `A` or `a` depending only on which `ListingElement` implementation
    /// happened to match (`Token` and `LocatedToken` render register operands
    /// with different default case), which is a real, user-visible
    /// inconsistency, not a cosmetic one - caught by running every engine
    /// test against both token types and diffing the results.
    fn rendered_text(&self, name: &str) -> Option<String> {
        let text = self.text_of(name)?;
        Some(if self.is_keyword_binding(name) {
            text.to_ascii_lowercase()
        }
        else {
            text
        })
    }

    /// Bind a name to plain text, requiring a case-insensitive match if it is
    /// already bound.
    ///
    /// Public because some constraints *produce* bindings rather than only
    /// testing them: upstream's `regpair(?pair, ?high, ?low)` is how a rule
    /// gets hold of a register pair's two halves, which its replacement then
    /// refers to by name even though nothing in the match pattern ever bound
    /// them.
    pub fn bind_text(&mut self, name: &str, text: String) -> bool {
        // An operand binding wins if there is one - a constraint must not
        // silently override what was really matched in the source.
        if let Some(existing) = self.operands.get(name) {
            return existing.to_string().eq_ignore_ascii_case(&text);
        }
        match self.texts.get(name) {
            Some(existing) => existing.eq_ignore_ascii_case(&text),
            None => {
                self.texts.insert(name.to_string(), text);
                true
            }
        }
    }

    /// Bind an operand, requiring equality if the name is already bound.
    fn bind_operand(&mut self, name: &str, operand: &'a D) -> bool {
        match self.operands.get(name) {
            Some(existing) => existing.to_data_access() == operand.to_data_access(),
            None => {
                self.operands.insert(name.to_string(), operand);
                true
            }
        }
    }

    fn bind_count(&mut self, name: &str, count: u32) -> bool {
        match self.counts.get(name) {
            Some(existing) => *existing == count,
            None => {
                self.counts.insert(name.to_string(), count);
                true
            }
        }
    }
}

/// One rule matching one run of instructions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeepholeMatch {
    /// The matched rule's `name:`, when it had one.
    pub rule_name: Option<String>,
    /// The matched rule's `pattern:` description, with `?variables`
    /// substituted from the match - this is the message shown to the user.
    pub message: String,
    /// Index into the input slice of the first matched instruction.
    pub start: usize,
    /// Index into the input slice one past the last matched instruction.
    pub end: usize,
    /// Index into the input slice of the instruction the message anchors to
    /// (the one matched by the rule's line numbered [`Rule::ANCHOR`]).
    pub anchor: usize,
    /// The suggested replacement, one entry per instruction. Empty means the
    /// matched instructions should simply be removed.
    pub replacement: Vec<String>
}

impl PeepholeMatch {
    /// The matched instructions, as a half-open range of input indices.
    pub fn range(&self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

/// Find every rule match in `tokens`.
///
/// Rules whose constraints this crate cannot evaluate are skipped entirely -
/// suggesting an optimization whose safety condition was never checked would
/// be worse than suggesting nothing. Address-aware constraints
/// (`reachableByJr`) always report unknown - use [`find_matches_with_resolver`]
/// to evaluate those too.
pub fn find_matches<T>(tokens: &[&T], rules: &RuleSet) -> Vec<PeepholeMatch>
where
    T: ListingElement,
    T::DataAccess: DataAccessElem
{
    find_matches_with_resolver(tokens, rules, &NoResolver)
}

/// As [`find_matches`], with a [`AddressResolver`] so address-aware
/// constraints (`reachableByJr`) can be evaluated too.
pub fn find_matches_with_resolver<T, R>(
    tokens: &[&T],
    rules: &RuleSet,
    resolver: &R
) -> Vec<PeepholeMatch>
where
    T: ListingElement,
    T::DataAccess: DataAccessElem,
    R: AddressResolver<T>
{
    let mut matches = Vec::new();
    let usable: Vec<&Rule> = rules
        .rules
        .iter()
        .filter(|r| constraints::all_supported(&r.constraints))
        .collect();

    // Built once for the whole call, then borrowed by every candidate match.
    let stream = AnalysisStream::build(tokens, |token| resolve_jq(token, resolver));
    let labels = liveness::label_index(&stream);
    let analysis = Analysis { stream, labels };

    // Instructions something points *into* - see `smc`. A whole-file property,
    // so computed here rather than per candidate match.
    let protected = smc::protected_tokens(tokens);

    let mut start = 0usize;
    while start < tokens.len() {
        let mut advanced = false;
        for rule in &usable {
            if let Some(m) = try_rule(rule, tokens, start, resolver, &analysis) {
                let next = m.end.max(start + 1);
                // Every rewrite changes the byte layout, so a match covering a
                // self-modifying-code target is dropped whatever the rule was
                // going to do with it. Dropped *after* matching rather than by
                // skipping the region, so `start` still advances past it and
                // a later rule cannot re-propose the same span.
                if !m.range().any(|i| protected.contains(&i)) {
                    matches.push(m);
                }
                start = next;
                advanced = true;
                break;
            }
        }
        if !advanced {
            start += 1;
        }
    }

    matches
}

/// Replay basm's own `JQ` decision: it assembles to `JR` when the target is
/// within relative range and `JP` otherwise (`Env::assemble_jq`, which uses
/// the same `target - here - 2` displacement `reachableByJr` does). Without
/// real addresses there is nothing to replay, so the instruction stays opaque
/// and any analysis crossing it fails closed.
fn resolve_jq<T, R>(token: &T, resolver: &R) -> Option<Mnemonic>
where
    T: ListingElement,
    T::DataAccess: DataAccessElem,
    R: AddressResolver<T>
{
    if token.mnemonic() != Some(&Mnemonic::Jq) {
        return None;
    }
    let here = resolver.address_of(token)?;
    // The target is the last operand - `jq label` or `jq cc, label`.
    let target = token
        .mnemonic_arg2()
        .or_else(|| token.mnemonic_arg1())?
        .get_expression()
        .filter(|e| e.is_label())
        .map(|e| e.label())
        .and_then(|name| resolver.value_of_label(name))?;

    let delta = target - i64::from(here) - 2;
    Some(if (-128..=127).contains(&delta) {
        Mnemonic::Jr
    }
    else {
        Mnemonic::Jp
    })
}

/// Attempt one rule at one starting position.
fn try_rule<'t, T, R>(
    rule: &Rule,
    tokens: &'t [&'t T],
    start: usize,
    resolver: &'t R,
    analysis: &'t Analysis<'t, T>
) -> Option<PeepholeMatch>
where
    T: ListingElement,
    T::DataAccess: DataAccessElem,
    R: AddressResolver<T>
{
    let mut captures = Captures::default();
    let mut positions = HashMap::new();
    let end = match_lines(
        &rule.match_lines,
        0,
        tokens,
        start,
        &mut captures,
        &mut positions
    )?;

    // A pattern must cover at least one real instruction; a rule made only of
    // wildcards would otherwise "match" everywhere while meaning nothing.
    if end == start {
        return None;
    }

    let anchor = positions.get(&Rule::ANCHOR)?.start;

    let ctx = MatchContext {
        tokens,
        positions,
        resolver,
        analysis
    };
    for constraint in &rule.constraints {
        if !constraints::evaluate(constraint, &mut captures, &ctx).is_satisfied() {
            return None;
        }
    }

    // A rule we cannot render a replacement for is not reported at all - see
    // `render_replacement`. Deliberately checked after the constraints, so an
    // unrenderable rule costs nothing on the overwhelming majority of
    // positions where it would not have applied anyway.
    let replacement =
        render_replacement(&rule.replacement_lines, &captures, tokens, &ctx.positions)?;

    Some(PeepholeMatch {
        rule_name: rule.name.clone(),
        message: substitute_message(&rule.description, &captures),
        start,
        end,
        anchor,
        replacement
    })
}

/// Match `lines[line_idx..]` against `tokens[pos..]`, returning the position
/// just past the match.
///
/// Recursive with backtracking, because wildcards and variable-count repeats
/// make this genuinely non-deterministic: a greedy wildcard that swallows too
/// much has to be able to give instructions back so a later line can match.
fn match_lines<'a, T>(
    lines: &[NumberedInstr],
    line_idx: usize,
    tokens: &[&'a T],
    pos: usize,
    captures: &mut Captures<'a, T::DataAccess>,
    positions: &mut HashMap<u32, std::ops::Range<usize>>
) -> Option<usize>
where
    T: ListingElement,
    T::DataAccess: DataAccessElem
{
    let Some(line) = lines.get(line_idx)
    else {
        return Some(pos);
    };

    match &line.instr {
        InstrPattern::Wildcard => {
            // Try the shortest wildcard first so a match reports the tightest
            // possible span; longer alternatives are explored on failure.
            //
            // Bounded by `MAX_WILDCARD_SPAN` rather than running to end of
            // file. Unbounded, a `*` costs one full clone of the captures and
            // positions maps for every remaining token, at every start
            // position - quadratic, and on a real 45k-token generated source
            // that was ~40s *per rule* (the whole 173-rule set took 436s and
            // timed out). See the run-length measurement in the `Repeat` arm
            // below, which is the same problem in the other looping construct.
            let limit = (tokens.len() - pos).min(MAX_WILDCARD_SPAN);
            for taken in 0..=limit {
                let mut trial = captures.clone();
                let mut trial_positions = positions.clone();
                trial_positions.entry(line.index).or_insert(pos..pos + taken);
                if let Some(end) = match_lines(
                    lines,
                    line_idx + 1,
                    tokens,
                    pos + taken,
                    &mut trial,
                    &mut trial_positions
                ) {
                    *captures = trial;
                    *positions = trial_positions;
                    return Some(end);
                }
            }
            None
        },

        InstrPattern::Repeat { count, instr } => {
            let repetitions: Vec<u32> = match count {
                RepeatCount::Fixed(n) => vec![*n],
                RepeatCount::Variable(name) => {
                    match captures.count_of(name) {
                        // Already bound elsewhere in the rule - only that
                        // exact count is admissible.
                        Some(bound) => vec![bound],
                        // Otherwise prefer longer runs: a rule collapsing N
                        // repeats is more useful the more it collapses.
                        //
                        // Only counts up to the run that actually matches here
                        // are worth trying. Measuring it first costs one
                        // forward scan; the obvious alternative - offering
                        // every count from "all remaining tokens" downwards
                        // and letting each trial discover it fails - is
                        // quadratic in file length *per position*, and on a
                        // real 11k-token source that alone took one such rule
                        // from ~1ms to 3.2s. It matters most where the rule
                        // does not apply at all, which is almost everywhere:
                        // a non-matching first token now yields an empty range
                        // and no trials whatsoever.
                        //
                        // Sound because a repeat is a prefix property: if n
                        // consecutive instructions match, so does every
                        // shorter count. The probe accumulates bindings across
                        // the run exactly as a real trial would, so it finds
                        // the same maximum a trial for that count would reach.
                        None => {
                            let mut probe = captures.clone();
                            let mut run = 0u32;
                            let mut cursor = pos;
                            while let Some(token) = tokens.get(cursor) {
                                if !match_instr(instr, *token, &mut probe) {
                                    break;
                                }
                                cursor += 1;
                                run += 1;
                            }
                            (1..=run).rev().collect()
                        }
                    }
                }
            };

            for n in repetitions {
                let mut trial = captures.clone();
                let mut trial_positions = positions.clone();
                if let RepeatCount::Variable(name) = count
                    && !trial.bind_count(name, n)
                {
                    continue;
                }

                let mut cursor = pos;
                let mut ok = true;
                for _ in 0..n {
                    match tokens.get(cursor) {
                        Some(token) if match_instr(instr, *token, &mut trial) => cursor += 1,
                        _ => {
                            ok = false;
                            break;
                        }
                    }
                }
                if !ok {
                    continue;
                }

                trial_positions.entry(line.index).or_insert(pos..cursor);
                if let Some(end) = match_lines(
                    lines,
                    line_idx + 1,
                    tokens,
                    cursor,
                    &mut trial,
                    &mut trial_positions
                ) {
                    *captures = trial;
                    *positions = trial_positions;
                    return Some(end);
                }
            }
            None
        },

        InstrPattern::Instr { .. } => {
            let token = tokens.get(pos)?;
            let mut trial = captures.clone();
            if !match_instr(&line.instr, *token, &mut trial) {
                return None;
            }
            let mut trial_positions = positions.clone();
            trial_positions.entry(line.index).or_insert(pos..pos + 1);
            let end = match_lines(
                lines,
                line_idx + 1,
                tokens,
                pos + 1,
                &mut trial,
                &mut trial_positions
            )?;
            *captures = trial;
            *positions = trial_positions;
            Some(end)
        }
    }
}

/// Match a single instruction pattern against a single token.
fn match_instr<'a, T>(
    pattern: &InstrPattern,
    token: &'a T,
    captures: &mut Captures<'a, T::DataAccess>
) -> bool
where
    T: ListingElement,
    T::DataAccess: DataAccessElem
{
    let InstrPattern::Instr { mnemonic, operands } = pattern
    else {
        // Nested wildcards/repeats are not meaningful inside a repeat body.
        return false;
    };

    let Some(actual_mnemonic) = token.mnemonic()
    else {
        return false;
    };
    let actual_name = actual_mnemonic.to_string();

    match mnemonic {
        MnemonicPattern::Literal(expected) => {
            if !expected.eq_ignore_ascii_case(&actual_name) {
                return false;
            }
        },
        MnemonicPattern::Variable(name) => {
            if !captures.bind_text(name, actual_name) {
                return false;
            }
        }
    }

    let actual_operands = [token.mnemonic_arg1(), token.mnemonic_arg2()];
    let expected_count = operands.len();
    let actual_count = actual_operands.iter().filter(|o| o.is_some()).count();
    if expected_count != actual_count {
        return false;
    }

    for (expected, actual) in operands.iter().zip(actual_operands.into_iter().flatten()) {
        if !match_operand(expected, actual, captures) {
            return false;
        }
    }

    true
}

/// Match one operand pattern against one real operand.
fn match_operand<'a, D>(pattern: &OperandPattern, operand: &'a D, captures: &mut Captures<'a, D>) -> bool
where D: DataAccessElem {
    match pattern {
        OperandPattern::Variable(name) => {
            // The variable's *kind* is encoded in its name prefix, per the
            // format's own convention.
            let kind_ok = if name.starts_with("reg") {
                operand.is_register8()
                    || operand.is_register16()
                    || operand.is_indexregister8()
                    || operand.is_indexregister16()
            }
            else if name.starts_with("const") || name.starts_with("8bitconst") {
                operand.is_expression()
            }
            else {
                // `?any` and anything else matches whatever is there.
                true
            };
            kind_ok && captures.bind_operand(name, operand)
        },

        OperandPattern::Number(value) => {
            operand
                .get_expression()
                .is_some_and(|e| e.is_value() && i64::from(e.value()) == *value)
        },

        OperandPattern::Ident(name) => match_ident(name, operand),

        OperandPattern::Indirect(_) | OperandPattern::Binary { .. } | OperandPattern::Unary { .. } => {
            // Structural operands (`(hl)`, `(?ix+?const)`, arithmetic) are
            // compared by rendered text - enough for the rules this crate
            // supports today, and never produces a false positive because a
            // mismatch just fails the rule.
            render_pattern_text(pattern)
                .is_some_and(|text| text.eq_ignore_ascii_case(&operand.to_string()))
        }
    }
}

/// Match a bare identifier - a register name, a condition flag, or a label.
fn match_ident<D>(name: &str, operand: &D) -> bool
where D: DataAccessElem {
    // Registers and flags render exactly as written, so a case-insensitive
    // comparison against the operand's own rendering handles every one of
    // them without restating the register tables here.
    if name.eq_ignore_ascii_case(&operand.to_string()) {
        return true;
    }
    // A label operand renders with its own text; compare that too.
    operand
        .get_expression()
        .is_some_and(|e| e.is_label() && e.label().eq_ignore_ascii_case(name))
}

/// Render a literal (capture-free) operand pattern back to text.
fn render_pattern_text(pattern: &OperandPattern) -> Option<String> {
    Some(match pattern {
        OperandPattern::Ident(name) => name.clone(),
        OperandPattern::Number(value) => value.to_string(),
        OperandPattern::Indirect(inner) => format!("({})", render_pattern_text(inner)?),
        OperandPattern::Variable(_) => return None,
        OperandPattern::Unary { .. } | OperandPattern::Binary { .. } => return None
    })
}

/// Build the replacement text, one entry per replacement line, or `None` if
/// any single line cannot be rendered.
///
/// All-or-nothing on purpose. An empty `Vec` is a *meaningful* replacement -
/// it is how rules like `unnecessary-ld-to-itself` say "delete the matched
/// instructions" - so silently dropping the lines that failed to render would
/// turn "I could not express this rewrite" into "delete this code", and a
/// partial success would emit a rewrite with instructions missing from the
/// middle of it. Both are far worse than declining the match: a rule whose
/// replacement we cannot write down is a rule we have no business suggesting.
fn render_replacement<T>(
    lines: &[NumberedInstr],
    captures: &Captures<'_, T::DataAccess>,
    tokens: &[&T],
    positions: &HashMap<u32, std::ops::Range<usize>>
) -> Option<Vec<String>>
where
    T: ListingElement,
    T::DataAccess: DataAccessElem
{
    let mut rendered = Vec::with_capacity(lines.len());
    for line in lines {
        // A `*` in a *replacement* means "and these instructions stay as they
        // are" - 118 of the upstream rules use one. It has to be written back
        // out verbatim, because a consumer replaces the whole matched span
        // with these lines: emitting nothing here would silently delete the
        // very instructions the rule promised to preserve.
        if matches!(line.instr, InstrPattern::Wildcard) {
            // Rendered from the parsed token, i.e. canonically: mnemonics come
            // back upper case and `1+2` as `0x1 + 0x2`. Symbols are *not*
            // touched (`MyRoutine` and `.skip` survive verbatim), so this is
            // safe - but applying such a fix does reformat instructions the
            // rule never intended to change, and drops their comments, the
            // same way `cpclib_basmopt::apply_fixes` already does for a
            // matched line's trailing comment.
            //
            // A `LocatedToken` could render its own source span verbatim
            // instead, which would be strictly nicer. Not done here because it
            // would make the two `ListingElement` implementations produce
            // *different* replacement text for the same source, and the engine
            // deliberately asserts they agree (see `tests/common`).
            let range = positions.get(&line.index)?.clone();
            // A wildcard that matched nothing contributes no line at all,
            // rather than a blank one.
            if range.is_empty() {
                continue;
            }
            let kept: Vec<String> = tokens
                .get(range)?
                .iter()
                .map(|token| token.to_token().to_string().trim().to_string())
                .collect();
            rendered.push(kept.join(" : "));
        }
        else {
            rendered.push(render_instr(&line.instr, captures)?);
        }
    }
    Some(rendered)
}

fn render_instr<D>(pattern: &InstrPattern, captures: &Captures<'_, D>) -> Option<String>
where D: DataAccessElem {
    match pattern {
        // Handled by `render_replacement`, which has the matched tokens a
        // wildcard needs; unreachable from a nested position because a
        // wildcard is not meaningful inside a repeat body.
        InstrPattern::Wildcard => None,
        InstrPattern::Repeat { count, instr } => {
            let n = match count {
                RepeatCount::Fixed(n) => *n,
                RepeatCount::Variable(name) => captures.count_of(name)?
            };
            let body = render_instr(instr, captures)?;
            Some(
                std::iter::repeat_n(body, n as usize)
                    .collect::<Vec<_>>()
                    .join(" : ")
            )
        },
        InstrPattern::Instr { mnemonic, operands } => {
            let name = match mnemonic {
                MnemonicPattern::Literal(name) => name.to_ascii_lowercase(),
                MnemonicPattern::Variable(var) => captures.text_of(var)?.to_ascii_lowercase()
            };
            if operands.is_empty() {
                return Some(name);
            }
            let rendered: Option<Vec<String>> = operands
                .iter()
                .map(|op| render_operand(op, captures))
                .collect();
            Some(format!("{} {}", name, rendered?.join(", ")))
        }
    }
}

/// Render one replacement operand.
///
/// A captured variable renders as **the text of the operand it matched**, so
/// a symbolic operand keeps its original spelling: an optimizer that rewrote
/// `jp some_label` into `jr 0xc123` would silently hardcode an address that
/// goes stale the moment the label moves.
fn render_operand<D>(pattern: &OperandPattern, captures: &Captures<'_, D>) -> Option<String>
where D: DataAccessElem {
    match pattern {
        // Keywords render in the engine's own canonical lower case (a
        // consumer that cares re-cases the whole suggestion to match the
        // surrounding source); a symbolic operand is returned untouched.
        OperandPattern::Variable(name) => captures.rendered_text(name),
        OperandPattern::Ident(name) => Some(name.clone()),
        OperandPattern::Number(value) => Some(value.to_string()),
        OperandPattern::Indirect(inner) => Some(format!("({})", render_operand(inner, captures)?)),
        OperandPattern::Unary { op, operand } => {
            let inner = render_operand(operand, captures)?;
            Some(match op {
                crate::dsl::UnOp::Neg => format!("-{inner}"),
                crate::dsl::UnOp::Not => format!("~{inner}")
            })
        },
        OperandPattern::Binary { lhs, op, rhs } => {
            let a = render_operand(lhs, captures)?;
            let b = render_operand(rhs, captures)?;
            let sym = match op {
                crate::dsl::BinOp::Add => "+",
                crate::dsl::BinOp::Sub => "-",
                crate::dsl::BinOp::Mul => "*",
                crate::dsl::BinOp::Div => "/",
                crate::dsl::BinOp::Mod => "%",
                crate::dsl::BinOp::ShiftLeft => "<<",
                crate::dsl::BinOp::ShiftRight => ">>",
                crate::dsl::BinOp::BitAnd => "&",
                crate::dsl::BinOp::BitOr => "|",
                crate::dsl::BinOp::BitXor => "^",
                crate::dsl::BinOp::Equal => "==",
                crate::dsl::BinOp::NotEqual => "!=",
                crate::dsl::BinOp::Less => "<",
                crate::dsl::BinOp::LessEqual => "<=",
                crate::dsl::BinOp::Greater => ">",
                crate::dsl::BinOp::GreaterEqual => ">="
            };
            Some(format!("{a} {sym} {b}"))
        }
    }
}

/// Substitute `?variables` in a rule's description with what they matched.
fn substitute_message<D>(description: &str, captures: &Captures<'_, D>) -> String
where D: DataAccessElem {
    let mut out = String::with_capacity(description.len());
    let mut rest = description;
    while let Some(at) = rest.find('?') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let name_len = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(after.len());
        let name = &after[..name_len];
        match captures.rendered_text(name) {
            Some(text) => out.push_str(&text),
            None => {
                out.push('?');
                out.push_str(name);
            }
        }
        rest = &after[name_len..];
    }
    out.push_str(rest);
    out
}

/// Constraints of a rule that this crate cannot evaluate, for diagnostics.
pub fn unsupported_constraints(rule: &Rule) -> Vec<&Constraint> {
    rule.constraints
        .iter()
        .filter(|c| !constraints::SUPPORTED.contains(&c.name.as_str()))
        .collect()
}
