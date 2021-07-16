use std::ops::{DerefMut,Deref};


use nom::{Compare, CompareResult, Err, FindSubstring, IResult, InputIter, InputLength, InputTake, Needed, Slice, error::{ErrorKind, ParseError}};
use nom_locate::LocatedSpan;

use super::context::{ParserContext, DEFAULT_CTX};


#[derive(Clone, PartialEq)]
pub struct Z80Span<'src, 'ctx>(pub(crate) LocatedSpan<&'src str, &'ctx ParserContext>);
impl<'src, 'ctx> From<&'src str> for Z80Span<'src, 'ctx> {
    fn from(s: &'src str) -> Self {
        Self(LocatedSpan::new_extra(s, &DEFAULT_CTX))
    }
}
impl<'src, 'ctx> Deref for  Z80Span<'src, 'ctx> {
    type Target = LocatedSpan<&'src str, &'ctx ParserContext>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'src, 'ctx> DerefMut for  Z80Span<'src, 'ctx> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<'src, 'ctx> AsRef<LocatedSpan<&'src str, &'ctx ParserContext>> for  Z80Span<'src, 'ctx> {
    fn as_ref(&self) -> &LocatedSpan<&'src str, &'ctx ParserContext>{
        self.deref()
    }
}
impl<'src, 'ctx> std::fmt::Debug for  Z80Span<'src, 'ctx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.deref().fmt(f)
    }

}
impl<'src, 'ctx> Compare<&'static str> for  Z80Span<'src, 'ctx> {
 fn compare(&self, t: &'static str) -> CompareResult {
     self.deref().compare(t)
 }
 fn compare_no_case(&self, t: &'static str) -> CompareResult {
    self.deref().compare_no_case(t)
}

}
impl<'src, 'ctx>  nom::InputIter for  Z80Span<'src, 'ctx> {
    type Item = <LocatedSpan<&'src str, &'ctx ParserContext> as nom::InputIter>::Item;

    type Iter = <LocatedSpan<&'src str, &'ctx ParserContext> as nom::InputIter>::Iter;

    type IterElem = <LocatedSpan<&'src str, &'ctx ParserContext> as nom::InputIter>::IterElem;

    fn iter_indices(&self) -> Self::Iter {
        self.deref().iter_indices()
    }

    fn iter_elements(&self) -> Self::IterElem {
        self.deref().iter_elements()
    }

    fn position<P>(&self, predicate: P) -> Option<usize>
  where
    P: Fn(Self::Item) -> bool {
        self.deref().position(predicate)
    }

    fn slice_index(&self, count: usize) -> Result<usize, Needed> {
        self.deref().slice_index(count)
    }
}

impl<'src, 'ctx>  nom::InputLength for  Z80Span<'src, 'ctx> {
    fn input_len(&self) -> usize {
        self.deref().input_len()
    }
}

impl<'src, 'ctx>  nom::InputTake for  Z80Span<'src, 'ctx> 
{
    fn take(&self, count: usize) -> Self {
        Self(self.deref().take(count))
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        let res = self.deref().take_split(count);
        (Self(res.0), Self(res.1))
    }
}


impl<'src, 'ctx>  nom::InputTakeAtPosition for  Z80Span<'src, 'ctx> {
    type Item = <LocatedSpan<&'src str, &'ctx ParserContext> as nom::InputIter>::Item;

    fn split_at_position<P, E: ParseError<Self>>(&self, predicate: P) -> IResult<Self, Self, E>
  where
    P: Fn(Self::Item) -> bool {
        match self.deref().position(predicate) {
            Some(n) => Ok(self.take_split(n)),
            None => Err(Err::Incomplete(nom::Needed::new(1))),
        }
    }

    fn split_at_position1<P, E: ParseError<Self>>(
    &self,
    predicate: P,
    e: ErrorKind,
  ) -> IResult<Self, Self, E>
  where
    P: Fn(Self::Item) -> bool {
        match self.deref().position(predicate) {
            Some(0) => Err(Err::Error(E::from_error_kind(self.clone(), e))),
            Some(n) => Ok(self.take_split(n)),
            None => Err(Err::Incomplete(nom::Needed::new(1))),
        }
    }

    fn split_at_position_complete<P, E: ParseError<Self>>(
    &self,
    predicate: P,
  ) -> IResult<Self, Self, E>
  where
    P: Fn(Self::Item) -> bool {
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
    P: Fn(Self::Item) -> bool {
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
impl<'src, 'ctx, U> FindSubstring<U> for  Z80Span<'src, 'ctx>
where
    &'src str: FindSubstring<U>,
{
    #[inline]
    fn find_substring(&self, substr: U) -> Option<usize> {
        self.fragment().find_substring(substr)
    }
}

impl<'src, 'ctx> Slice<std::ops::Range<usize>> for  Z80Span<'src, 'ctx> {
    fn slice(&self, range: std::ops::Range<usize>) -> Self {
        Self(self.deref().slice(range))
    }
}
impl<'src, 'ctx> Slice<std::ops::RangeFrom<usize>> for  Z80Span<'src, 'ctx> {
    fn slice(&self, range: std::ops::RangeFrom<usize>) -> Self {
        Self(self.deref().slice(range))
    }
}
impl<'src, 'ctx> Slice<std::ops::RangeTo<usize>> for  Z80Span<'src, 'ctx> {
    fn slice(&self, range: std::ops::RangeTo<usize>) -> Self {
        Self(self.deref().slice(range))
    }
}

impl<'src, 'ctx>  Z80Span<'src, 'ctx> {
    pub fn new_extra(src: &'src str, ctx: &'ctx ParserContext) -> Self {
        Self(LocatedSpan::new_extra(src, ctx))
    }
}
