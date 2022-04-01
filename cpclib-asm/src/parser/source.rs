use std::borrow::Borrow;
use std::ops::{Deref, DerefMut};

use cpclib_common::nom::error::{ErrorKind, ParseError};
use cpclib_common::nom::{
    Compare, CompareResult, Err, FindSubstring, IResult, InputIter, InputLength, InputTake, Needed,
    Offset, Slice
};
use cpclib_common::nom_locate::LocatedSpan;
use cpclib_tokens::symbols::Source;

use super::context::ParserContext;
use super::ParsingState;

type InnerZ80Span = LocatedSpan<
    // the type of data, owned by the base listing of interest
    &'static str,
    // The parsing context
    // TODO remove it an pass it over the parse arguments
    &'static ParserContext
>;

#[derive(Clone, PartialEq, Eq)]
pub struct Z80Span(pub(crate) InnerZ80Span);

impl AsRef<str> for Z80Span {
    #[inline]
    fn as_ref(&self) -> &str {
        self.fragment()
    }
}

impl Z80Span {
    #[inline]
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl Borrow<str> for Z80Span {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
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
        write!(
            f,
            "{}:{}:{} <{}>",
            self.context()
                .current_filename
                .as_ref()
                .map(|f| f.to_str().unwrap_or("<invalid filename>"))
                .unwrap_or("unknown"),
            self.location_line(),
            self.get_utf8_column(),
            self.as_str()
        )
    }
}

impl Into<Source> for &Z80Span {
    #[inline]
    fn into(self) -> Source {
        Source::new(
            self.context()
                .current_filename
                .as_ref()
                .map(|fname| fname.display().to_string())
                .unwrap_or_else(|| "<INLINE>".into()),
            self.0.location_line() as _,
            self.0.get_utf8_column()
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

impl<'a> Into<LocatedSpan<&'a str>> for Z80Span {
    #[inline]
    fn into(self) -> LocatedSpan<&'a str> {
        unsafe {
            LocatedSpan::new_from_raw_offset(
                self.location_offset(),
                self.location_line(),
                self.fragment(),
                ()
            )
        }
    }
}

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

impl Compare<&'static str> for Z80Span {
    #[inline]
    fn compare(&self, t: &'static str) -> CompareResult {
        self.deref().compare(t)
    }

    #[inline]
    fn compare_no_case(&self, t: &'static str) -> CompareResult {
        self.deref().compare_no_case(t)
    }
}
impl cpclib_common::nom::InputIter for Z80Span {
    type Item = <InnerZ80Span as cpclib_common::nom::InputIter>::Item;
    type Iter = <InnerZ80Span as cpclib_common::nom::InputIter>::Iter;
    type IterElem = <InnerZ80Span as cpclib_common::nom::InputIter>::IterElem;

    #[inline]
    fn iter_indices(&self) -> Self::Iter {
        self.deref().iter_indices()
    }

    #[inline]
    fn iter_elements(&self) -> Self::IterElem {
        self.deref().iter_elements()
    }

    #[inline]
    fn position<P>(&self, predicate: P) -> Option<usize>
    where P: Fn(Self::Item) -> bool {
        self.deref().position(predicate)
    }

    #[inline]
    fn slice_index(&self, count: usize) -> Result<usize, Needed> {
        self.deref().slice_index(count)
    }
}

impl cpclib_common::nom::InputLength for Z80Span {
    #[inline]
    fn input_len(&self) -> usize {
        self.deref().input_len()
    }
}

impl Offset for Z80Span {
    #[inline]
    fn offset(&self, second: &Self) -> usize {
        self.deref().offset(second.deref())
    }
}

impl cpclib_common::nom::InputTake for Z80Span {
    #[inline]
    fn take(&self, count: usize) -> Self {
        Self(self.deref().take(count))
    }

    #[inline]
    fn take_split(&self,
        count: usize) -> (Self, Self) {
        let res = self.deref().take_split(count);
        (Self(res.0), Self(res.1))
    }
}

impl cpclib_common::nom::InputTakeAtPosition for Z80Span {
    type Item = <InnerZ80Span as cpclib_common::nom::InputIter>::Item;

    #[inline]
    fn split_at_position<P, E: ParseError<Self>>(&self, predicate: P) -> IResult<Self, Self, E>
    where P: Fn(Self::Item) -> bool {
        match self.deref().position(predicate) {
            Some(n) => Ok(self.take_split(n)),
            None => Err(Err::Incomplete(cpclib_common::nom::Needed::new(1)))
        }
    }

    #[inline]
    fn split_at_position1<P, E: ParseError<Self>>(
        &self,
        predicate: P,
        e: ErrorKind
    ) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool
    {
        match self.deref().position(predicate) {
            Some(0) => Err(Err::Error(E::from_error_kind(self.clone(), e))),
            Some(n) => Ok(self.take_split(n)),
            None => Err(Err::Incomplete(cpclib_common::nom::Needed::new(1)))
        }
    }

    #[inline]
    fn split_at_position_complete<P, E: ParseError<Self>>(
        &self,
        predicate: P
    ) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool
    {
        match self.split_at_position(predicate) {
            Err(Err::Incomplete(_)) => Ok(self.take_split(self.input_len())),
            res => res
        }
    }

    #[inline]
    fn split_at_position1_complete<P, E: ParseError<Self>>(
        &self,
        predicate: P,
        e: ErrorKind
    ) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool
    {
        match self.fragment().position(predicate) {
            Some(0) => Err(Err::Error(E::from_error_kind(self.clone(), e))),
            Some(n) => Ok(self.take_split(n)),
            None => {
                if self.fragment().input_len() == 0 {
                    Err(Err::Error(E::from_error_kind(self.clone(), e)))
                }
                else {
                    Ok(self.take_split(self.input_len()))
                }
            }
        }
    }
}
impl<'src, 'ctx, U> FindSubstring<U> for Z80Span
where &'src str: FindSubstring<U>
{
    #[inline]
    fn find_substring(&self, substr: U) -> Option<usize> {
        self.fragment().find_substring(substr)
    }
}

impl Slice<std::ops::Range<usize>> for Z80Span {
    #[inline]
    fn slice(&self, range: std::ops::Range<usize>) -> Self {
        Self(self.deref().slice(range))
    }
}
impl Slice<std::ops::RangeFrom<usize>> for Z80Span {
    #[inline]
    fn slice(&self, range: std::ops::RangeFrom<usize>) -> Self {
        Self(self.deref().slice(range))
    }
}
impl Slice<std::ops::RangeTo<usize>> for Z80Span {
    #[inline]
    fn slice(&self, range: std::ops::RangeTo<usize>) -> Self {
        Self(self.deref().slice(range))
    }
}

impl Z80Span {
    pub fn new_extra(src: &str, ctx: &ParserContext) -> Self {
        Self(LocatedSpan::new_extra(
            // pointer is always good as source is stored in a Arc
            unsafe { &*(src as *const str) as &'static str },
            unsafe { &*(ctx as *const ParserContext) as &'static ParserContext }
        ))
    }

    pub fn context(&self) -> &ParserContext {
        &self.0.extra
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
        &self.context().state
    }
}
