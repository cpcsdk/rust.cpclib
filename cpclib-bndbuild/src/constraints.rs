use cpclib_common::winnow::ascii::space0;
use cpclib_common::winnow::combinator::{alt, delimited};
use cpclib_common::winnow::token::tag_no_case;
use cpclib_common::winnow::{self, PResult, Parser};
use serde::{self, Deserialize, Deserializer};

#[derive(Debug, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum Constraint {
    Windows,
    Linux,
    MacOsx,
    // And(Box<Constraint>, Box<Constraint>),
    // Or(Box<Constraint>, Box<Constraint>),
    Not(Box<Constraint>)
}

// TODO Implement the other stuff
pub(crate) fn deserialize_constraint<'de, D>(
    deserializer: D
) -> Result<Option<Constraint>, D::Error>
where D: Deserializer<'de> {
    let s = String::deserialize(deserializer)?;
    let cons = parse_constraint
        .parse(&s)
        .map_err(|e| serde::de::Error::custom(e.to_string()))?;

    Ok(Some(cons))
}

fn parse_constraint(input: &mut &str) -> PResult<Constraint> {
    alt((parse_negated_constraint, parse_positive_constraint)).parse_next(input)
}

fn parse_negated_constraint(input: &mut &str) -> PResult<Constraint> {
    delimited(
        (tag_no_case("not("), space0),
        parse_positive_constraint,
        (space0, ')', space0)
    )
    .parse_next(input)
}

fn parse_positive_constraint(input: &mut &str) -> PResult<Constraint> {
    parse_leaf_constraint.parse_next(input)
}

fn parse_os_constraint(input: &mut &str) -> PResult<Constraint> {
    tag_no_case("os").parse_next(input)?;
    delimited(
        ('(', space0),
        alt((
            tag_no_case("windows").value(Constraint::Windows),
            tag_no_case("linux").value(Constraint::Linux),
            tag_no_case("macos").value(Constraint::MacOsx)
        )),
        (space0, ')', space0)
    )
    .parse_next(input)
}

fn parse_leaf_constraint(input: &mut &str) -> PResult<Constraint> {
    parse_os_constraint.parse_next(input)
}

impl Constraint {
    pub fn corresponds(&self) -> bool {
        match self {
            Constraint::Windows => {
                if cfg!(target_os = "windows") {
                    true
                }
                else {
                    false
                }
            }
            Constraint::Linux => {
                if cfg!(target_os = "linux") {
                    true
                }
                else {
                    false
                }
            }
            Constraint::MacOsx => {
                if cfg!(target_os = "macos") {
                    true
                }
                else {
                    false
                }
            }
            Constraint::Not(c) => !c.corresponds()
            // 		Constraint::And(a, b) => a.corresponds() && b.corresponds(),
            // 		Constraint::Or(a, b) => a.corresponds() || b.corresponds(),
        }
    }
}
