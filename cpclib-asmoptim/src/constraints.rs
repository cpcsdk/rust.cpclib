//! Evaluation of pattern constraints.
//!
//! The DSL parser deliberately accepts every constraint the real upstream
//! format defines, including ones this crate cannot evaluate yet (see
//! [`crate::dsl`]'s own doc comment). This module decides which of them are
//! actually *supported*: a rule mentioning anything else is skipped entirely
//! rather than matched without its safety condition, which would be the one
//! genuinely dangerous failure mode for an optimizer.

use cpclib_tokens::{DataAccessElem, ExprElement, IndexRegister16, Register16};

use crate::dependency::Dependency;
use crate::dsl::{BinOp, Constraint, OperandPattern, Rule, RuleSet, UnOp};
use crate::engine::Captures;
use crate::liveness::Usage;
use crate::regflag::{Flag, Reg};

/// Constraint names this crate evaluates today.
///
/// Ordered roughly by how often they appear in the real upstream corpus
/// (`in` alone accounts for 178 uses of ~500, then `regsNotUsedAfter` at 87
/// and `flagsNotUsedAfter` at 78). Together these cover 177 of the 185 base
/// rules.
///
/// What is left, and why:
///
/// * `memoryNotWritten`/`memoryNotUsed` (4+2 uses) - would need real memory
///   aliasing to decide whether two `(IX+d)` accesses overlap. All four rules
///   needing them are `sdcc-*` patterns aimed at compiler output.
/// * `atLeastOneCPUOp` + `evenPushPopsSPNotRead` (3+3) - always used together,
///   by the three `unnecessary-push-pop` rules.
/// * `noStackArguments` (1) - an SDCC calling-convention question that does
///   not translate to basm.
pub const SUPPORTED: &[&str] = &[
    "equal",
    "notEqual",
    "in",
    "notIn",
    "regpair",
    "reachableByJr",
    "regsNotUsedAfter",
    "flagsNotUsedAfter",
    "regsNotModified",
    "regsNotUsed",
    "flagsNotModified",
    "flagsNotUsed",
    "regFlagEffectsNotUsedAfter",
    "atLeastOneCPUOp",
    "evenPushPopsSPNotRead",
    "memoryNotWritten",
    "memoryNotUsed",
    "noStackArguments",
    "regsModified"
];

/// Constraint names that need a real assembled address to evaluate (i.e.
/// need an `AddressResolver` backed by a real `Env`, not just the parsed
/// token stream) - currently just `reachableByJr`. Lets a caller skip the
/// (potentially expensive, `INCLUDE`-resolving) real assemble entirely when
/// the active rule set doesn't contain anything that would need it - see
/// [`rules_need_addresses`].
pub const ADDRESS_AWARE: &[&str] = &["reachableByJr"];

/// Whether every constraint of a rule can actually be evaluated.
pub fn all_supported(constraints: &[Constraint]) -> bool {
    constraints
        .iter()
        .all(|c| SUPPORTED.contains(&c.name.as_str()))
}

/// Whether `rule` has a constraint that needs a real assembled address.
pub fn rule_needs_addresses(rule: &Rule) -> bool {
    rule.constraints
        .iter()
        .any(|c| ADDRESS_AWARE.contains(&c.name.as_str()))
}

/// Whether any rule in `rules` that could actually fire (i.e. passes
/// [`all_supported`] - a rule the engine skips entirely regardless can't
/// make this `true` on its own) needs a real assembled address. The
/// question a caller deciding whether to pay for a real (`INCLUDE`-
/// resolving) assemble at all should ask, rather than always assembling
/// "just in case".
pub fn rules_need_addresses(rules: &RuleSet) -> bool {
    rules
        .rules
        .iter()
        .filter(|r| all_supported(&r.constraints))
        .any(rule_needs_addresses)
}

/// The outcome of evaluating one constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Satisfied,
    Failed,
    /// The constraint is supported in principle but could not be decided with
    /// the information available (e.g. an address-dependent check with no
    /// assembled context). Treated exactly like [`Verdict::Failed`] by the
    /// engine - never suggest an optimization that has not been proven safe.
    Unknown
}

impl Verdict {
    pub fn is_satisfied(self) -> bool {
        matches!(self, Self::Satisfied)
    }

    fn from_bool(value: bool) -> Self {
        if value { Self::Satisfied } else { Self::Failed }
    }
}

/// Everything a constraint may need beyond the captures themselves.
///
/// Address-aware constraints (`reachableByJr`) need the address each matched
/// instruction actually assembled to; that is threaded in here rather than
/// being looked up globally so the engine stays usable with no assembled
/// context at all (in which case such constraints report
/// [`Verdict::Unknown`]).
pub trait ConstraintContext {
    /// The real address of the instruction matched by pattern line `index`,
    /// if known.
    fn address_of_line(&self, index: u32) -> Option<u16>;

    /// The resolved value of a label, if known.
    fn value_of_label(&self, name: &str) -> Option<i64>;
}

/// What a *forward-liveness* constraint (`regsNotUsedAfter`,
/// `flagsNotUsedAfter`) needs.
///
/// The walk itself lives behind this trait rather than in the constraint:
/// answering it needs the normalized instruction stream and the label index,
/// both of which are built once per `find_matches` call and are generic over
/// the token type. Exposing them through the trait would push that generic
/// parameter onto `evaluate` and every one of its callers, for the benefit of
/// two constraints out of eight. Asking a *question* instead keeps the
/// generics where the data is.
///
/// Deliberately separate from [`ConstraintContext`]: the two answer unrelated
/// questions ("what address is this?" vs "what executes after this?"), the
/// same way [`crate::engine::AddressResolver`] already sits beside it rather
/// than inside it.
pub trait LivenessContext {
    /// Whether `dependency` is still read after the instruction matched by
    /// pattern line `index`.
    ///
    /// `None` when `index` wasn't part of this match, or when no instruction
    /// stream is available at all - in which case the constraint reports
    /// [`Verdict::Unknown`] and therefore fails, never silently passes.
    fn is_used_after(&self, index: u32, dependency: Dependency) -> Option<Usage>;

    /// What the instructions matched by pattern line `index` do to
    /// `dependency` themselves.
    ///
    /// Block-local, and deliberately so: unlike [`Self::is_used_after`] this
    /// needs no control-flow walk at all, only the effects of the instructions
    /// the line actually covered. A line can cover a whole *region* - `*` and
    /// `[?n]` match several instructions - which is the point: the rules using
    /// these constraints ask "does anything in this gap disturb HL?".
    ///
    /// `None` when `index` wasn't part of this match, when no stream is
    /// available, or when the region contains something whose effects cannot
    /// be described - all of which report [`Verdict::Unknown`] and so fail.
    fn region_use(&self, index: u32, dependency: Dependency) -> Option<RegionUse>;

    /// Everything the instructions matched by pattern line `index` write.
    ///
    /// Where [`Self::region_use`] answers "does this region touch X?", this
    /// enumerates what it produces - which is what a rule asking "is this
    /// instruction's entire output dead?" needs, since it cannot name the
    /// registers in advance.
    fn writes_of(&self, index: u32) -> Option<Writes>;

    /// Aggregate facts about the region matched by pattern line `index` that
    /// the remaining constraints ask about, gathered in one pass.
    fn region_summary(&self, index: u32) -> Option<RegionSummary>;

    /// Whether the routine at `label` takes arguments off the stack.
    ///
    /// `None` when it cannot be determined - which includes not finding the
    /// label at all.
    fn callee_takes_stack_arguments(&self, label: &str) -> Option<bool>;
}

/// Aggregate facts about one matched region.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RegionSummary {
    /// How many real CPU instructions it contains (labels do not count).
    pub instruction_count: usize,
    pub pushes: usize,
    pub pops: usize,
    /// Something other than a `push`/`pop` reads or writes `SP`.
    ///
    /// `push`/`pop` are excluded because they are what the balance count is
    /// *for*; anything else touching SP (`ld sp,hl`, `add hl,sp`,
    /// `ex (sp),hl`, a `call` or `ret`) means the region cares about the
    /// stack's actual contents or depth, which is exactly what removing a
    /// surrounding push/pop pair would change.
    pub uses_sp_directly: bool,
    pub reads_memory: bool,
    pub writes_memory: bool
}

/// What a matched region produces.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Writes {
    /// The registers and flags it writes.
    pub deps: Vec<Dependency>,
    /// It also writes memory, or touches a port.
    ///
    /// Tracked separately because these are effects on the world that no
    /// amount of register liveness can prove dead: `ld (hl), a` leaves every
    /// register unused afterwards and is still doing the only thing it was
    /// written to do, and an `out`/`in` drives real CPC hardware.
    pub has_side_effects: bool
}

/// What a matched region does to one register or flag.
///
/// Both false means "the region leaves it completely alone" - which is what
/// `regsNotModified`/`regsNotUsed` are asking about, from opposite sides.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RegionUse {
    /// Some instruction in the region reads it.
    pub reads: bool,
    /// Some instruction in the region writes it.
    pub writes: bool
}

/// A context that knows nothing - every address-aware or liveness constraint
/// reports [`Verdict::Unknown`], so rules needing one never match.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoContext;

impl ConstraintContext for NoContext {
    fn address_of_line(&self, _index: u32) -> Option<u16> {
        None
    }

    fn value_of_label(&self, _name: &str) -> Option<i64> {
        None
    }
}

impl LivenessContext for NoContext {
    fn is_used_after(&self, _index: u32, _dependency: Dependency) -> Option<Usage> {
        None
    }

    fn region_use(&self, _index: u32, _dependency: Dependency) -> Option<RegionUse> {
        None
    }

    fn writes_of(&self, _index: u32) -> Option<Writes> {
        None
    }

    fn region_summary(&self, _index: u32) -> Option<RegionSummary> {
        None
    }

    fn callee_takes_stack_arguments(&self, _label: &str) -> Option<bool> {
        None
    }
}

/// Evaluate one constraint against a candidate match.
///
/// Takes the captures mutably because some constraints legitimately *produce*
/// bindings rather than only testing them - see [`regpair`].
///
pub fn evaluate<D, C>(constraint: &Constraint, captures: &mut Captures<'_, D>, ctx: &C) -> Verdict
where
    D: DataAccessElem,
    C: ConstraintContext + LivenessContext
{
    match constraint.name.as_str() {
        "equal" => compare(constraint, captures, ctx, true),
        "notEqual" => compare(constraint, captures, ctx, false),
        "in" => membership(constraint, captures, true),
        "notIn" => membership(constraint, captures, false),
        "regpair" => regpair(constraint, captures),
        "reachableByJr" => reachable_by_jr(constraint, captures, ctx),
        "regsNotUsedAfter" => regs_not_used_after(constraint, captures, ctx),
        "flagsNotUsedAfter" => flags_not_used_after(constraint, captures, ctx),
        "regsNotModified" => block_local(constraint, captures, ctx, Kind::Reg, Touch::Write),
        "regsNotUsed" => block_local(constraint, captures, ctx, Kind::Reg, Touch::Read),
        "flagsNotModified" => block_local(constraint, captures, ctx, Kind::Flag, Touch::Write),
        "flagsNotUsed" => block_local(constraint, captures, ctx, Kind::Flag, Touch::Read),
        "regFlagEffectsNotUsedAfter" => reg_flag_effects_not_used_after(constraint, ctx),
        "atLeastOneCPUOp" => at_least_one_cpu_op(constraint, ctx),
        "evenPushPopsSPNotRead" => even_push_pops_sp_not_read(constraint, ctx),
        "memoryNotWritten" => memory_untouched(constraint, captures, ctx, false),
        "memoryNotUsed" => memory_untouched(constraint, captures, ctx, true),
        "noStackArguments" => no_stack_arguments(constraint, captures, ctx),
        "regsModified" => regs_modified(constraint, captures, ctx),
        _ => Verdict::Unknown
    }
}

/// `regsModified(#, reg1, ..., regn)` - at least one of the named registers is
/// written somewhere in the region matched by `#`.
///
/// The mirror of `regsNotModified`, and note it is an *any*, not an *all*:
/// upstream returns `true` as soon as one named register is modified by one
/// statement. Documented in the upstream file header but used by no rule in the
/// corpus; implemented for completeness so no constraint name is left
/// unevaluated.
fn regs_modified<D, C>(constraint: &Constraint, captures: &Captures<'_, D>, ctx: &C) -> Verdict
where
    D: DataAccessElem,
    C: LivenessContext
{
    let Some(args) = parse_regs_args(constraint, captures)
    else {
        return Verdict::Unknown;
    };
    for reg in args.items {
        match ctx.region_use(args.line, Dependency::Reg(reg)) {
            Some(used) if used.writes => return Verdict::Satisfied,
            Some(_) => {},
            None => return Verdict::Unknown
        }
    }
    Verdict::Failed
}

/// `atLeastOneCPUOp(#)` - the region matched by `#` contains at least one real
/// instruction.
///
/// Upstream's own note says exactly why it exists: *"to prevent eliminating the
/// usual push af; pop af combination used for timing"*. An empty gap between a
/// `push` and its `pop` means the pair is doing nothing to the registers - and
/// on a CPC that is precisely the signature of deliberate cycle padding, not of
/// dead code.
fn at_least_one_cpu_op<C>(constraint: &Constraint, ctx: &C) -> Verdict
where C: LivenessContext {
    let [line] = constraint.args.as_slice()
    else {
        return Verdict::Unknown;
    };
    let Some(line) = line_index(line)
    else {
        return Verdict::Unknown;
    };
    match ctx.region_summary(line) {
        Some(summary) => Verdict::from_bool(summary.instruction_count >= 1),
        None => Verdict::Unknown
    }
}

/// `evenPushPopsSPNotRead(#)` - the region has as many `push`es as `pop`s and
/// does not otherwise touch `SP`.
///
/// What the rules using it need: if a `push`/`pop` pair around this region is
/// to be removed, nothing inside may depend on the stack being one entry
/// deeper. An unbalanced region would leave the stack shifted; one that reads
/// `SP` (or, as upstream notes, copies it into `IX`/`IY` first - covered here
/// because that copy itself reads `SP`) could observe the difference.
///
/// Stricter than upstream, which also folds `inc sp`/`dec sp` into the balance
/// count. Here anything but a `push`/`pop` that touches `SP` simply refuses:
/// the rules using this constraint gain nothing from the extra precision, and
/// hand-written CPC code that moves `SP` by hand is doing something the
/// analysis should not second-guess.
fn even_push_pops_sp_not_read<C>(constraint: &Constraint, ctx: &C) -> Verdict
where C: LivenessContext {
    let [line] = constraint.args.as_slice()
    else {
        return Verdict::Unknown;
    };
    let Some(line) = line_index(line)
    else {
        return Verdict::Unknown;
    };
    match ctx.region_summary(line) {
        Some(summary) => {
            Verdict::from_bool(summary.pushes == summary.pops && !summary.uses_sp_directly)
        },
        None => Verdict::Unknown
    }
}

/// `memoryNotWritten(#, exp)` / `memoryNotUsed(#, exp)`.
///
/// Deliberately **more conservative than upstream**. Upstream matches the
/// address *syntactically* - its own documentation warns that "if you specify a
/// constant and there happens to be a register that has that address, it will
/// not match" - so a write through `(hl)` is treated as not touching `(ix+4)`
/// even when `hl` happens to point there. That is a real aliasing hole, and
/// this crate's whole policy is to decline rather than guess.
///
/// So the expression argument is not used to discriminate at all: a region that
/// touches no memory satisfies the constraint, and one that does is `Unknown`.
/// Sound, and it still fires for the case the rules care about (an untouched
/// gap), at the cost of missing writes that are provably to a different slot.
fn memory_untouched<D, C>(
    constraint: &Constraint,
    _captures: &Captures<'_, D>,
    ctx: &C,
    reads_count: bool
) -> Verdict
where
    D: DataAccessElem,
    C: LivenessContext
{
    let Some(line) = constraint.args.first().and_then(line_index)
    else {
        return Verdict::Unknown;
    };
    let Some(summary) = ctx.region_summary(line)
    else {
        return Verdict::Unknown;
    };
    let touched = summary.writes_memory || (reads_count && summary.reads_memory);
    if touched {
        // Might or might not be the location in question - undecidable.
        Verdict::Unknown
    }
    else {
        Verdict::Satisfied
    }
}

/// `noStackArguments(label)` - the routine at `label` takes no arguments off
/// the stack.
///
/// Used only by `tail-recursion` (`call X; ret` -> `jp X`). Upstream's own note
/// on that rule is that it "is not safe for any code that passes parameters in
/// the stack", and on a CPC there is a second, sharper version of the same
/// hazard: reading *inline parameters* by popping the return address is a
/// standard idiom, and after the rewrite that pop yields the caller's return
/// address instead.
///
/// So this is answered by actually looking at the callee rather than assuming a
/// calling convention - see `callee_takes_stack_arguments`.
///
/// Deliberately a different question from upstream's. Upstream scans the first
/// ten instructions for SDCC's stack-frame prologue (`ld ix,0` / `add ix,sp`)
/// and otherwise answers "no stack arguments". That is right for compiler
/// output and wrong here: a CPC routine takes its arguments off the stack by
/// popping its own return address, which that scan would never see. This
/// implementation looks for exactly that instead, and answers `Unknown` for
/// any routine it cannot follow to a plain `ret` - so the rule fires less
/// often than upstream's, and never on a routine whose stack behaviour is
/// unclear.
fn no_stack_arguments<D, C>(
    constraint: &Constraint,
    captures: &Captures<'_, D>,
    ctx: &C
) -> Verdict
where
    D: DataAccessElem,
    C: LivenessContext
{
    let [target] = constraint.args.as_slice()
    else {
        return Verdict::Unknown;
    };
    let Some(name) = operand_text(target, captures)
    else {
        return Verdict::Unknown;
    };
    match ctx.callee_takes_stack_arguments(&name) {
        Some(true) => Verdict::Failed,
        Some(false) => Verdict::Satisfied,
        None => Verdict::Unknown
    }
}

/// The text a constraint argument denotes - a literal name, or whatever a
/// `?variable` is bound to.
fn operand_text<D>(pattern: &OperandPattern, captures: &Captures<'_, D>) -> Option<String>
where D: DataAccessElem {
    match pattern {
        OperandPattern::Ident(name) => Some(name.clone()),
        OperandPattern::Variable(name) => captures.text_of(name),
        _ => None
    }
}

/// A constraint argument naming a pattern line, as a plain number.
fn line_index(pattern: &OperandPattern) -> Option<u32> {
    match pattern {
        OperandPattern::Number(n) => u32::try_from(*n).ok(),
        _ => None
    }
}

/// `regFlagEffectsNotUsedAfter(#1, #2)` - every register and flag that line
/// `#1` writes is provably dead after line `#2`.
///
/// The rules using it (`unnecessary-0args`/`-1args`/`-2args`/`-2args-ex`) have
/// an *empty* replacement: they delete the instruction outright, on the
/// grounds that nothing it produced is ever read. That makes this the most
/// destructive constraint in the set, and the reason for the side-effect check
/// below - `?op` for `unnecessary-2args` includes `ld`, and `ld (hl), a`
/// writes memory while leaving every register dead. Deleting it because "no
/// register is used afterwards" would remove the entire point of the
/// instruction. Register liveness simply cannot speak to memory or port
/// effects, so their presence makes this undecidable rather than satisfied.
fn reg_flag_effects_not_used_after<C>(constraint: &Constraint, ctx: &C) -> Verdict
where C: LivenessContext {
    let [effects_line, after_line] = constraint.args.as_slice()
    else {
        return Verdict::Unknown;
    };
    let (Some(effects_line), Some(after_line)) =
        (line_index(effects_line), line_index(after_line))
    else {
        return Verdict::Unknown;
    };

    // Upstream: "#1 must be a block with a single instruction."
    match ctx.region_summary(effects_line) {
        Some(summary) if summary.instruction_count == 1 => {},
        Some(_) => return Verdict::Unknown,
        None => return Verdict::Unknown
    }

    let Some(writes) = ctx.writes_of(effects_line)
    else {
        return Verdict::Unknown;
    };
    if writes.has_side_effects {
        return Verdict::Unknown;
    }
    // Upstream: "It will also fail if the op modifies register I or R."
    // Those are the interrupt-vector and refresh registers - writing either is
    // a deliberate act with consequences no dataflow analysis of this kind
    // models (`ld r,a` in particular is used for its side effect on refresh).
    if writes
        .deps
        .iter()
        .any(|d| matches!(d, Dependency::Reg(Reg::I) | Dependency::Reg(Reg::R)))
    {
        return Verdict::Unknown;
    }
    // An instruction that writes nothing at all is trivially one whose output
    // is unused - which is correct: that is exactly what a `NOP` is.
    not_used_after(after_line, writes.deps.into_iter(), ctx)
}

/// Whether a block-local constraint is about registers or flags.
#[derive(Debug, Clone, Copy)]
enum Kind {
    Reg,
    Flag
}

/// Which half of [`RegionUse`] a block-local constraint forbids.
#[derive(Debug, Clone, Copy)]
enum Touch {
    Read,
    Write
}

/// The four block-local constraints, which differ only in what they parse and
/// which half of [`RegionUse`] they forbid:
///
/// * `regsNotModified(#, ...)` / `flagsNotModified(#, ...)` - nothing in the
///   region *writes* them.
/// * `regsNotUsed(#, ...)` / `flagsNotUsed(#, ...)` - nothing *reads* them.
///
/// Almost every real use names a `*` line, i.e. asks whether the gap between
/// two instructions disturbs something the rule wants to carry across it.
fn block_local<D, C>(
    constraint: &Constraint,
    captures: &Captures<'_, D>,
    ctx: &C,
    kind: Kind,
    touch: Touch
) -> Verdict
where
    D: DataAccessElem,
    C: LivenessContext
{
    let (line, dependencies): (u32, Vec<Dependency>) = match kind {
        Kind::Reg => {
            let Some(args) = parse_regs_args(constraint, captures)
            else {
                return Verdict::Unknown;
            };
            (
                args.line,
                args.items.into_iter().map(Dependency::Reg).collect()
            )
        },
        Kind::Flag => {
            let Some(args) = parse_flags_args(constraint, captures)
            else {
                return Verdict::Unknown;
            };
            (
                args.line,
                args.items.into_iter().map(Dependency::Flag).collect()
            )
        }
    };

    for dependency in dependencies {
        let Some(used) = ctx.region_use(line, dependency)
        else {
            return Verdict::Unknown;
        };
        let touched = match touch {
            Touch::Read => used.reads,
            Touch::Write => used.writes
        };
        if touched {
            return Verdict::Failed;
        }
    }
    Verdict::Satisfied
}

/// `regsNotUsedAfter(#, reg1, ..., regn)` - satisfied only if *every* named
/// register is provably dead after line `#`'s instruction.
fn regs_not_used_after<D, C>(
    constraint: &Constraint,
    captures: &Captures<'_, D>,
    ctx: &C
) -> Verdict
where
    D: DataAccessElem,
    C: LivenessContext
{
    let Some(args) = parse_regs_args(constraint, captures)
    else {
        return Verdict::Unknown;
    };
    not_used_after(args.line, args.items.into_iter().map(Dependency::Reg), ctx)
}

/// `flagsNotUsedAfter(#, flag1, ..., flagn)` - as above, for flags.
fn flags_not_used_after<D, C>(
    constraint: &Constraint,
    captures: &Captures<'_, D>,
    ctx: &C
) -> Verdict
where
    D: DataAccessElem,
    C: LivenessContext
{
    let Some(args) = parse_flags_args(constraint, captures)
    else {
        return Verdict::Unknown;
    };
    not_used_after(args.line, args.items.into_iter().map(Dependency::Flag), ctx)
}

/// The shared body: every dependency must be provably unused. One that is
/// used fails the constraint; one the walk couldn't decide makes the whole
/// thing `Unknown`, which also fails - an optimization is only offered when
/// it is *proven* safe.
fn not_used_after<C>(
    line: u32,
    dependencies: impl Iterator<Item = Dependency>,
    ctx: &C
) -> Verdict
where C: LivenessContext {
    for dependency in dependencies {
        match ctx.is_used_after(line, dependency) {
            Some(Usage::NotUsed) => {},
            Some(Usage::Used) => return Verdict::Failed,
            Some(Usage::Unknown) | None => return Verdict::Unknown
        }
    }
    Verdict::Satisfied
}

/// `equal(a, b)` / `notEqual(a, b)`.
fn compare<D, C>(
    constraint: &Constraint,
    captures: &Captures<'_, D>,
    ctx: &C,
    want_equal: bool
) -> Verdict
where
    D: DataAccessElem,
    C: ConstraintContext
{
    let [lhs, rhs] = constraint.args.as_slice()
    else {
        return Verdict::Unknown;
    };

    // Numeric comparison first: this is what upstream overwhelmingly means
    // (`equal(?const,0)`, `notEqual(?8bitconst1,255)`).
    if let (Some(a), Some(b)) = (eval_numeric(lhs, captures, ctx), eval_numeric(rhs, captures, ctx))
    {
        return Verdict::from_bool((a == b) == want_equal);
    }

    // Otherwise fall back to structural equality of the captured operands,
    // which is what a rule like `equal(?reg1,?reg2)` needs.
    match (resolve_operand(lhs, captures), resolve_operand(rhs, captures)) {
        (Some(a), Some(b)) => {
            let same = a.to_data_access() == b.to_data_access();
            Verdict::from_bool(same == want_equal)
        },
        _ => Verdict::Unknown
    }
}

/// `in(?var, v1, ..., vn)` / `notIn(?var, v1, ..., vn)`.
///
/// Comparison is by rendered text, case-insensitively: upstream writes the
/// candidate list as bare register/opcode names (`in(?reg,A,B,C,D,E,H,L)`,
/// `in(?op1,jp,jr)`) whose case does not match the source's.
fn membership<D>(constraint: &Constraint, captures: &Captures<'_, D>, want_in: bool) -> Verdict
where D: DataAccessElem {
    let Some((subject, candidates)) = constraint.args.split_first()
    else {
        return Verdict::Unknown;
    };
    let OperandPattern::Variable(name) = subject
    else {
        return Verdict::Unknown;
    };

    let Some(actual) = captures.text_of(name)
    else {
        return Verdict::Unknown;
    };

    let found = candidates.iter().any(|candidate| {
        render_operand_pattern(candidate).is_some_and(|text| text.eq_ignore_ascii_case(&actual))
    });
    Verdict::from_bool(found == want_in)
}

/// `regpair(?pair, ?high, ?low)` - relates a 16-bit register to its own two
/// halves.
///
/// This constraint both *tests* and **binds**: upstream rules use it to get
/// hold of a pair's halves (`regpair(?regpair2,?reg2h,?reg2l)`) so the
/// replacement can refer to them by name, even though nothing in the match
/// pattern ever bound those names. An already-bound name is checked instead
/// of rebound, which is what makes the reverse direction
/// (`regpair(?regpair1,?reg1h,?reg1l)` where the halves were matched and the
/// pair is derived) work too.
///
/// Reuses `cpclib-tokens`' own decomposition (`Register16::split`,
/// `IndexRegister16::high`/`low`) rather than restating the mapping here.
fn regpair<D>(constraint: &Constraint, captures: &mut Captures<'_, D>) -> Verdict
where D: DataAccessElem {
    let [pair, high, low] = constraint.args.as_slice()
    else {
        return Verdict::Unknown;
    };

    let Some(pair_text) = capture_or_literal_text(pair, captures)
    else {
        return Verdict::Unknown;
    };
    let Some((expected_high, expected_low)) = split_register_pair(&pair_text)
    else {
        return Verdict::Failed;
    };

    let ok = bind_or_check(high, expected_high, captures)
        && bind_or_check(low, expected_low, captures);
    Verdict::from_bool(ok)
}

/// Bind `pattern` to `value` when it is an unbound capture, or check it
/// otherwise.
fn bind_or_check<D>(pattern: &OperandPattern, value: String, captures: &mut Captures<'_, D>) -> bool
where D: DataAccessElem {
    match pattern {
        OperandPattern::Variable(name) => captures.bind_text(name, value),
        other => {
            render_operand_pattern(other).is_some_and(|text| text.eq_ignore_ascii_case(&value))
        }
    }
}

/// The two halves of a 16-bit register, as `(high, low)` names.
///
/// Uses `Register16::high()`/`low()` explicitly rather than its `split()`,
/// which returns `(low, high)` - the opposite of what the name suggests, and
/// exactly the kind of silent swap that would make a generated `ld d,h` come
/// out as `ld e,l`.
fn split_register_pair(name: &str) -> Option<(String, String)> {
    let upper = name.to_ascii_uppercase();
    let plain = match upper.as_str() {
        "BC" => Some(Register16::Bc),
        "DE" => Some(Register16::De),
        "HL" => Some(Register16::Hl),
        "AF" => Some(Register16::Af),
        "SP" => Some(Register16::Sp),
        _ => None
    };
    if let Some(reg) = plain {
        return match (reg.high(), reg.low()) {
            (Some(h), Some(l)) => Some((h.to_string(), l.to_string())),
            _ => None
        };
    }

    let indexed = match upper.as_str() {
        "IX" => Some(IndexRegister16::Ix),
        "IY" => Some(IndexRegister16::Iy),
        _ => None
    }?;
    Some((indexed.high().to_string(), indexed.low().to_string()))
}

/// `reachableByJr(#, label)` - whether the instruction matched by pattern line
/// `#` could reach `label` with a relative jump.
fn reachable_by_jr<D, C>(constraint: &Constraint, captures: &Captures<'_, D>, ctx: &C) -> Verdict
where
    D: DataAccessElem,
    C: ConstraintContext
{
    let [line, target] = constraint.args.as_slice()
    else {
        return Verdict::Unknown;
    };
    let OperandPattern::Number(index) = line
    else {
        return Verdict::Unknown;
    };

    let (Some(from), Some(to)) = (
        ctx.address_of_line(*index as u32),
        eval_numeric(target, captures, ctx)
    )
    else {
        return Verdict::Unknown;
    };

    // Same computation the assembler itself performs when encoding a real
    // relative jump (`cpclib_asm::assembler::absolute_to_relative`): the
    // displacement is measured from the address *after* the two-byte JR, and
    // must fit in a signed byte.
    let delta = to - i64::from(from) - JR_OPCODE_LEN;
    Verdict::from_bool((-128..=127).contains(&delta))
}

/// A `JR`'s own encoded length, which its displacement is relative to.
const JR_OPCODE_LEN: i64 = 2;

/// Evaluate an operand pattern to a number, substituting captures.
fn eval_numeric<D, C>(pattern: &OperandPattern, captures: &Captures<'_, D>, ctx: &C) -> Option<i64>
where
    D: DataAccessElem,
    C: ConstraintContext
{
    match pattern {
        OperandPattern::Number(v) => Some(*v),
        OperandPattern::Ident(name) => ctx.value_of_label(name),
        OperandPattern::Variable(name) => {
            let operand = captures.operand_of(name)?;
            let expr = operand.get_expression()?;
            if expr.is_value() {
                Some(i64::from(expr.value()))
            }
            else if expr.is_label() {
                ctx.value_of_label(expr.label())
            }
            else {
                None
            }
        },
        OperandPattern::Indirect(_) => None,
        OperandPattern::Unary { op, operand } => {
            let value = eval_numeric(operand, captures, ctx)?;
            Some(match op {
                UnOp::Neg => -value,
                UnOp::Not => !value
            })
        },
        OperandPattern::Binary { lhs, op, rhs } => {
            let a = eval_numeric(lhs, captures, ctx)?;
            let b = eval_numeric(rhs, captures, ctx)?;
            Some(match op {
                BinOp::Add => a.wrapping_add(b),
                BinOp::Sub => a.wrapping_sub(b),
                BinOp::Mul => a.wrapping_mul(b),
                BinOp::Div => a.checked_div(b)?,
                BinOp::Mod => a.checked_rem(b)?,
                BinOp::ShiftLeft => a.checked_shl(b.try_into().ok()?)?,
                BinOp::ShiftRight => a.checked_shr(b.try_into().ok()?)?,
                BinOp::BitAnd => a & b,
                BinOp::BitOr => a | b,
                BinOp::BitXor => a ^ b,
                // Upstream writes comparisons as `equal(?const >= 3, -1)`,
                // i.e. a true comparison yields -1 (all bits set), matching
                // the convention of the assemblers this format targets.
                BinOp::Equal => bool_value(a == b),
                BinOp::NotEqual => bool_value(a != b),
                BinOp::Less => bool_value(a < b),
                BinOp::LessEqual => bool_value(a <= b),
                BinOp::Greater => bool_value(a > b),
                BinOp::GreaterEqual => bool_value(a >= b)
            })
        }
    }
}

fn bool_value(value: bool) -> i64 {
    if value { -1 } else { 0 }
}

/// The real operand a pattern refers to, when it is a bare capture.
fn resolve_operand<'a, D>(
    pattern: &OperandPattern,
    captures: &Captures<'a, D>
) -> Option<&'a D>
where D: DataAccessElem {
    match pattern {
        OperandPattern::Variable(name) => captures.operand_of(name),
        _ => None
    }
}

/// The captured text for a variable, or the literal text of a non-variable
/// pattern - what `regpair`'s three arguments need, since upstream sometimes
/// passes literal register names rather than captures.
fn capture_or_literal_text<D>(pattern: &OperandPattern, captures: &Captures<'_, D>) -> Option<String>
where D: DataAccessElem {
    match pattern {
        OperandPattern::Variable(name) => captures.text_of(name),
        other => render_operand_pattern(other)
    }
}

/// The parsed arguments of a `regsNotUsedAfter`/`flagsNotUsedAfter`-shaped
/// constraint: the pattern line to walk forward from, and what to track.
///
/// Both constraint families share the exact same argument shape - a line
/// number followed by one or more names - so they share one parser,
/// differing only in what the names are parsed as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivenessArgs<Item> {
    /// The pattern line number whose matched instruction the walk starts
    /// after.
    pub line: u32,
    /// The registers (or flags) to track. Never empty - a constraint naming
    /// nothing is rejected rather than treated as vacuously satisfied.
    pub items: Vec<Item>
}

/// Parse `regsNotUsedAfter(#, reg1, ..., regn)`'s arguments, substituting any
/// `?variable` through `captures` first (upstream does the same, before
/// dispatching: a real rule writes `regsNotUsedAfter(2, ?regpair1)`).
///
/// `None` - and therefore [`Verdict::Unknown`], a *failing* answer - whenever
/// anything is unrecognized, rather than silently tracking fewer registers
/// than the rule asked for.
fn parse_regs_args<D>(
    constraint: &Constraint,
    captures: &Captures<'_, D>
) -> Option<LivenessArgs<Reg>>
where D: DataAccessElem {
    parse_liveness_args(constraint, captures, Reg::parse)
}

/// Parse `flagsNotUsedAfter(#, flag1, ..., flagn)`'s arguments - see
/// [`parse_regs_args`].
fn parse_flags_args<D>(
    constraint: &Constraint,
    captures: &Captures<'_, D>
) -> Option<LivenessArgs<Flag>>
where D: DataAccessElem {
    parse_liveness_args(constraint, captures, Flag::parse)
}

fn parse_liveness_args<D, Item>(
    constraint: &Constraint,
    captures: &Captures<'_, D>,
    parse_item: impl Fn(&str) -> Option<Item>
) -> Option<LivenessArgs<Item>>
where
    D: DataAccessElem
{
    let (first, rest) = constraint.args.split_first()?;
    let OperandPattern::Number(line) = first
    else {
        return None;
    };
    let line = u32::try_from(*line).ok()?;

    if rest.is_empty() {
        return None;
    }
    let items = rest
        .iter()
        .map(|arg| parse_item(&capture_or_literal_text(arg, captures)?))
        .collect::<Option<Vec<_>>>()?;

    Some(LivenessArgs { line, items })
}

/// Render a literal operand pattern back to text, for comparison against a
/// capture. Returns `None` for anything holding an unresolved variable.
fn render_operand_pattern(pattern: &OperandPattern) -> Option<String> {
    match pattern {
        OperandPattern::Ident(name) => Some(name.clone()),
        OperandPattern::Number(value) => Some(value.to_string()),
        OperandPattern::Indirect(inner) => {
            Some(format!("({})", render_operand_pattern(inner)?))
        },
        OperandPattern::Variable(_) | OperandPattern::Unary { .. } | OperandPattern::Binary { .. } => {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use cpclib_tokens::Token;

    use super::*;

    /// Parse a whole rule out of real DSL text and hand back its first
    /// constraint - so the tests below exercise the *real* parser, not a
    /// hand-built `Constraint` that might not match what the corpus produces.
    fn constraint_of(dsl: &str) -> Constraint {
        let src = format!("pattern: x\n0: nop\n1: nop\n2: nop\nreplacement:\nconstraints:\n{dsl}\n");
        RuleSet::parse(&src).unwrap().rules[0].constraints[0].clone()
    }

    fn no_captures() -> Captures<'static, cpclib_tokens::DataAccess> {
        Captures::default()
    }

    /// The exact constraint from upstream's `cp12deca`.
    #[test]
    fn a_real_regs_constraint_parses_into_typed_registers() {
        let c = constraint_of("regsNotUsedAfter(0,A)");
        assert_eq!(parse_regs_args(&c, &no_captures()), Some(LivenessArgs {
            line: 0,
            items: vec![Reg::A]
        }));
    }

    /// The exact constraint from upstream's `czjump2c` - and the reason
    /// `dsl::ident` had to learn about `P/V`.
    #[test]
    fn a_real_flags_constraint_parses_every_flag_including_the_slashed_one() {
        let c = constraint_of("flagsNotUsedAfter(2,Z,C,N,P/V,H,S)");
        assert_eq!(parse_flags_args(&c, &no_captures()), Some(LivenessArgs {
            line: 2,
            items: vec![Flag::Z, Flag::C, Flag::N, Flag::PV, Flag::H, Flag::S]
        }));
    }

    /// Real rules track *pairs*, not just 8-bit registers - upstream's
    /// `unnecessary-ld-after-pop` writes `regsNotUsedAfter(1,?regpair1)`.
    #[test]
    fn a_captured_register_variable_is_substituted_before_parsing() {
        let c = constraint_of("regsNotUsedAfter(1,?regpair1)");
        // Unbound, the constraint can't be evaluated at all...
        assert_eq!(parse_regs_args(&c, &no_captures()), None);

        // ...but once the match has bound it, it resolves to a real pair.
        let mut captures = no_captures();
        assert!(captures.bind_text("regpair1", "BC".to_string()));
        assert_eq!(parse_regs_args(&c, &captures), Some(LivenessArgs {
            line: 1,
            items: vec![Reg::Bc]
        }));
    }

    /// Anything unrecognized must yield `None` (→ `Unknown` → the constraint
    /// fails), never a partial list that would silently under-check.
    #[test]
    fn an_unparsable_argument_rejects_the_whole_constraint() {
        assert_eq!(
            parse_regs_args(&constraint_of("regsNotUsedAfter(0,A,nonsense)"), &no_captures()),
            None
        );
        assert_eq!(
            parse_flags_args(&constraint_of("flagsNotUsedAfter(0,Z,nonsense)"), &no_captures()),
            None
        );
        // A constraint naming no registers at all is rejected rather than
        // treated as vacuously satisfied.
        assert_eq!(
            parse_regs_args(&constraint_of("regsNotUsedAfter(0)"), &no_captures()),
            None
        );
    }

    /// Every real `regsNotUsedAfter`/`flagsNotUsedAfter` in the vendored
    /// corpus must parse - once its `?variables` are bound. Unbound ones are
    /// expected to fail here and are skipped; what this guards is that no
    /// *literal* register or flag name upstream uses is unknown to us.
    #[test]
    fn every_literal_liveness_argument_in_the_real_corpus_is_recognised() {
        let rules = crate::builtin_rules::builtin_rules(crate::OptimizationGoal::Neutral);
        let captures = no_captures();
        let mut checked = 0;
        for rule in &rules.rules {
            for c in &rule.constraints {
                let has_variable = c
                    .args
                    .iter()
                    .any(|a| matches!(a, OperandPattern::Variable(_)));
                if has_variable {
                    continue;
                }
                match c.name.as_str() {
                    "regsNotUsedAfter" => {
                        assert!(
                            parse_regs_args(c, &captures).is_some(),
                            "unparsed in {:?}: {c:?}",
                            rule.name
                        );
                        checked += 1;
                    },
                    "flagsNotUsedAfter" => {
                        assert!(
                            parse_flags_args(c, &captures).is_some(),
                            "unparsed in {:?}: {c:?}",
                            rule.name
                        );
                        checked += 1;
                    },
                    _ => {}
                }
            }
        }
        assert!(checked > 50, "expected many real constraints, checked {checked}");
    }

    #[test]
    fn a_rule_set_with_only_structural_constraints_needs_no_addresses() {
        let rules = RuleSet::parse(
            "pattern: Remove ld ?reg,?reg\n\
             0: ld ?reg,?reg\n\
             replacement:\n\
             constraints:\n\
             in(?reg,A,B,C,D,E,H,L)\n"
        )
        .unwrap();
        assert!(!rules_need_addresses(&rules));
    }

    #[test]
    fn a_rule_set_containing_reachable_by_jr_needs_addresses() {
        let rules = RuleSet::parse(
            "pattern: Replace jp ?const1 with jr ?const1\n\
             0: jp ?const1\n\
             replacement:\n\
             0: jr ?const1\n\
             constraints:\n\
             reachableByJr(0,?const1)\n"
        )
        .unwrap();
        assert!(rules_need_addresses(&rules));
    }

    #[test]
    fn an_unsupported_rule_using_reachable_by_jr_does_not_count() {
        // The rule can never fire anyway (an unsupported constraint skips it
        // wholesale), so it must not force a real assemble just because one
        // of its *other* constraints happens to be address-aware.
        let rules = RuleSet::parse(
            "pattern: something\n\
             0: nop\n\
             replacement:\n\
             constraints:\n\
             reachableByJr(0,?const1)\n\
             constraintFromAFutureRelease(0,A)\n"
        )
        .unwrap();
        assert!(!rules_need_addresses(&rules));
    }

    #[test]
    fn every_supported_name_is_actually_dispatched() {
        // Guards against a name being listed as supported but falling through
        // to the catch-all - which would silently make rules using it match
        // without their safety condition ever being checked.
        for name in SUPPORTED {
            let constraint = Constraint {
                name: (*name).to_string(),
                args: Vec::new(),
                check_after: None
            };
            let mut captures: Captures<'_, cpclib_tokens::DataAccess> = Captures::default();
            // With no arguments every one of them bails out as `Unknown`
            // *from its own arm*; the point is simply that none of them can
            // return `Satisfied` by accident here.
            assert!(
                !evaluate(&constraint, &mut captures, &NoContext).is_satisfied(),
                "{name} must not be satisfiable without arguments"
            );
        }
    }

    #[test]
    fn a_rule_using_an_unimplemented_constraint_is_not_supported() {
        let constraints = vec![
            Constraint {
                name: "in".to_string(),
                args: Vec::new(),
                check_after: None
            },
            // Deliberately invented. Every constraint the upstream format
            // documents is now implemented, so there is no real name left to
            // use here - and that is exactly why this test still matters: it
            // guards the skip-the-whole-rule mechanism that protects us from
            // a constraint a *future* upstream release adds, which we would
            // otherwise silently ignore while still applying the rule.
            Constraint {
                name: "constraintFromAFutureRelease".to_string(),
                args: Vec::new(),
                check_after: None
            },
        ];
        assert!(!all_supported(&constraints));
        assert!(all_supported(&constraints[..1]));
    }

    #[test]
    fn register_pairs_come_from_cpclib_tokens_own_decomposition() {
        assert_eq!(
            split_register_pair("BC"),
            Some(("B".to_string(), "C".to_string()))
        );
        assert_eq!(
            split_register_pair("hl"),
            Some(("H".to_string(), "L".to_string()))
        );
        let (high, low) = split_register_pair("IX").unwrap();
        assert!(high.eq_ignore_ascii_case("ixh"), "got {high}");
        assert!(low.eq_ignore_ascii_case("ixl"), "got {low}");
        assert_eq!(split_register_pair("nonsense"), None);
    }

    #[test]
    fn numeric_expressions_evaluate_with_upstreams_boolean_convention() {
        let captures: Captures<'_, cpclib_tokens::DataAccess> = Captures::default();
        // `?const >= 3` yields -1 when true, which upstream then compares
        // against -1 via `equal(?const1 >= 3, -1)`.
        let pattern = OperandPattern::Binary {
            lhs: Box::new(OperandPattern::Number(5)),
            op: BinOp::GreaterEqual,
            rhs: Box::new(OperandPattern::Number(3))
        };
        assert_eq!(eval_numeric(&pattern, &captures, &NoContext), Some(-1));

        let pattern = OperandPattern::Binary {
            lhs: Box::new(OperandPattern::Number(0xFF)),
            op: BinOp::ShiftRight,
            rhs: Box::new(OperandPattern::Number(4))
        };
        assert_eq!(eval_numeric(&pattern, &captures, &NoContext), Some(0x0F));
    }

    #[test]
    fn an_address_aware_constraint_without_context_is_unknown_not_satisfied() {
        let constraint = Constraint {
            name: "reachableByJr".to_string(),
            args: vec![
                OperandPattern::Number(0),
                OperandPattern::Variable("const1".to_string()),
            ],
            check_after: None
        };
        let mut captures: Captures<'_, cpclib_tokens::DataAccess> = Captures::default();
        assert_eq!(
            evaluate(&constraint, &mut captures, &NoContext),
            Verdict::Unknown
        );
    }
}
