#![allow(clippy::cast_lossless)]

use cpclib_common::winnow::error::{AddContext, ParserError, StrContext};
use cpclib_common::winnow::stream::Stream;

use super::obtained::LocatedListing;
use crate::parser::source::Z80Span;
use crate::InnerZ80Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Z80ParserErrorKind {
    /// Static string added by the `context` function
    Context(StrContext),
    /// Like `Context`, but also records the byte offset (within the same source
    /// file) of the furthest actual failure that triggered this context.
    /// The error renderer uses it to extend the highlighted region from the
    /// start of a multi-line construct to where the parser actually got stuck.
    ContextWithEnd {
        context: StrContext,
        end_offset: usize
    },
    /// Debug-only context label: shown only when `[DBG]` filtering is disabled.
    /// Structurally separate so the renderer never needs to inspect label text.
    DebugContext(StrContext),
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

impl Z80ParserErrorKind {
    /// Returns the human-readable label for this error kind.
    /// Must not be called on `Inner` entries — those are flattened by `errors()` before display.
    pub fn display_label(&self) -> String {
        match self {
            Self::Context(ctx)
            | Self::ContextWithEnd { context: ctx, .. }
            | Self::DebugContext(ctx) => ctx.to_string(),
            Self::Winnow => "Unknown error".to_owned(),
            Self::Char(c) => format!("Error with char '{c}'"),
            Self::Inner { .. } => unreachable!("Inner entries are flattened before display"),
        }
    }

    /// Returns `true` when this is a debug-only context label.
    pub fn is_dbg(&self) -> bool {
        matches!(self, Self::DebugContext(_))
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

    /// Create a new error from input - convenience method that delegates to the trait method
    pub fn from_input(input: &InnerZ80Span) -> Self {
        <Self as ParserError<InnerZ80Span>>::from_input(input)
    }
}

impl ParserError<InnerZ80Span> for Z80ParserError {
    type Inner = Self;

    fn from_input(input: &InnerZ80Span) -> Self {
        Self(vec![(*input, Z80ParserErrorKind::Winnow)])
    }

    fn append(
        mut self,
        input: &InnerZ80Span,
        _token_start: &<InnerZ80Span as Stream>::Checkpoint
    ) -> Self {
        self.0.push((*input, Z80ParserErrorKind::Winnow));
        self
    }

    fn into_inner(self) -> Result<Self::Inner, Self> {
        Ok(self)
    }

    fn assert(input: &InnerZ80Span, _message: &'static str) -> Self {
        #[cfg(debug_assertions)]
        panic!("assert `{_message}` failed at {input:#?}");
        #[cfg(not(debug_assertions))]
        Self::from_input(input)
    }

    fn or(self, other: Self) -> Self {
        other
    }
}

/// Recursively search `errors` (descending into `Inner` wrappers) for the
/// furthest byte offset within `origin_file` among all non-context leaf entries.
fn find_furthest_offset_same_file(
    origin_file: &str,
    errors: &[(InnerZ80Span, Z80ParserErrorKind)]
) -> Option<usize> {
    errors
        .iter()
        .filter_map(|(span, kind)| match kind {
            Z80ParserErrorKind::Context(_)
            | Z80ParserErrorKind::ContextWithEnd { .. }
            | Z80ParserErrorKind::DebugContext(_) => None,
            Z80ParserErrorKind::Inner { error, .. } => {
                if Z80Span::from(*span).filename() == origin_file {
                    find_furthest_offset_same_file(origin_file, &error.0)
                }
                else {
                    None
                }
            },
            _ => {
                let s = Z80Span::from(*span);
                if s.filename() == origin_file { Some(s.offset_from_start()) } else { None }
            }
        })
        .max()
}

/// Returns the furthest byte offset among all non-context entries already in
/// `errors` (recursing into `Inner` wrappers), but only when that offset is
/// strictly further into the file than `origin`'s own position.
fn furthest_failure_offset_same_file(
    origin: &InnerZ80Span,
    errors: &[(InnerZ80Span, Z80ParserErrorKind)]
) -> Option<usize> {
    let origin_span = Z80Span::from(*origin);
    find_furthest_offset_same_file(origin_span.filename(), errors)
        .filter(|&off| off > origin_span.offset_from_start())
}

impl Z80ParserError {
    fn make_context_kind(&self, input: &InnerZ80Span, ctx: StrContext) -> Z80ParserErrorKind {
        match furthest_failure_offset_same_file(input, &self.0) {
            Some(end_offset) => Z80ParserErrorKind::ContextWithEnd { context: ctx, end_offset },
            None => Z80ParserErrorKind::Context(ctx)
        }
    }
}

impl AddContext<InnerZ80Span> for Z80ParserError {
    fn add_context(
        mut self,
        input: &InnerZ80Span,
        _start: &<InnerZ80Span as Stream>::Checkpoint,
        ctx: &'static str
    ) -> Self {
        let kind = self.make_context_kind(input, StrContext::Label(ctx));
        self.0.push((*input, kind));
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
        let kind = self.make_context_kind(input, ctx);
        self.0.push((*input, kind));
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
