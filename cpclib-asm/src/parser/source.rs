use cpclib_common::nom::{
    error::{ErrorKind, ParseError},
    Compare, CompareResult, Err, FindSubstring, IResult, InputIter, InputLength, InputTake, Needed,
    Offset, Slice,
};
use cpclib_common::nom_locate::LocatedSpan;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use super::context::ParserContext;

#[derive(Clone, PartialEq)]
pub struct Z80Span(
    pub(crate)  LocatedSpan<
        // the type of data
        &'static str,
        (
            // The full source (same as the &str)
            Arc<String>,
            // The parsing context
            Arc<ParserContext>,
        ),
    >,
);

impl From<&'src str> for Z80Span {
    fn from(s: &'src str) -> Self {
        let src = Arc::new(s.to_owned());
        let ctx = Arc::default();

        let len = src.len();

        Self(LocatedSpan::new_extra(
            // The string is safe on the heap
            unsafe { 
                std::str::from_utf8_unchecked(
                    &*std::ptr::slice_from_raw_parts(src.as_ptr(), len) as _
                )
            },
            (src, ctx),
        ))
    }
}

impl From<String> for Z80Span {
    fn from(s: String) -> Self {
        let src = Arc::new(s);
        let ctx = Arc::default();

        Self(LocatedSpan::new_extra(
            // The string is safe on the heap
            unsafe { &*(src.as_str() as *const str) as &'static str },
            (src, ctx),
        ))
    }
}

impl Z80Span {
    pub fn from_standard_span(
        span: LocatedSpan<&'static str, ()>,
        extra: (Arc<String>, Arc<ParserContext>),
    ) -> Self {
        {
            let span_addr = span.fragment().as_ptr();
            let extra_addr = extra.0.as_ptr();
         // TODO; no idea why it fails :()
            //   assert!(std::ptr::eq(span_addr, extra_addr));
        }

        Self(unsafe {
            LocatedSpan::new_from_raw_offset(
                span.location_offset(),
                span.location_line(),
                span.fragment(),
                extra,
            )
        })
    }
}

impl<'a> Into<LocatedSpan<&'a str>> for Z80Span {
    fn into(self) -> LocatedSpan<&'a str> {
        unsafe {
            LocatedSpan::new_from_raw_offset(
                self.location_offset(),
                self.location_line(),
                self.fragment(),
                (),
            )
        }
    }
}

impl Deref for Z80Span {
    type Target = LocatedSpan<&'static str, (Arc<String>, Arc<ParserContext>)>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Z80Span {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl AsRef<LocatedSpan<&'static str, (Arc<String>, Arc<ParserContext>)>> for Z80Span {
    fn as_ref(&self) -> &LocatedSpan<&'static str, (Arc<String>, Arc<ParserContext>)> {
        self.deref()
    }
}
impl std::fmt::Debug for Z80Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.deref().fmt(f)
    }
}
impl Compare<&'static str> for Z80Span {
    fn compare(&self, t: &'static str) -> CompareResult {
        self.deref().compare(t)
    }
    fn compare_no_case(&self, t: &'static str) -> CompareResult {
        self.deref().compare_no_case(t)
    }
}
impl cpclib_common::nom::InputIter for Z80Span {
    type Item =
        <LocatedSpan<&'static str, (Arc<String>, Arc<ParserContext>)> as cpclib_common::nom::InputIter>::Item;

    type Iter =
        <LocatedSpan<&'static str, (Arc<String>, Arc<ParserContext>)> as cpclib_common::nom::InputIter>::Iter;

    type IterElem =
        <LocatedSpan<&'static str, (Arc<String>, Arc<ParserContext>)> as cpclib_common::nom::InputIter>::IterElem;

    fn iter_indices(&self) -> Self::Iter {
        self.deref().iter_indices()
    }

    fn iter_elements(&self) -> Self::IterElem {
        self.deref().iter_elements()
    }

    fn position<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Item) -> bool,
    {
        self.deref().position(predicate)
    }

    fn slice_index(&self, count: usize) -> Result<usize, Needed> {
        self.deref().slice_index(count)
    }
}

impl cpclib_common::nom::InputLength for Z80Span {
    fn input_len(&self) -> usize {
        self.deref().input_len()
    }
}

impl Offset for Z80Span {
    fn offset(&self, second: &Self) -> usize {
        self.deref().offset(second.deref())
    }
}

impl cpclib_common::nom::InputTake for Z80Span {
    fn take(&self, count: usize) -> Self {
        Self(self.deref().take(count))
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        let res = self.deref().take_split(count);
        (Self(res.0), Self(res.1))
    }
}

impl cpclib_common::nom::InputTakeAtPosition for Z80Span {
    type Item =
        <LocatedSpan<&'static str, (Arc<String>, Arc<ParserContext>)> as cpclib_common::nom::InputIter>::Item;

    fn split_at_position<P, E: ParseError<Self>>(&self, predicate: P) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.deref().position(predicate) {
            Some(n) => Ok(self.take_split(n)),
            None => Err(Err::Incomplete(cpclib_common::nom::Needed::new(1))),
        }
    }

    fn split_at_position1<P, E: ParseError<Self>>(
        &self,
        predicate: P,
        e: ErrorKind,
    ) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.deref().position(predicate) {
            Some(0) => Err(Err::Error(E::from_error_kind(self.clone(), e))),
            Some(n) => Ok(self.take_split(n)),
            None => Err(Err::Incomplete(cpclib_common::nom::Needed::new(1))),
        }
    }

    fn split_at_position_complete<P, E: ParseError<Self>>(
        &self,
        predicate: P,
    ) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.split_at_position(predicate) {
            Err(Err::Incomplete(_)) => Ok(self.take_split(self.input_len())),
            res => res,
        }
    }

    fn split_at_position1_complete<P, E: ParseError<Self>>(
        &self,
        predicate: P,
        e: ErrorKind,
    ) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.fragment().position(predicate) {
            Some(0) => Err(Err::Error(E::from_error_kind(self.clone(), e))),
            Some(n) => Ok(self.take_split(n)),
            None => {
                if self.fragment().input_len() == 0 {
                    Err(Err::Error(E::from_error_kind(self.clone(), e)))
                } else {
                    Ok(self.take_split(self.input_len()))
                }
            }
        }
    }
}
impl<'src, 'ctx, U> FindSubstring<U> for Z80Span
where
    &'src str: FindSubstring<U>,
{
    #[inline]
    fn find_substring(&self, substr: U) -> Option<usize> {
        self.fragment().find_substring(substr)
    }
}

impl Slice<std::ops::Range<usize>> for Z80Span {
    fn slice(&self, range: std::ops::Range<usize>) -> Self {
        Self(self.deref().slice(range))
    }
}
impl Slice<std::ops::RangeFrom<usize>> for Z80Span {
    fn slice(&self, range: std::ops::RangeFrom<usize>) -> Self {
        Self(self.deref().slice(range))
    }
}
impl Slice<std::ops::RangeTo<usize>> for Z80Span {
    fn slice(&self, range: std::ops::RangeTo<usize>) -> Self {
        Self(self.deref().slice(range))
    }
}

impl Z80Span {
    pub fn new_extra<S: Into<String>>(src: S, ctx: ParserContext) -> Self {
        let src = Arc::new(src.into());
        let ctx = Arc::new(ctx);

        Self::new_extra_from_rc(src, ctx)
    }

    pub fn new_extra_from_rc(src: Arc<String>, ctx: Arc<ParserContext>) -> Self {
        Self(LocatedSpan::new_extra(
            // pointer is always good as source is store in a Arc
            unsafe { &*(src.as_str() as *const str) as &'static str },
            (Arc::clone(&src), Arc::clone(&ctx)),
        ))
    }

    pub fn context_mut(&mut self) -> &mut ParserContext {
        Arc::get_mut(&mut self.0.extra.1).unwrap()
    }

    pub fn context(& self) -> & ParserContext {
        & self.0.extra.1
    }
}
