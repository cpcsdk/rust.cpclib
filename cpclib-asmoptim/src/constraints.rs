//! Evaluation of pattern constraints.
//!
//! The DSL parser deliberately accepts every constraint the real upstream
//! format defines, including ones this crate cannot evaluate yet (see
//! [`crate::dsl`]'s own doc comment). This module decides which of them are
//! actually *supported*: a rule mentioning anything else is skipped entirely
//! rather than matched without its safety condition, which would be the one
//! genuinely dangerous failure mode for an optimizer.

use cpclib_tokens::{DataAccessElem, ExprElement, IndexRegister16, ListingElement, Register16};

use crate::dependency::Dependency;
use crate::dsl::{BinOp, Constraint, OperandPattern, Rule, RuleSet, UnOp};
use crate::engine::Captures;
use crate::liveness::Usage;
use crate::regflag::{Flag, Reg};

/// Constraint names this crate evaluates today.
///
/// Ordered roughly by how often they appear in the real upstream corpus
/// (`in` alone accounts for 168 uses of ~500, then `regsNotUsedAfter` at 86
/// and `flagsNotUsedAfter` at 78). The remaining unimplemented ones are all
/// far rarer; the biggest group left is the *block-local* family
/// (`regsNotModified`/`regsNotUsed`/`flagsNotModified`/`flagsNotUsed`, 45/13/
/// 11/1 uses), which needs no control-flow walk at all now that
/// [`crate::effects`] exists - just the classifier run over the already
/// matched instructions.
pub const SUPPORTED: &[&str] = &[
    "equal",
    "notEqual",
    "in",
    "notIn",
    "regpair",
    "reachableByJr",
    "regsNotUsedAfter",
    "flagsNotUsedAfter"
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
        _ => Verdict::Unknown
    }
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
             regsNotModified(0,A)\n"
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
            // Still genuinely unimplemented: block-local, 45 real uses.
            Constraint {
                name: "regsNotModified".to_string(),
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
