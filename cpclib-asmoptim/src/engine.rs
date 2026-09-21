//! Matching an instruction stream against optimization patterns.
//!
//! The engine is generic over `ListingElement`, so it runs against whatever
//! token type the caller already has (the LSP passes `LocatedToken`s straight
//! out of its parsed listing). It never mutates anything and never decides
//! *whether* to apply an optimization - it only reports that a rule matched,
//! where, and what the replacement would be.

use std::collections::HashMap;

use cpclib_asm::flatten::flatten_for_analysis;
use cpclib_asm::implementation::tokens::TokenExt;
use cpclib_asm::parser::LocatedToken;
use cpclib_tokens::{BinaryOperation, DataAccessElem, ExprElement, ListingElement, Mnemonic};

use crate::analysis_op::OpClass;
use crate::constraints::{
    self, ConstraintContext, LivenessContext, Reason, RegionSummary, RegionUse, Writes
};
use crate::dependency::Dependency;
use crate::regflag::Reg;
use crate::effects::effects_of;
use crate::liveness::{self, Liveness};
use crate::match_cost::MatchCost;
use crate::stream::AnalysisStream;
use crate::noopt;
use crate::smc;
use crate::dsl::{
    BinOp, Constraint, InstrPattern, MnemonicPattern, NumberedInstr, OperandPattern, RepeatCount,
    Rule, RuleSet
};

/// How many *instructions* a `*` wildcard may span.
///
/// Counted in instructions, not source lines - basm puts several on a line
/// (`ld a, 5 : inc hl : ret`), and the whole engine works on the flat
/// instruction stream, so a "line" is not a meaningful unit here.
///
/// A peephole rule's `*` means "and some instructions in between" - a gap the
/// rule then constrains (`regsNotModified(1, HL, ?reg)` and friends). Those
/// constraints make a long gap essentially never satisfiable anyway: the
/// longer the gap, the more certain something in it disturbs the register the
/// rule wants to carry across.
///
/// Bounding it keeps matching linear in the instruction count. The cost is a real if
/// narrow capability loss - a rule whose gap genuinely runs longer than this,
/// and whose constraints somehow still hold, is no longer found. 32 is
/// comfortably above any gap seen in the real corpus while keeping a
/// 45k-instruction generated file analysable at all.
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
    T: ListingElement + std::fmt::Display,
    T::DataAccess: DataAccessElem
{
    fn is_used_after(&self, index: u32, dependency: Dependency) -> Option<Liveness> {
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
        let answer = liveness::is_used_after(
            &self.analysis.stream,
            &self.analysis.labels,
            start,
            dependency
        );
        // The walk works in op indices; everything above this trait works in
        // token indices. Convert here so a `Reason`'s witness is already in
        // the caller's own coordinates - an expansion's several ops all map
        // back to the one token the user wrote.
        Some(Liveness {
            witness: answer
                .witness
                .and_then(|op| self.analysis.stream.token_of_op(op)),
            ..answer
        })
    }

    fn region_summary(&self, index: u32) -> Option<RegionSummary> {
        let range = self.positions.get(&index)?.clone();
        let mut summary = RegionSummary::default();
        let mut depth: i32 = 0;

        for op in self.analysis.stream.ops_for_token_range(range)? {
            match op.classify() {
                OpClass::Inert => continue,
                OpClass::Opaque => return None,
                OpClass::Executes => {}
            }
            let effects = effects_of(op)?;
            summary.instruction_count += 1;
            summary.reads_memory |= effects.reads_memory;
            summary.writes_memory |= effects.writes_memory;

            match op.mnemonic() {
                Some(Mnemonic::Push) => {
                    summary.pushes += 1;
                    depth += 1;
                },
                Some(Mnemonic::Pop) => {
                    summary.pops += 1;
                    depth -= 1;
                    // A pop with nothing of its own left to take is taking
                    // whatever the surrounding code pushed.
                    if depth < 0 {
                        summary.reaches_outside_stack = true;
                    }
                },
                _ => {
                    // Everything else that touches SP is a direct use. `call`
                    // and `ret` land here through their own SP effects, which
                    // is right: they move the stack pointer for reasons this
                    // balance count does not track.
                    summary.uses_sp_directly |= effects
                        .reads
                        .iter()
                        .chain(effects.writes.iter())
                        .any(|r| *r == Reg::Sp);
                }
            }
        }
        Some(summary)
    }

    fn callee_takes_stack_arguments(&self, label: &str) -> Option<bool> {
        let start = *self.analysis.labels.get(label)?;
        let ops = self.analysis.stream.ops();

        // A straight-line scan to the routine's first `ret`. Anything that
        // diverges from that - a jump, a nested call, an instruction whose
        // effects are unknown - is answered `None` (undecidable) rather than
        // guessed at, because getting this wrong turns `call X; ret` into
        // `jp X` for a routine that reads its arguments off the stack.
        let mut depth: i32 = 0;
        for op in ops.get(start..)? {
            match op.classify() {
                OpClass::Inert => continue,
                OpClass::Opaque => return None,
                OpClass::Executes => {}
            }
            let effects = effects_of(op)?;
            match op.mnemonic() {
                Some(Mnemonic::Push) => depth += 1,
                Some(Mnemonic::Pop) => {
                    depth -= 1;
                    // Popping something it never pushed: the routine is
                    // reaching into the caller's stack frame, which is exactly
                    // what "takes stack arguments" means. On a CPC this is
                    // also how a routine reads inline parameters placed after
                    // its own call site.
                    if depth < 0 {
                        return Some(true);
                    }
                },
                Some(Mnemonic::Ret) => {
                    // A conditional `ret` leaves the routine along a path this
                    // scan does not follow; only a plain one ends it.
                    return if op.arg1().is_none() && depth == 0 {
                        Some(false)
                    }
                    else {
                        None
                    };
                },
                _ => {
                    if effects
                        .reads
                        .iter()
                        .chain(effects.writes.iter())
                        .any(|r| *r == Reg::Sp)
                    {
                        return None;
                    }
                    // A jump, a call, or a return moves control somewhere this
                    // scan does not model.
                    //
                    // Asked of `cpclib-z80flow`'s own table rather than of a
                    // list kept here: the list this replaces named
                    // `JP`/`JR`/`JQ`/`CALL`/`RST`/`DJNZ` and quietly omitted
                    // `RETI`/`RETN`, so an interrupt handler's own return read
                    // as an ordinary instruction and the scan walked straight
                    // past the end of the routine.
                    if cpclib_z80flow::diverts_control(op.mnemonic()) {
                        return None;
                    }
                }
            }
        }
        None
    }

    fn region_token_span(&self, index: u32) -> Option<std::ops::Range<usize>> {
        self.positions.get(&index).cloned()
    }

    fn writes_of(&self, index: u32) -> Option<Writes> {
        let range = self.positions.get(&index)?.clone();
        let mut writes = Writes::default();

        for op in self.analysis.stream.ops_for_token_range(range)? {
            match op.classify() {
                OpClass::Inert => continue,
                OpClass::Opaque => return None,
                OpClass::Executes => {}
            }
            let effects = effects_of(op)?;

            // A port read counts too: `in a,(c)` drives real hardware, so an
            // instruction carrying one is never removable however dead its
            // registers look. A memory *read* is genuinely free of effects and
            // deliberately not listed.
            writes.has_side_effects |=
                effects.writes_memory || effects.writes_port || effects.reads_port;

            writes
                .deps
                .extend(effects.writes.iter().copied().map(Dependency::Reg));
            writes
                .deps
                .extend(effects.writes_flags.iter().copied().map(Dependency::Flag));
        }
        Some(writes)
    }

    fn region_use(&self, index: u32, dependency: Dependency) -> Option<RegionUse> {
        let range = self.positions.get(&index)?.clone();
        // A region that matched nothing touches nothing - a `*` taking zero
        // instructions trivially satisfies "these registers are not modified
        // here", which is what the rules using it rely on. That falls out of
        // `ops_for_token_range` handing back an empty slice.
        let mut used = RegionUse::default();

        for op in self.analysis.stream.ops_for_token_range(range)? {
            match op.classify() {
                OpClass::Inert => continue,
                OpClass::Opaque => return None,
                OpClass::Executes => {}
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
    /// Names in `texts` whose spelling came from the source and must survive
    /// rendering untouched - see [`Captures::bind_verbatim_text`].
    verbatim: std::collections::HashSet<String>,
    counts: HashMap<String, u32>
}

impl<D> Default for Captures<'_, D> {
    fn default() -> Self {
        Self {
            operands: HashMap::new(),
            texts: HashMap::new(),
            verbatim: std::collections::HashSet::new(),
            counts: HashMap::new()
        }
    }
}

impl<D> Clone for Captures<'_, D> {
    fn clone(&self) -> Self {
        Self {
            operands: self.operands.clone(),
            texts: self.texts.clone(),
            verbatim: self.verbatim.clone(),
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
            // mnemonic slot - keyword either way, unless it was bound
            // verbatim because its spelling is the user's own.
            return !self.verbatim.contains(name);
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

    /// Bind `name` to text that must be rendered back **exactly** as written.
    ///
    /// [`Self::bind_text`] is for keywords (register names, mnemonics), which
    /// may be case-folded to a canonical spelling. This is for anything whose
    /// spelling is the user's: the displacement of an indexed access is
    /// usually a number but is just as often a symbol (`(ix + FRAME_COUNT)`),
    /// and folding that would rewrite it into a different symbol - the exact
    /// failure this crate has been bitten by before with labels.
    fn bind_verbatim_text(&mut self, name: &str, text: String) -> bool {
        if !self.bind_text(name, text) {
            return false;
        }
        self.verbatim.insert(name.to_string());
        true
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
    pub replacement: Vec<String>,
    /// Why this is safe - one entry per safety-bearing constraint that had to
    /// be satisfied, in the order they were checked. Empty for a rule resting
    /// only on the shape of the instructions, where there is nothing to
    /// explain beyond the pattern itself.
    ///
    /// A [`Reason::witness`] indexes the same token slice the match does.
    pub reasons: Vec<Reason>,
    /// [`Rule::is_pure_dead_output_deletion`] for the rule that produced
    /// this match - computed once here, where the actual [`Rule`] is still
    /// in hand, rather than asking every consumer to re-derive it from
    /// [`Self::rule_name`] afterward (which cannot work for an unnamed
    /// rule - see that method's own doc comment). A bulk-apply flow
    /// (an editor's "Fix All", `basmopt --in-place`) must skip any match
    /// with this set; a per-site reviewed flow (a single quickfix, a plain
    /// diagnostic) may still offer it.
    pub bulk_unsafe: bool
}

impl PeepholeMatch {
    /// The matched instructions, as a half-open range of input indices.
    pub fn range(&self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

/// `try_rule`'s real result. A thin wrapper around `PeepholeMatch` today
/// (rather than just returning that directly) so a field can be added here
/// later without touching the public type - no external consumer needs
/// anything beyond `m` today (`cpclib-lsp`, `cpclib-basmopt`, this crate's
/// own `upstream_potests.rs`). The replacement's own cost is deliberately
/// *not* computed here - see `replacement_cost`'s own doc comment for why
/// pricing is deferred to the caller instead of happening eagerly for every
/// successful match.
struct RuleAttempt {
    m: PeepholeMatch
}

/// Find every rule match in `tokens`, ranking same-position candidates for
/// `goal` (see [`MatchCost::cmp_better`]'s own doc comment: `Size` and
/// `Neutral` prefer the candidate that saves the most bytes first, `Speed`
/// prefers the one that saves the most cycles first - a demomaking-specific
/// choice this crate makes on top of upstream's own fixed bytes-first order,
/// since which one actually matters depends on what's being built: a
/// standard demo cares about runtime speed, a size-constrained intro cares
/// about what ships after crunching, which raw byte savings only
/// approximates).
///
/// Rules whose constraints this crate cannot evaluate are skipped entirely -
/// suggesting an optimization whose safety condition was never checked would
/// be worse than suggesting nothing. Address-aware constraints
/// (`reachableByJr`) always report unknown - use [`find_matches_with_resolver`]
/// to evaluate those too.
pub fn find_matches<T>(
    tokens: &[&T],
    rules: &RuleSet,
    goal: crate::OptimizationGoal
) -> Vec<PeepholeMatch>
where
    T: TokenExt + std::fmt::Display,
    T::DataAccess: DataAccessElem
{
    find_matches_with_resolver(tokens, rules, &NoResolver, goal)
}

/// As [`find_matches`], with a [`AddressResolver`] so address-aware
/// constraints (`reachableByJr`) can be evaluated too.
pub fn find_matches_with_resolver<T, R>(
    tokens: &[&T],
    rules: &RuleSet,
    resolver: &R,
    goal: crate::OptimizationGoal
) -> Vec<PeepholeMatch>
where
    T: TokenExt + std::fmt::Display,
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

    // The mnemonic each rule's first line demands, where it demands one at
    // all. `InstrPattern::Instr` matches `tokens[pos]` exactly - it never
    // skips ahead - so a rule whose first line names a mnemonic cannot
    // possibly match a position holding a different one, and does not need
    // `try_rule`'s per-attempt `Captures` and `HashMap` to find that out.
    //
    // This only ever skips attempts that were going to fail regardless of
    // rule order - every rule that passes this pre-filter is still tried at
    // a given position (see the loop below), so file order is not itself a
    // selection criterion.
    let first_mnemonic: Vec<Option<&str>> = usable
        .iter()
        .map(|rule| match rule.match_lines.first().map(|line| &line.instr) {
            Some(InstrPattern::Instr {
                mnemonic: MnemonicPattern::Literal(name),
                ..
            }) => Some(name.as_str()),
            _ => None
        })
        .collect();

    // Instructions something points *into* - see `smc`. A whole-file property,
    // so computed here rather than per candidate match. Unioned with
    // `noopt`'s own veto - a human explicitly marking an instruction as
    // intentional is a different reason to protect it, but the same kind of
    // whole-file, rule-independent hard veto.
    let mut protected = smc::protected_tokens(tokens);
    protected.extend(noopt::protected_tokens(tokens));

    // Shared across every candidate at every position - see its own doc
    // comment (`match_cost::ByteCostCache`) for why keying by an
    // instruction's own text, rather than pricing it fresh every time it
    // recurs, is what makes a match-dense real file affordable at all.
    let cost_cache = crate::match_cost::ByteCostCache::new();

    /// Does this span really execute from its first instruction to its last?
    ///
    /// Every pattern is written as though it does, and every block-local
    /// constraint (`regsNotModified` and friends) reasons on that basis - they
    /// ask what the *instructions* in a region do, which only answers the
    /// question if those instructions are what runs, in that order.
    ///
    /// Two things break it, both taken from real code this got wrong:
    ///
    /// * **A label inside the span.** It is an entry point, so execution can
    ///   arrive in the middle with entirely different register contents.
    ///   `unnecessary-ld` matched `ld c,a` ... `ld a,c` across
    ///   `jr .add_instruction_loop` / `.try_handle_nop:` in `birthtro`'s
    ///   scroller and offered to delete the reload - which is only valid on the
    ///   fall-through path, not for anything jumping to that label.
    /// * **A branch inside the span**, other than as its last instruction:
    ///   whatever follows it does not run next.
    ///
    /// A `call` is deliberately not a branch here - it comes back, so the
    /// sequence continues (which is what lets `tail-recursion` match
    /// `call X; ret`).
    fn span_runs_straight_through<T>(tokens: &[&T], start: usize, end: usize) -> bool
    where T: ListingElement {
        for (offset, token) in tokens[start..end].iter().enumerate() {
            let index = start + offset;
            if index > start && token.is_label() {
                return false;
            }
            let is_branch = matches!(
                token.mnemonic(),
                Some(
                    Mnemonic::Jp
                        | Mnemonic::Jr
                        | Mnemonic::Jq
                        | Mnemonic::Djnz
                        | Mnemonic::Ret
                        | Mnemonic::Reti
                        | Mnemonic::Retn
                        | Mnemonic::Rst
                )
            );
            if is_branch && index + 1 < end {
                return false;
            }
        }
        true
    }

    let mut start = 0usize;
    while start < tokens.len() {
        let here = tokens[start].mnemonic();
        // Every rule applicable at this position is tried - not just until
        // the first one succeeds - and the objectively best surviving
        // candidate wins (`MatchCost::cmp_better`: most bytes saved, then
        // most cycles saved, then fewest constraints, then fewest
        // instructions spanned - see `match_cost.rs`'s own module doc
        // comment for why file order alone used to decide this, and why
        // that was a real gap rather than a deliberate simplification).
        //
        // Two passes, not one: first collect every genuinely eligible rule
        // (pattern matched, constraints satisfied, post-checks - SMC
        // protection, straight-line execution - passed) *without* pricing
        // it, then price and rank only if there's more than one. Pricing
        // (`MatchCost::compute`) does a real per-candidate assemble, and the
        // overwhelming common case is exactly one eligible rule at a
        // position with nothing to rank it against. Measured on a real
        // match-dense file (12k tokens, ~9000 matches, none of them
        // genuinely competing): pricing every candidate regardless cost
        // ~7x more than pricing only on an actual choice (231ms -> 1.65s) -
        // almost all of that gone once singleton candidates skip
        // `MatchCost::compute` entirely. This also fixes a real
        // pre-existing gap as a side effect: previously, a position where
        // the *only* matching rule failed a post-check produced no match at
        // all, even when a different rule would have passed - now every
        // applicable rule gets a real chance.
        let mut eligible: Vec<(&Rule, RuleAttempt)> = Vec::new();
        for (index, rule) in usable.iter().enumerate() {
            if let Some(expected) = first_mnemonic[index]
                && here.is_none_or(|actual| !mnemonic_is(expected, actual))
            {
                continue;
            }
            let Some(attempt) = try_rule(rule, tokens, start, resolver, &analysis)
            else {
                continue;
            };
            if attempt.m.range().any(|i| protected.contains(&i))
                || !span_runs_straight_through(tokens, attempt.m.start, attempt.m.end)
            {
                continue;
            }
            eligible.push((rule, attempt));
        }

        let winner: Option<PeepholeMatch> = if eligible.len() == 1 {
            Some(eligible.pop().unwrap().1.m)
        }
        else {
            let mut best: Option<(PeepholeMatch, MatchCost)> = None;
            for (rule, attempt) in eligible {
                // Priced only now that there is genuinely something to rank
                // it against - see `replacement_cost`'s own doc comment.
                // `unwrap_or((None, None))` is defensive, not expected: this
                // exact text already parsed successfully in `try_rule`'s own
                // `is_assemblable` check.
                let (replacement_bytes, replacement_cycles) =
                    replacement_cost(&attempt.m.replacement, &cost_cache).unwrap_or((None, None));
                let cost = MatchCost::compute(
                    tokens[attempt.m.start..attempt.m.end].iter().copied(),
                    replacement_bytes,
                    replacement_cycles,
                    rule.constraints.len(),
                    attempt.m.end - attempt.m.start,
                    &cost_cache
                );
                // Strictly-greater replaces; a tie keeps whichever was found
                // first (earlier file order) - the same outcome as before
                // this change when nothing objectively distinguishes two
                // rules.
                let is_better = match &best {
                    None => true,
                    Some((_, best_cost)) => {
                        cost.cmp_better(best_cost, goal) == std::cmp::Ordering::Greater
                    }
                };
                if is_better {
                    best = Some((attempt.m, cost));
                }
            }
            best.map(|(m, _)| m)
        };

        match winner {
            Some(m) => {
                start = m.end.max(start + 1);
                matches.push(m);
            },
            None => start += 1
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
    T: ListingElement + std::fmt::Display,
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

/// Whether the instruction at `tokens[start]` sits under a label that is
/// itself loaded into a 16-bit register somewhere in `tokens` and, within a
/// short window afterward, block-copied via `LDI`/`LDIR`/`LDD`/`LDDR` - the
/// shape of code that relocates itself or a jump table at runtime (real
/// example this guards against: skyline's own interrupt vector installer,
/// `vector: jp target ... ld hl,vector : ld de,&38 : ldi : ldi : ldi`,
/// copying the *jump instruction's own bytes* elsewhere before they run).
///
/// This is about the label at the matched instruction's own address, not
/// the jump's target: `reachableByJr` already proves the target is in
/// range *from the address this instruction assembled to*, but if those
/// exact bytes get copied somewhere else before they run, a `JR` computed
/// there would be wrong (its displacement is relative to wherever it
/// executes, not where it was written) even though the original `JP`
/// stayed correct regardless of where its bytes end up.
///
/// A conservative whole-file text scan rather than a real data-flow proof
/// - this crate's established policy (see `constraints.rs`'s memory-
/// aliasing note) is to decline rather than guess when a sound answer
/// isn't cheaply available, and over-declining `jp2jr` here only costs a
/// missed optimization, never a wrong one.
fn instruction_may_be_relocated<T>(tokens: &[&T], start: usize) -> bool
where T: ListingElement + std::fmt::Display {
    const COPY_MNEMONICS: [&str; 4] = ["LDI", "LDIR", "LDD", "LDDR"];
    const LOOKAHEAD: usize = 16;

    // Each token rendered once (`Display` on a generic `T` always builds a
    // fresh `String` - there is no existing borrowed text to point at
    // instead, for either `Token` or `LocatedToken`) and upper-cased once;
    // everything below borrows `&str` slices out of this rather than
    // re-rendering or re-allocating per token.
    let lines: Vec<String> = tokens.iter().map(|t| t.to_string().to_ascii_uppercase()).collect();

    // Every label immediately attached to this instruction - walk backward
    // until a non-label token (an earlier real instruction, or the start of
    // the file) stops it.
    let labels: Vec<&str> = tokens[..start]
        .iter()
        .zip(lines[..start].iter())
        .rev()
        .take_while(|(t, _)| t.is_label())
        .filter_map(|(_, line)| first_word(line))
        .collect();
    if labels.is_empty() {
        return false;
    }

    for label in labels {
        for (i, line) in lines.iter().enumerate() {
            let loads_label = ["LD HL,", "LD DE,", "LD BC,"].iter().any(|p| line.contains(p))
                && contains_word(line, label);
            if !loads_label {
                continue;
            }
            let window_end = (i + LOOKAHEAD).min(lines.len());
            if lines[i..window_end]
                .iter()
                .any(|l| COPY_MNEMONICS.iter().any(|m| contains_word(l, m)))
            {
                return true;
            }
        }
    }
    false
}

/// The first word of `text` (split on anything that isn't alphanumeric or
/// `_`) - used to pull a bare label name out of its already upper-cased
/// `Display` rendering (`"VECTOR:"` -> `"VECTOR"`) without needing a
/// dedicated `ListingElement` accessor for it. Borrows from `text` rather
/// than allocating - `text` is expected to already be upper-cased (see
/// this function's only caller), so there is no case-folding left to do
/// here.
fn first_word(text: &str) -> Option<&str> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .find(|w| !w.is_empty())
}

/// Whether `needle` appears in `haystack` as a whole word (split on
/// anything that isn't alphanumeric or `_`) - a plain `contains` would
/// also match `LDIR` inside `LDIRX` or a label that is a substring of a
/// longer one.
fn contains_word(haystack: &str, needle: &str) -> bool {
    haystack
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|word| word == needle)
}

/// Attempt one rule at one starting position.
fn try_rule<'t, T, R>(
    rule: &Rule,
    tokens: &'t [&'t T],
    start: usize,
    resolver: &'t R,
    analysis: &'t Analysis<'t, T>
) -> Option<RuleAttempt>
where
    T: ListingElement + std::fmt::Display,
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
    // Collected as the constraints decide, so a suggestion can say what makes
    // it safe rather than only that it is.
    let mut reasons = Vec::new();
    for constraint in &rule.constraints {
        if !constraints::evaluate(constraint, &mut captures, &ctx, &mut reasons).is_satisfied() {
            return None;
        }
    }

    // A rule we cannot render a replacement for is not reported at all - see
    // `render_replacement`. Deliberately checked after the constraints, so an
    // unrenderable rule costs nothing on the overwhelming majority of
    // positions where it would not have applied anyway.
    let replacement =
        render_replacement(&rule.replacement_lines, &captures, tokens, &ctx.positions)?;

    // ...and a replacement we *can* render is still no use if the assembler
    // cannot read it back. Rendering only guarantees we produced text, not
    // that the text is basm: an upstream rule emitting `-(10) & 65535` reads
    // as an indirection to basm ("invalid LD: wrong source") and would break
    // the file it was meant to improve. Checking here makes "a suggestion
    // never breaks the source" structural rather than something the tests
    // happen to sample.
    //
    // Deliberately syntax-only, *not* also pricing the replacement here:
    // this runs for every successful pattern+constraint match, including
    // the overwhelming majority that turn out to be the only eligible rule
    // at their position and are never actually ranked against anything (see
    // the caller's own doc comment on why pricing is deferred). Pricing a
    // replacement that never needed it was the single largest cost in an
    // earlier version of this change, measured directly: ~7x slower on a
    // real match-dense file purely from computing real per-candidate cost
    // unconditionally.
    if !replacement.iter().all(|line| is_assemblable(line)) {
        return None;
    }

    // `jp2jr` (`jp ?const1` -> `jr ?const1`, the *only* rule this hazard
    // applies to today - see `instruction_may_be_relocated`'s own doc
    // comment) is unsafe for a jump whose own bytes get copied elsewhere
    // at runtime, regardless of what `reachableByJr` already proved about
    // the target's distance *from where the instruction was assembled*.
    let relocation_unsafe =
        rule.name.as_deref() == Some("jp2jr") && instruction_may_be_relocated(tokens, start);

    Some(RuleAttempt {
        m: PeepholeMatch {
            rule_name: rule.name.clone(),
            message: substitute_message(&rule.description, &captures),
            start,
            end,
            anchor,
            replacement,
            bulk_unsafe: rule.is_pure_dead_output_deletion() || relocation_unsafe,
            reasons
        }
    })
}

/// Whether one rendered replacement line is something basm can actually
/// read - syntax-only, deliberately never assembled (see `try_rule`'s own
/// call site for why pricing must wait until it's known to be needed). A
/// comment line (which a preserved region can contain) parses fine and is
/// accepted.
fn is_assemblable(line: &str) -> bool {
    cpclib_asm::parser::parse_z80_str(format!("    {line}\n")).is_ok()
}

/// Parse every rendered replacement line and price it, using `cache` to
/// avoid re-assembling a line whose exact text this call has already priced
/// - real demoscene sources are full of repeated idioms (`ld b,b`, `xor a`,
/// a save/restore `push`/`pop` pair, ...), and pricing is a real per-line
/// assemble (`TokenExt::number_of_bytes`), not a cheap lookup. `None` (the
/// outer `Option`) the moment a line fails to even parse - should not
/// happen in practice, since `try_rule` already validated this exact text
/// via `is_assemblable` before ever calling this. The two inner `Option`s
/// are independent: byte size and cycle cost can each be unknown without
/// the other being (a fake instruction, ~30 of them in this corpus, prices
/// in bytes via a real assemble but has no cycle-table entry - see
/// `match_cost::token_cycles`).
///
/// Flattened with [`flatten_for_analysis`] (not a bare `.iter()`) for the
/// same reason every other execution-order-sensitive walk in this crate is:
/// a replacement line is rare but not forbidden from containing a nested
/// sequential block, and this must see what actually runs, in order.
fn replacement_cost(
    lines: &[String],
    cache: &crate::match_cost::ByteCostCache
) -> Option<(Option<usize>, Option<u32>)> {
    let mut bytes = Some(0usize);
    let mut cycles = Some(0u32);
    for line in lines {
        if let Some(priced) = cache.get(line) {
            bytes = match (bytes, priced.0) {
                (Some(a), Some(b)) => Some(a + b),
                _ => None
            };
            cycles = match (cycles, priced.1) {
                (Some(a), Some(b)) => Some(a + b),
                _ => None
            };
            continue;
        }
        let listing = cpclib_asm::parser::parse_z80_str(format!("    {line}\n")).ok()?;
        let line_tokens: Vec<&LocatedToken> = flatten_for_analysis(listing.iter()).collect();
        let line_bytes = crate::match_cost::span_bytes(line_tokens.iter().copied());
        let line_cycles = crate::match_cost::span_cycles(line_tokens.iter().copied());
        cache.insert(line.clone(), (line_bytes, line_cycles));
        bytes = match (bytes, line_bytes) {
            (Some(a), Some(b)) => Some(a + b),
            _ => None
        };
        cycles = match (cycles, line_cycles) {
            (Some(a), Some(b)) => Some(a + b),
            _ => None
        };
    }
    Some((bytes, cycles))
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
    T: ListingElement + std::fmt::Display,
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

/// Is `actual` the mnemonic named by `expected`, ignoring case?
///
/// A `Mnemonic`'s name is only reachable through `Display`, and the obvious
/// `expected.eq_ignore_ascii_case(&actual.to_string())` was one heap
/// allocation per rule per token - by a wide margin the hottest thing in the
/// matcher on a large file, and paid in full even for the ~99% of rules whose
/// first instruction was never going to match. This compares the formatted
/// name as it is written, without ever materialising it.
fn mnemonic_is(expected: &str, actual: &Mnemonic) -> bool {
    use std::fmt::Write;

    /// Consumes `Display` output and compares it to `expected` as it arrives.
    struct Compare<'a> {
        rest: &'a str,
        equal: bool
    }

    impl Write for Compare<'_> {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            if !self.equal {
                return Ok(());
            }
            match self.rest.split_at_checked(s.len()) {
                Some((head, tail)) if head.eq_ignore_ascii_case(s) => self.rest = tail,
                _ => self.equal = false
            }
            Ok(())
        }
    }

    let mut compare = Compare {
        rest: expected,
        equal: true
    };
    let _ = write!(compare, "{actual}");
    compare.equal && compare.rest.is_empty()
}

/// Match a single instruction pattern against a single token.
fn match_instr<'a, T>(
    pattern: &InstrPattern,
    token: &'a T,
    captures: &mut Captures<'a, T::DataAccess>
) -> bool
where
    T: ListingElement + std::fmt::Display,
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

    match mnemonic {
        MnemonicPattern::Literal(expected) => {
            if !mnemonic_is(expected, actual_mnemonic) {
                return false;
            }
        },
        MnemonicPattern::Variable(name) => {
            if !captures.bind_text(name, actual_mnemonic.to_string()) {
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

        // `(ix + 4)`, `(?regixiy + ?const1)` - an indexed access, matched
        // against its parts rather than its text.
        //
        // Text comparison cannot do this job: a `?variable` inside the pattern
        // has nothing to render *to* until it binds, and even the fully
        // literal form fails because the pattern's spelling and the operand's
        // `Display` need not agree on spacing or number base. The result was
        // that no indexed operand ever matched - which silently disabled every
        // rule using one, the four `sdcc-*` index-register rules included.
        OperandPattern::Indirect(inner)
            if matches!(
                inner.as_ref(),
                OperandPattern::Binary {
                    op: BinOp::Add,
                    ..
                }
            ) && operand.is_indexregister_with_index() =>
        {
            let OperandPattern::Binary { lhs, rhs, .. } = inner.as_ref()
            else {
                return false;
            };
            let Some(register) = operand.get_indexregister16()
            else {
                return false;
            };
            // The base half names the index register, the offset half its
            // displacement; either may be a literal or a capture.
            let base_ok = match lhs.as_ref() {
                OperandPattern::Ident(name) => name.eq_ignore_ascii_case(&register.to_string()),
                // Bound to the *register name*, not to the whole access: the
                // rules go on to test it with `in(?regixiy,ix,iy)`, which
                // compares against the binding's text.
                OperandPattern::Variable(name) => {
                    name.starts_with("reg") && captures.bind_text(name, register.to_string())
                },
                _ => false
            };
            if !base_ok {
                return false;
            }
            // `get_index` rather than `get_expression`: an indexed access keeps
            // its displacement, *and the `+`/`-` that introduced it*, in its
            // own slot - `get_expression` returns nothing for one.
            //
            // Only `+` matches. The pattern spells the operator literally
            // (`(?regixiy + ?const1)`) and the replacement re-renders it the
            // same way, so matching `(ix - 5)` here would bind `?const1` to
            // `5` and emit `(ix + 5)` - a different address, silently. That
            // really happened: `ld (ix - 5), a` came back as
            // `ld (ix + 0x5), a`.
            //
            // Declining instead of trying to carry the sign through costs
            // very little: the only rules using an indexed operand are the
            // four `sdcc-*` patterns aimed at compiler output, where negative
            // displacements are common but the code is not hand-written CPC
            // source. Getting an address wrong is not worth the coverage.
            let Some((BinaryOperation::Add, offset)) = operand.get_index()
            else {
                return false;
            };
            match rhs.as_ref() {
                OperandPattern::Number(value) => {
                    offset.is_value() && i64::from(offset.value()) == *value
                },
                // Bound to the displacement's own text - it has no
                // `DataAccess` to point at, and it may be a symbol, so it is
                // bound verbatim rather than as a foldable keyword.
                OperandPattern::Variable(name) => {
                    // `to_expr().to_simplified_string()` rather than `Display`:
                    // `ExprElement` is generic over the token type and carries
                    // no `Display` bound of its own.
                    captures
                        .bind_verbatim_text(name, offset.to_expr().to_simplified_string())
                },
                _ => false
            }
        },

        OperandPattern::Indirect(_) | OperandPattern::Binary { .. } | OperandPattern::Unary { .. } => {
            // Everything else structural (`(hl)`, plain arithmetic) is still
            // compared by rendered text - enough for the rules that use it, and
            // never a false positive because a mismatch just fails the rule.
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
    T: ListingElement + std::fmt::Display,
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
            // Rendered through `Display`, which is what preserves the user's
            // own text: a `LocatedToken` displays its source span, so case,
            // number base (`#7F10` stays `#7F10`) and comments all survive
            // into a region the rule promised not to change. A spanless
            // `Token` falls back to its canonical rendering, which is the best
            // that can be done without a source to quote.
            //
            // The two therefore produce deliberately *different text* for the
            // same source, and the parity tests compare what that text parses
            // to rather than the text itself (see `tests/common`).
            let range = positions.get(&line.index)?.clone();
            // A wildcard that matched nothing contributes no line at all,
            // rather than a blank one.
            if range.is_empty() {
                continue;
            }
            // One replacement entry per instruction, rather than joining them
            // with basm's `:` separator. A comment runs to end of line, so
            // joining would fold everything after it *into* the comment -
            // turning `ld bc,#7f10 ; set the gate array : out (c),c` into a
            // silent deletion of the `out`.
            for token in tokens.get(range)? {
                let text = token.to_string().trim().to_string();
                if !text.is_empty() {
                    rendered.push(text);
                }
            }
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
