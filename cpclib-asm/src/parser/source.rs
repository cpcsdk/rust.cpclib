use std::fmt::Display;
use std::ops::{Deref, DerefMut};

use cpclib_common::smol_str::SmolStr;
use cpclib_common::winnow::stream::{AsBStr, Offset};
use cpclib_common::winnow::{BStr, Bytes, Located, Stateful};
use cpclib_tokens::symbols::{Source, Symbol};
use line_col::LineColLookup;
use line_span::LineSpanExt;

use super::context::ParserContext;
use super::ParsingState;

// This type is only handled by the parser
pub type InnerZ80Span = Stateful<
    Located<
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

impl Into<InnerZ80Span> for Z80Span {
    fn into(self) -> InnerZ80Span {
        self.0
    }
}

impl AsRef<str> for Z80Span {
    #[inline]
    fn as_ref(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(self.0.as_bstr()) }
    }
}

impl<'a> Into<&'a str> for &'a Z80Span {
    fn into(self) -> &'a str {
        AsRef::as_ref(self)
    }
}

pub trait SourceString: Display {
    fn as_str(&self) -> &str;
}

impl Into<Symbol> for &dyn SourceString {
    fn into(self) -> Symbol {
        self.as_str().into()
    }
}

impl Into<Symbol> for &Z80Span {
    fn into(self) -> Symbol {
        self.as_str().into()
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

impl SourceString for &SmolStr {
    fn as_str(&self) -> &str {
        self.as_ref()
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
        // TODO store this lookup somewhere instead of recomputing it each time
        let lookup = LineColLookup::new(self.complete_source());

        let offset = self.offset_from_start();
        lookup.get(offset)
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
                .map(|f| f.to_str().unwrap_or("<invalid filename>"))
                .unwrap_or("<unknown filename>"),
            line,
            column,
            self.as_str()
        )
    }
}

impl Into<SmolStr> for &Z80Span {
    fn into(self) -> SmolStr {
        SmolStr::from(self.as_str())
    }
}

impl Into<Source> for &Z80Span {
    #[inline]
    fn into(self) -> Source {
        let (line, column) = self.relative_line_and_column();

        Source::new(
            self.context()
                .current_filename
                .as_ref()
                .map(|fname| fname.display().to_string())
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
// Self(LocatedSpan::new_extra(
// The string is safe on the heap
// unsafe { &*(src.as_str() as *const str) as &'static str },
// (src, ctx)
// ))
// }
// }

// check if still needed
// impl Z80Span {
// pub fn from_standard_span(
// span: LocatedSpan<&'static str, ()>,
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
// LocatedSpan::new_from_raw_offset(
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
        let src = unsafe { std::mem::transmute(BStr::new(src)) };
        let ctx = unsafe { &*(ctx as *const ParserContext) as &'static ParserContext };

        Self(Stateful {
            input: Located::new(src),
            state: ctx
        })
    }

    pub fn context(&self) -> &ParserContext {
        &self.state
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
