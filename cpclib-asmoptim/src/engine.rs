//! Matching an instruction stream against optimization patterns.
//!
//! The engine is generic over `ListingElement`, so it runs against whatever
//! token type the caller already has (the LSP passes `LocatedToken`s straight
//! out of its parsed listing). It never mutates anything and never decides
//! *whether* to apply an optimization - it only reports that a rule matched,
//! where, and what the replacement would be.

use std::collections::HashMap;

use cpclib_tokens::{DataAccessElem, ExprElement, ListingElement};

use crate::constraints::{self, ConstraintContext, NoContext};
use crate::dsl::{
    Constraint, InstrPattern, MnemonicPattern, NumberedInstr, OperandPattern, RepeatCount, Rule,
    RuleSet
};

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
#[derive(Debug, Clone)]
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
/// be worse than suggesting nothing.
pub fn find_matches<T>(tokens: &[&T], rules: &RuleSet) -> Vec<PeepholeMatch>
where
    T: ListingElement,
    T::DataAccess: DataAccessElem
{
    find_matches_with_context(tokens, rules, &NoContext)
}

/// As [`find_matches`], with a context supplying real addresses so that
/// address-aware constraints (`reachableByJr`) can be evaluated.
pub fn find_matches_with_context<T, C>(
    tokens: &[&T],
    rules: &RuleSet,
    ctx: &C
) -> Vec<PeepholeMatch>
where
    T: ListingElement,
    T::DataAccess: DataAccessElem,
    C: ConstraintContext
{
    let mut matches = Vec::new();
    let usable: Vec<&Rule> = rules
        .rules
        .iter()
        .filter(|r| constraints::all_supported(&r.constraints))
        .collect();

    let mut start = 0usize;
    while start < tokens.len() {
        let mut advanced = false;
        for rule in &usable {
            if let Some(m) = try_rule(rule, tokens, start, ctx) {
                let next = m.end.max(start + 1);
                matches.push(m);
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

/// Attempt one rule at one starting position.
fn try_rule<T, C>(rule: &Rule, tokens: &[&T], start: usize, ctx: &C) -> Option<PeepholeMatch>
where
    T: ListingElement,
    T::DataAccess: DataAccessElem,
    C: ConstraintContext
{
    let mut captures = Captures::default();
    let mut anchor = None;
    let end = match_lines(
        &rule.match_lines,
        0,
        tokens,
        start,
        &mut captures,
        &mut anchor
    )?;

    // A pattern must cover at least one real instruction; a rule made only of
    // wildcards would otherwise "match" everywhere while meaning nothing.
    if end == start {
        return None;
    }

    let anchor = anchor?;

    for constraint in &rule.constraints {
        if !constraints::evaluate(constraint, &mut captures, ctx).is_satisfied() {
            return None;
        }
    }

    Some(PeepholeMatch {
        rule_name: rule.name.clone(),
        message: substitute_message(&rule.description, &captures),
        start,
        end,
        anchor,
        replacement: render_replacement(&rule.replacement_lines, &captures)
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
    anchor: &mut Option<usize>
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
            for taken in 0..=(tokens.len() - pos) {
                let mut trial = captures.clone();
                let mut trial_anchor = *anchor;
                if line.index == Rule::ANCHOR && trial_anchor.is_none() {
                    trial_anchor = Some(pos);
                }
                if let Some(end) = match_lines(
                    lines,
                    line_idx + 1,
                    tokens,
                    pos + taken,
                    &mut trial,
                    &mut trial_anchor
                ) {
                    *captures = trial;
                    *anchor = trial_anchor;
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
                        None => (1..=(tokens.len() - pos) as u32).rev().collect()
                    }
                }
            };

            for n in repetitions {
                let mut trial = captures.clone();
                let mut trial_anchor = *anchor;
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

                if line.index == Rule::ANCHOR && trial_anchor.is_none() {
                    trial_anchor = Some(pos);
                }
                if let Some(end) = match_lines(
                    lines,
                    line_idx + 1,
                    tokens,
                    cursor,
                    &mut trial,
                    &mut trial_anchor
                ) {
                    *captures = trial;
                    *anchor = trial_anchor;
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
            let mut trial_anchor = *anchor;
            if line.index == Rule::ANCHOR && trial_anchor.is_none() {
                trial_anchor = Some(pos);
            }
            let end = match_lines(
                lines,
                line_idx + 1,
                tokens,
                pos + 1,
                &mut trial,
                &mut trial_anchor
            )?;
            *captures = trial;
            *anchor = trial_anchor;
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

/// Build the replacement text, one entry per replacement line.
fn render_replacement<D>(lines: &[NumberedInstr], captures: &Captures<'_, D>) -> Vec<String>
where D: DataAccessElem {
    lines
        .iter()
        .filter_map(|line| render_instr(&line.instr, captures))
        .collect()
}

fn render_instr<D>(pattern: &InstrPattern, captures: &Captures<'_, D>) -> Option<String>
where D: DataAccessElem {
    match pattern {
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
        OperandPattern::Variable(name) => {
            let text = captures.text_of(name)?;
            // Keywords render in the engine's own canonical lower case (a
            // consumer that cares re-cases the whole suggestion to match the
            // surrounding source); a symbolic operand is returned untouched.
            Some(if captures.is_keyword_binding(name) {
                text.to_ascii_lowercase()
            }
            else {
                text
            })
        },
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
        match captures.text_of(name) {
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
