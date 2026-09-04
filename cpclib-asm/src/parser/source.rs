// SAFETY: All fields of Z80Span are Sync (InnerZ80Span, which is a wrapper around references to static data and ParserContext)
unsafe impl Sync for Z80Span {}
use std::fmt::Display;
use std::ops::{Deref, DerefMut};

use cpclib_common::smol_str::SmolStr;
use cpclib_common::winnow::stream::{AsBStr, LocatingSlice, Offset};
use cpclib_common::winnow::{BStr, Stateful};
use cpclib_tokens::symbols::{SourceLocation, Symbol};
use line_span::LineSpanExt;

use super::ParsingState;
use super::context::ParserContext;

// This type is only handled by the parser
pub type InnerZ80Span = Stateful<
    LocatingSlice<
        // the type of data, owned by the base listing of interest
        &'static BStr
    >,
    // The parsing context
    // TODO remove it an pass it over the parse arguments
    &'static ParserContext
>;

#[derive(Clone, PartialEq, Eq)]
pub struct Z80Span(pub(crate) InnerZ80Span);

impl From<InnerZ80Span> for Z80Span {
    fn from(value: InnerZ80Span) -> Self {
        Self(value)
    }
}

impl From<Z80Span> for InnerZ80Span {
    fn from(val: Z80Span) -> Self {
        val.0
    }
}

impl AsRef<str> for Z80Span {
    #[inline]
    fn as_ref(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(self.0.as_bstr()) }
    }
}

impl<'a> From<&'a Z80Span> for &'a str {
    fn from(val: &'a Z80Span) -> Self {
        AsRef::as_ref(val)
    }
}

pub trait SourceString: Display {
    fn as_str(&self) -> &str;
}

impl From<&dyn SourceString> for Symbol {
    fn from(val: &dyn SourceString) -> Self {
        val.as_str().into()
    }
}

impl From<&Z80Span> for Symbol {
    fn from(val: &Z80Span) -> Self {
        val.as_str().into()
    }
}

impl SourceString for &Z80Span {
    fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl SourceString for Z80Span {
    fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl SourceString for &String {
    fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl SourceString for String {
    fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl SourceString for &SmolStr {
    fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl SourceString for str {
    fn as_str(&self) -> &str {
        self
    }
}

impl SourceString for SmolStr {
    fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl Z80Span {
    #[inline]
    pub fn complete_source(&self) -> &str {
        self.0.state.complete_source()
    }

    /// Get the offset from the start of the string (when considered to be a array of bytes)
    #[inline]
    pub fn offset_from_start(&self) -> usize {
        let src = self.complete_source();
        let src = src.as_bstr();
        self.as_bstr().offset_from(&src)
    }

    /// Get the line and column relatively to the source start
    #[inline]
    pub fn relative_line_and_column(&self) -> (usize, usize) {
        let offset = self.offset_from_start();
        self.context().relative_line_and_column(offset)
    }

    #[inline]
    pub fn location_line(&self) -> u32 {
        self.relative_line_and_column().0 as _
    }

    /// Get the full line from the whole source code that contains the following span
    #[inline]
    pub fn complete_line(&self) -> &str {
        let offset = self.offset_from_start();
        let range = self.complete_source().find_line_range(offset);
        let line = &self.complete_source().as_bytes()[range.start..range.end];
        unsafe { std::str::from_utf8_unchecked(line) }
    }

    #[inline]
    pub fn get_line_beginning(&self) -> &str {
        self.complete_line()
    }

    #[inline]
    pub fn filename(&self) -> &str {
        self.state
            .filename()
            .as_ref()
            .map(|p| p.as_os_str().to_str().unwrap_or("[Invalid file name]"))
            .unwrap_or_else(|| {
                self.state
                    .context_name
                    .as_ref()
                    .map(|s| s.as_ref())
                    .unwrap_or_else(|| "no file specified")
            })
    }
}

impl std::fmt::Display for Z80Span {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Debug for Z80Span {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (line, column) = self.relative_line_and_column();
        write!(
            f,
            "{}:{}:{} <{}>",
            self.context()
                .current_filename
                .as_ref()
                .map(|p| p.as_str())
                .unwrap_or("<unknown filename>"),
            line,
            column,
            self.as_str()
        )
    }
}

impl From<&Z80Span> for SmolStr {
    fn from(val: &Z80Span) -> Self {
        SmolStr::from(val.as_str())
    }
}

impl From<&Z80Span> for SourceLocation {
    #[inline]
    fn from(val: &Z80Span) -> Self {
        let (line, column) = val.relative_line_and_column();

        SourceLocation::new(
            val.context()
                .current_filename
                .as_ref()
                .map(|fname| fname.as_str().to_owned())
                .unwrap_or_else(|| "<INLINE>".into()),
            line as _,
            column
        )
    }
}

// Impossible as the string MUST exist more than the span
// impl From<String> for Z80Span {
// fn from(s: String) -> Self {
// let src = Arc::new(s);
// let ctx = Arc::default();
//
// Self(LocatingSliceSpan::new_extra(
// The string is safe on the heap
// unsafe { &*(src.as_str() as *const str) as &'static str },
// (src, ctx)
// ))
// }
// }

// check if still needed
// impl Z80Span {
// pub fn from_standard_span(
// span: LocatingSliceSpan<&'static str, ()>,
// extra: (Arc<String>, Arc<ParserContext>)
// ) -> Self {
// {
// let _span_addr = span.fragment().as_ptr();
// let _extra_addr = extra.as_ptr();
// TODO; no idea why it fails :()
//   assert!(std::ptr::eq(span_addr, extra_addr));
// }
//
// Self(unsafe {
// LocatingSliceSpan::new_from_raw_offset(
// span.location_offset(),
// span.location_line(),
// span.fragment(),
// extra
// )
// })
// }
// }

impl Deref for Z80Span {
    type Target = InnerZ80Span;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Z80Span {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl AsRef<InnerZ80Span> for Z80Span {
    #[inline]
    fn as_ref(&self) -> &InnerZ80Span {
        self.deref()
    }
}

impl Z80Span {
    pub fn new_extra<S: ?Sized + AsRef<[u8]>>(src: &S, ctx: &ParserContext) -> Self {
        let src = unsafe { std::mem::transmute::<&BStr, &'static BStr>(BStr::new(src)) };
        let ctx = unsafe { &*(ctx as *const ParserContext) as &'static ParserContext };

        Self(Stateful {
            input: LocatingSlice::new(src),
            state: ctx
        })
    }

    pub fn context(&self) -> &ParserContext {
        self.state
    }

    /// A value that uniquely identifies this span's position within the
    /// listing it was parsed from - stable for as long as that listing is
    /// alive, but meaningless across a *different* parse of the same text
    /// (a re-parse gets its own, fresh contexts).
    ///
    /// Two coordinates, not just [`Self::offset_from_start`]: a real project
    /// `include`s several source files, and each gets its own
    /// [`ParserContext`] whose own source starts back at offset 0 - so the
    /// offset alone collides between files. Each included file's context is
    /// a separate, stably-addressed object for the listing's lifetime (see
    /// [`Self::context`]), so pairing the context's own identity with the
    /// offset disambiguates cheaply, with no filename comparison needed.
    pub fn identity(&self) -> SpanIdentity {
        SpanIdentity(
            std::ptr::from_ref(self.context()) as usize,
            self.offset_from_start()
        )
    }
}

/// See [`Z80Span::identity`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpanIdentity(usize, usize);

#[cfg(test)]
mod identity_tests {
    use super::super::context::ParserOptions;
    use super::*;

    /// Reproduces the real multi-file bug directly: two independently parsed
    /// sources (standing in for two files a project `include`s) each start
    /// their own span numbering at offset 0, so `offset_from_start()` alone
    /// collides between them - `Env::address_trace` used to be keyed by
    /// exactly that, silently mixing up two different files' addresses.
    #[test]
    fn two_spans_at_the_same_offset_in_different_parses_have_different_identities() {
        let ctx_a = ParserOptions::default().context_builder().build("nop");
        let ctx_b = ParserOptions::default().context_builder().build("nop");

        let span_a = ctx_a.build_span(ctx_a.source);
        let span_b = ctx_b.build_span(ctx_b.source);

        assert_eq!(span_a.offset_from_start(), 0);
        assert_eq!(span_b.offset_from_start(), 0);
        assert_ne!(
            span_a.identity(),
            span_b.identity(),
            "two different parses' spans at the same offset must not collide"
        );
    }

    #[test]
    fn the_same_span_has_a_stable_identity() {
        let ctx = ParserOptions::default().context_builder().build("nop");
        let span = ctx.build_span(ctx.source);
        assert_eq!(span.identity(), span.clone().identity());
    }

    #[test]
    fn two_different_offsets_in_the_same_parse_have_different_identities() {
        let ctx = ParserOptions::default()
            .context_builder()
            .build("nop : nop");
        let whole = ctx.build_span(ctx.source);
        let tail = ctx.build_span(&ctx.source[7..]);
        assert_ne!(whole.identity(), tail.identity());
    }
}

impl Z80Span {
    // Used when the state is changing (it controls the parsing)
    // pub fn clone_with_state(&self, state: ParsingState) -> Self {
    // eprintln!("Z80Span::clone_with_state used. Need to check if it could be done differently as the state is supposed to be hold by the listing");
    // let ctx = self.context().clone_with_state(state);
    // let mut clone = self.clone();
    // clone.extra =  w(ctx);
    // clone
    // }
    pub fn state(&self) -> &ParsingState {
        self.context().state()
    }
}
