use cpclib_common::nom::branch::*;
use cpclib_common::nom::bytes::complete::*;
use cpclib_common::nom::character::complete::*;
use cpclib_common::nom::combinator::*;
use cpclib_common::nom::sequence::*;
use cpclib_common::nom::*;
use serde::{self, Deserialize, Deserializer};

#[derive(Debug, Deserialize, PartialEq, Eq, Hash)]
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
    let (input, cons) =
        parse_constraint(&s).map_err(|e| serde::de::Error::custom(e.to_string()))?;

    if input.len() != 0 {
        unimplemented!()
    }

    Ok(Some(cons))
}

fn parse_constraint(input: &str) -> IResult<&str, Constraint> {
    alt((parse_negated_constraint, parse_positive_constraint))(input)
}

fn parse_negated_constraint(input: &str) -> IResult<&str, Constraint> {
    delimited(
        tuple((tag_no_case("not("), space0)),
        parse_positive_constraint,
        tuple((space0, char(')'), space0))
    )(input)
}

fn parse_positive_constraint(input: &str) -> IResult<&str, Constraint> {
    parse_leaf_constraint(input)
}

fn parse_os_constraint(input: &str) -> IResult<&str, Constraint> {
    let (input, _) = tag_no_case("os")(input)?;
    delimited(
        tuple((char('('), space0)),
        alt((
            map(tag_no_case("windows"), |_| Constraint::Windows),
            map(tag_no_case("linux"), |_| Constraint::Linux),
            map(tag_no_case("macos"), |_| Constraint::MacOsx)
        )),
        tuple((space0, char(')'), space0))
    )(input)
}

fn parse_leaf_constraint(input: &str) -> IResult<&str, Constraint> {
    parse_os_constraint(input)
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
