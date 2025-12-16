#![allow(clippy::cast_lossless)]

#[allow(deprecated)]
use cpclib_common::winnow::error::ErrorKind;
use cpclib_common::winnow::error::{AddContext, ParserError, StrContext};
use cpclib_common::winnow::stream::Stream;

use super::obtained::LocatedListing;
use crate::InnerZ80Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Z80ParserErrorKind {
    /// Static string added by the `context` function
    Context(StrContext),
    /// Indicates which character was expected by the `char` function
    Char(char),
    /// Error kind given by various nom parsers
    Winnow,
    /// Chain of errors provided by an inner listing
    Inner {
        listing: std::sync::Arc<LocatedListing>,
        error: Box<Z80ParserError>
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Z80ParserError(pub Vec<(InnerZ80Span, Z80ParserErrorKind)>);

impl Z80ParserError {
    pub fn errors(&self) -> Vec<(&InnerZ80Span, &Z80ParserErrorKind)> {
        let mut res = Vec::new();

        for e in self.0.iter() {
            if let Z80ParserErrorKind::Inner { listing: _, error } = &e.1 {
                res.extend(error.errors())
            }
            else {
                res.push((&e.0, &e.1));
            }
        }
        res
    }
}

impl From<char> for Z80ParserErrorKind {
    fn from(other: char) -> Self {
        Self::Char(other)
    }
}

impl Z80ParserError {
    pub fn from_inner_error(
        input: &InnerZ80Span,
        listing: std::sync::Arc<LocatedListing>,
        error: Box<Z80ParserError>
    ) -> Self {
        Self(vec![(*input, Z80ParserErrorKind::Inner { listing, error })])
    }

    #[allow(deprecated)]
    pub fn from_input(input: &InnerZ80Span) -> Self {
        Self::from_error_kind(input, ErrorKind::Fail)
    }
}

impl ParserError<InnerZ80Span> for Z80ParserError {
    #[allow(deprecated)]
    fn from_error_kind(input: &InnerZ80Span, _kind: ErrorKind) -> Self {
        Self(vec![(*input, Z80ParserErrorKind::Winnow)])
    }

    #[allow(deprecated)]
    fn append(
        mut self,
        input: &InnerZ80Span,
        _token_start: &<InnerZ80Span as Stream>::Checkpoint,
        _kind: ErrorKind
    ) -> Self {
        self.0.push((*input, Z80ParserErrorKind::Winnow));
        self
    }

    fn assert(input: &InnerZ80Span, _message: &'static str) -> Self {
        #[cfg(debug_assertions)]
        panic!("assert `{_message}` failed at {input:#?}");
        #[cfg(not(debug_assertions))]
        #[allow(deprecated)]
        Self::from_error_kind(input, ErrorKind::Assert)
    }

    fn or(self, other: Self) -> Self {
        other
    }
}

impl AddContext<InnerZ80Span> for Z80ParserError {
    fn add_context(
        mut self,
        input: &InnerZ80Span,
        _start: &<InnerZ80Span as Stream>::Checkpoint,
        ctx: &'static str
    ) -> Self {
        self.0
            .push((*input, Z80ParserErrorKind::Context(StrContext::Label(ctx))));
        self
    }
}

impl AddContext<InnerZ80Span, StrContext> for Z80ParserError {
    fn add_context(
        mut self,
        input: &InnerZ80Span,
        _start: &<InnerZ80Span as Stream>::Checkpoint,
        ctx: StrContext
    ) -> Self {
        self.0.push((*input, Z80ParserErrorKind::Context(ctx)));
        self
    }
}

/// ...
pub mod error_code {
    /// ...
    pub const ASSERT_MUST_BE_FOLLOWED_BY_AN_EXPRESSION: u32 = 128;
    /// ...
    pub const INVALID_ARGUMENT: u32 = 129;
    /// ...
    pub const UNABLE_TO_PARSE_INNER_CONTENT: u32 = 130;
}
