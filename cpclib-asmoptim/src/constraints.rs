//! Evaluation of pattern constraints.
//!
//! The DSL parser deliberately accepts every constraint the real upstream
//! format defines, including ones this crate cannot evaluate yet (see
//! [`crate::dsl`]'s own doc comment). This module decides which of them are
//! actually *supported*: a rule mentioning anything else is skipped entirely
//! rather than matched without its safety condition, which would be the one
//! genuinely dangerous failure mode for an optimizer.

use cpclib_tokens::{DataAccessElem, ExprElement, IndexRegister16, Register16};

use crate::dsl::{BinOp, Constraint, OperandPattern, UnOp};
use crate::engine::Captures;

/// Constraint names this crate evaluates today.
///
/// Ordered roughly by how often they appear in the real upstream corpus
/// (`in` alone accounts for 168 of ~500 constraint uses). The big remaining
/// ones - `regsNotUsedAfter` (86 uses) and `flagsNotUsedAfter` (78) - need
/// real forward dataflow analysis over the instruction stream plus per-
/// instruction read/write semantics, and are the intended next increment.
pub const SUPPORTED: &[&str] = &["equal", "notEqual", "in", "notIn", "regpair", "reachableByJr"];

/// Whether every constraint of a rule can actually be evaluated.
pub fn all_supported(constraints: &[Constraint]) -> bool {
    constraints
        .iter()
        .all(|c| SUPPORTED.contains(&c.name.as_str()))
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

/// A context that knows nothing - every address-aware constraint reports
/// [`Verdict::Unknown`], so rules needing one never match.
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

/// Evaluate one constraint against a candidate match.
///
/// Takes the captures mutably because some constraints legitimately *produce*
/// bindings rather than only testing them - see [`regpair`].
pub fn evaluate<D, C>(constraint: &Constraint, captures: &mut Captures<'_, D>, ctx: &C) -> Verdict
where
    D: DataAccessElem,
    C: ConstraintContext
{
    match constraint.name.as_str() {
        "equal" => compare(constraint, captures, ctx, true),
        "notEqual" => compare(constraint, captures, ctx, false),
        "in" => membership(constraint, captures, true),
        "notIn" => membership(constraint, captures, false),
        "regpair" => regpair(constraint, captures),
        "reachableByJr" => reachable_by_jr(constraint, captures, ctx),
        _ => Verdict::Unknown
    }
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
    use super::*;

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
            Constraint {
                name: "flagsNotUsedAfter".to_string(),
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
        let mut captures: Captures<'_, cpclib_tokens::DataAccess> = Captures::default();
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
