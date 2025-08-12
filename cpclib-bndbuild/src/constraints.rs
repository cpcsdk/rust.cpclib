use std::fmt::Display;
use std::ops::Deref;

use cpclib_common::itertools::Itertools;
use cpclib_common::winnow::ascii::{Caseless, alphanumeric1, space0};
use cpclib_common::winnow::combinator::{alt, delimited, repeat, separated, terminated};
use cpclib_common::winnow::{ModalResult, Parser};
use serde::{self, Deserialize, Deserializer};

#[derive(Debug, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum Os {
    Windows,
    Linux,
    MacOsx
}

pub trait Corresponds {
    fn corresponds(&self) -> bool;
}

impl Corresponds for Os {
    fn corresponds(&self) -> bool {
        match self {
            Os::Windows => {
                if cfg!(target_os = "windows") {
                    true
                }
                else {
                    false
                }
            },
            Os::Linux => {
                if cfg!(target_os = "linux") {
                    true
                }
                else {
                    false
                }
            },
            Os::MacOsx => {
                if cfg!(target_os = "macos") {
                    true
                }
                else {
                    false
                }
            },
        }
    }
}

impl Display for Os {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            Os::Windows => "windows".to_owned(),
            Os::Linux => "linux".to_owned(),
            Os::MacOsx => "macos".to_owned()
        };
        write!(f, "{repr}")
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Hostname(String);

impl Corresponds for Hostname {
    fn corresponds(&self) -> bool {
        hostname::get()
            .map(|h| h.display().to_string().eq_ignore_ascii_case(&self.0))
            .unwrap_or(false)
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum LogicalExpression {
    And(Vec<Constraint>),
    Or(Vec<Constraint>),
    Not(Box<Constraint>)
}

impl Display for LogicalExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (code, elems) = match self {
            LogicalExpression::And(vec) => ("and", vec.as_slice()),
            LogicalExpression::Or(vec) => ("or", vec.as_slice()),
            LogicalExpression::Not(constraint) => ("not", std::slice::from_ref(constraint.deref()))
        };

        write!(
            f,
            "{code}({})",
            elems.iter().map(|c| c.to_string()).join(",")
        )
    }
}

impl Corresponds for LogicalExpression {
    fn corresponds(&self) -> bool {
        match self {
            LogicalExpression::And(vec) => vec.iter().all(|c| c.corresponds()),
            LogicalExpression::Or(vec) => vec.iter().any(|c| c.corresponds()),
            LogicalExpression::Not(c) => !c.corresponds()
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum Constraint {
    Os(Os),
    Hostname(Hostname),
    LogicalExpression(LogicalExpression)
}

impl From<LogicalExpression> for Constraint {
    fn from(value: LogicalExpression) -> Self {
        Self::LogicalExpression(value)
    }
}

impl Display for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Constraint::Os(os) => format!("os({os})"),
            Constraint::Hostname(c) => format!("hostname({})", c.0),
            Constraint::LogicalExpression(c) => format!("{c}")
        };
        write!(f, "{text}")
    }
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

fn parse_constraint(input: &mut &str) -> ModalResult<Constraint> {
    alt((parse_logical_constraint, parse_positive_constraint)).parse_next(input)
}

fn parse_negated_constraint(input: &mut &str) -> ModalResult<LogicalExpression> {
    delimited(
        (Caseless("not("), space0),
        parse_constraint,
        (space0, ')', space0)
    )
    .map(|c| LogicalExpression::Not(Box::new(c)))
    .parse_next(input)
}

fn parse_positive_constraint(input: &mut &str) -> ModalResult<Constraint> {
    alt((
        parse_and_or_constraint.map(Constraint::from),
        parse_leaf_constraint
    ))
    .parse_next(input)
}

fn parse_and_or_constraint(input: &mut &str) -> ModalResult<LogicalExpression> {
    #[derive(Clone)]
    enum Logic {
        And,
        Or
    }

    (
        terminated(
            alt((
                Caseless("or").value(Logic::Or),
                Caseless("and").value(Logic::And)
            )),
            ("(", space0)
        ),
        separated(2.., parse_constraint, (space0, ",", space0)),
        (space0, ")", space0).value(())
    )
        .map(|(l, c, _)| {
            match l {
                Logic::And => LogicalExpression::And(c),
                Logic::Or => LogicalExpression::Or(c)
            }
        })
        .parse_next(input)
}

fn parse_logical_constraint(input: &mut &str) -> ModalResult<Constraint> {
    alt((parse_negated_constraint, parse_and_or_constraint))
        .map(Constraint::from)
        .parse_next(input)
}

fn parse_os_constraint(input: &mut &str) -> ModalResult<Constraint> {
    Caseless("os").parse_next(input)?;
    delimited(
        ('(', space0),
        alt((
            Caseless("windows").value(Os::Windows),
            Caseless("linux").value(Os::Linux),
            alt((Caseless("macosx"), Caseless("macos"))).value(Os::MacOsx)
        )),
        (space0, ')', space0)
    )
    .map(Constraint::Os)
    .parse_next(input)
}

fn parse_hostname_constraint(input: &mut &str) -> ModalResult<Constraint> {
    delimited(
        (Caseless("hostname("), space0),
        repeat(1.., alt((alphanumeric1, "_", "-"))),
        (space0, ")", space0)
    )
    .map(|txt: String| Constraint::Hostname(Hostname(txt.to_string())))
    .parse_next(input)
}

fn parse_leaf_constraint(input: &mut &str) -> ModalResult<Constraint> {
    alt((parse_hostname_constraint, parse_os_constraint)).parse_next(input)
}

impl Corresponds for Constraint {
    fn corresponds(&self) -> bool {
        match self {
            Constraint::Os(os) => os.corresponds(),
            Constraint::Hostname(host) => host.corresponds(),
            Constraint::LogicalExpression(c) => c.corresponds()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::constraints::Corresponds;
    #[test]
    fn test_hostname() {
        let s = "hostname(FAKE)".to_string();
        let c = parse_hostname_constraint.parse(&s).unwrap();
        assert!(!c.corresponds());

        let s = "not(hostname(FAKE))".to_string();
        let c = parse_logical_constraint.parse(&s).unwrap();
        assert!(c.corresponds());

        let s = format!("hostname({})", hostname::get().unwrap().display());
        let c = parse_hostname_constraint.parse(&s).unwrap();
        assert!(c.corresponds());
    }

    #[test]
    fn test_os() {
        let s = "os(windows)".to_string();
        parse_os_constraint.parse(&s).unwrap();

        let s = "os(linux)".to_string();
        parse_os_constraint.parse(&s).unwrap();

        let s = "os(macosx)".to_string();
        parse_os_constraint.parse(&s).unwrap();
    }

    #[test]
    fn test_logical_expression() {
        let s: String = "and(os(windows), os(linux), os(macosx))".to_string();
        let c = parse_logical_constraint.parse(&s).unwrap();
        assert!(!c.corresponds());

        let s: String = "or(os(windows), os(linux), os(macosx))".to_string();
        let c = parse_logical_constraint.parse(&s).unwrap();
        assert!(c.corresponds());

        let s = "HOSTNAME(ELIOT1)".to_owned();
        let c = parse_hostname_constraint.parse(&s).unwrap();
        assert!(!c.corresponds());

        let s = "OR(HOSTNAME(ELIOT1), HOSTNAME(ELIOT2))".to_owned();
        let c = parse_and_or_constraint.parse(&s).unwrap();
        assert!(!c.corresponds());
        let c = parse_positive_constraint.parse(&s).unwrap();
        assert!(!c.corresponds());

        let s = "NOT(OR(HOSTNAME(ELIOT1), HOSTNAME(ELIOT2)))".to_owned();
        let c = parse_negated_constraint.parse(&s).unwrap();
        assert!(c.corresponds());

        let s = "NOT(OR(HOSTNAME(ELIOT1), HOSTNAME(ELIOT2)))".to_owned();
        let c = parse_logical_constraint.parse(&s).unwrap();
        assert!(c.corresponds());

        let s = "NOT(NOT(OR(HOSTNAME(ELIOT1), HOSTNAME(ELIOT2))))".to_owned();
        let c = parse_logical_constraint.parse(&s).unwrap();
        assert!(!c.corresponds());
    }
}
