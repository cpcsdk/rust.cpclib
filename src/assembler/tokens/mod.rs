use memchr;
use nom::{AtEof, Compare, CompareResult, FindSubstring, InputIter, InputLength};
use std::iter::Enumerate;
use std::slice::Iter;

pub(crate) mod data_access;
pub(crate) mod expression;
pub(crate) mod instructions;
pub(crate) mod listing;
pub(crate) mod registers;
pub(crate) mod tokens;

pub use self::data_access::*;
pub use self::expression::*;
pub use self::instructions::*;
pub use self::listing::*;
pub use self::registers::*;
pub use self::tokens::*;

/// Represent the type of the input elements.
pub type InputElement = u8;

/// Represent the type of the input.
pub type Input<'a> = &'a [InputElement];

use crate::assembler::parser;
use std::fmt;

// Stolen code from https://github.com/tagua-vm/parser/blob/737e8625e51580cb6d8aaecea5b2f04fefbccaa5/source/tokens.rs

/// A span is a set of meta information about a token.
///
/// The `Span` structure can be used as an input of the nom parsers.
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Span<'a> {
    /// The offset represents the position of the slice relatively to
    /// the input of the parser. It starts at offset 0.
    pub offset: usize,

    /// The line number of the slice relatively to the input of the
    /// parser. It starts at line 1.
    pub line: u32,

    /// The column number of the slice relatively to the input of the
    /// parser. It starts at column 1.
    pub column: u32,

    /// The slice that is spanned.
    slice: Input<'a>,
}

impl<'a> Span<'a> {
    /// Create a span for a particular input with default `offset`,
    /// `line`, and `column` values.
    ///
    /// `offset` starts at 0, `line` starts at 1, and `column` starts at 1.
    ///
    pub fn new(input: Input<'a>) -> Self {
        Span {
            offset: 0,
            line: 1,
            column: 1,
            slice: input,
        }
    }

    /// Create a span for a particular input at a particular offset, line, and column.
    ///
    /// # Examples
    ///
    pub fn new_at(input: Input<'a>, offset: usize, line: u32, column: u32) -> Self {
        Span {
            offset,
            line,
            column,
            slice: input,
        }
    }

    /// Create a blank span.
    /// This is strictly equivalent to `Span::new(b"")`.
    ///
    /// # Examples
    pub fn empty() -> Self {
        Self::new(b"")
    }

    /// Extract the entire slice of the span.
    ///
    /// # Examples
    pub fn as_slice(&self) -> Input<'a> {
        self.slice
    }
}

/// Implement `InputLength` from nom to be able to use the `Span`
/// structure as an input of the parsers.
///
/// This trait aims at computing the length of the input.
impl<'a> InputLength for Span<'a> {
    /// Compute the length of the slice in the span.
    ///
    /// # Examples
    ///
    fn input_len(&self) -> usize {
        self.slice.len()
    }
}

/// Implement `AtEof` from nom to be able to use the `Span` structure
/// as an input of the parsers.
///
/// This trait aims at determining whether the current span is at the
/// end of the input.
impl<'a> AtEof for Span<'a> {
    fn at_eof(&self) -> bool {
        self.slice.at_eof()
    }
}

/// Implement `InputIter` from nom to be able to use the `Span`
/// structure as an input of the parsers.
///
/// This trait aims at iterating over the input.
impl<'a> InputIter for Span<'a> {
    /// Type of an element of the span' slice.
    type Item = &'a InputElement;

    /// Type of a raw element of the span' slice.
    type RawItem = InputElement;

    /// Type of the enumerator iterator.
    type Iter = Enumerate<Iter<'a, Self::RawItem>>;

    /// Type of the iterator.
    type IterElem = Iter<'a, Self::RawItem>;

    /// Return an iterator that enumerates the byte offset and the
    /// element of the slice in the span.
    fn iter_indices(&self) -> Self::Iter {
        self.slice.iter().enumerate()
    }

    /// Return an iterator over the elements of the slice in the span.
    ///
    fn iter_elements(&self) -> Self::IterElem {
        self.slice.iter()
    }

    /// Find the byte position of an element in the slice of the span.
    ///
    fn position<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::RawItem) -> bool,
    {
        self.slice.iter().position(|x| predicate(*x))
    }

    /// Get the byte offset from the element's position in the slice
    /// of the span.
    ///
    fn slice_index(&self, count: usize) -> Option<usize> {
        if self.slice.len() >= count {
            Some(count)
        } else {
            None
        }
    }
}

/// Implement `FindSubstring` from nom to be able to use the `Span`
/// structure as an input of the parsers.
///
/// This traits aims at finding a substring in an input.
impl<'a, 'b> FindSubstring<Input<'b>> for Span<'a> {
    /// Find the position of a substring in the current span.
    ///
    fn find_substring(&self, substring: Input<'b>) -> Option<usize> {
        let substring_length = substring.len();

        if substring_length == 0 {
            None
        } else if substring_length == 1 {
            memchr::memchr(substring[0], self.slice)
        } else {
            let max = self.slice.len() - substring_length;
            let mut offset = 0;
            let mut haystack = self.slice;

            while let Some(position) = memchr::memchr(substring[0], haystack) {
                offset += position;

                if offset > max {
                    return None;
                }

                if &haystack[position..position + substring_length] == substring {
                    return Some(offset);
                }

                haystack = &haystack[position + 1..];
                offset += 1;
            }

            None
        }
    }
}

/// Implement `Compare` from nom to be able to use the `Span`
/// structure as an input of the parsers.
///
/// This trait aims at comparing inputs.
impl<'a, 'b> Compare<Input<'b>> for Span<'a> {
    /// Compare self to another input for equality.
    ///
    fn compare(&self, element: Input<'b>) -> CompareResult {
        self.slice.compare(element)
    }

    /// Compare self to another input for equality independently of the case.
    fn compare_no_case(&self, element: Input<'b>) -> CompareResult {
        self.slice.compare_no_case(element)
    }
}

#[cfg(test)]
mod test {
    use crate::assembler::tokens::{
        DataAccess, Expr, FlagTest, Listing, ListingElement, Mnemonic, Register16, Register8, Token,
    };
    use std::str::FromStr;
    #[test]
    fn test_size() {
        assert_eq!(
            Token::OpCode(
                Mnemonic::Jp,
                None,
                Some(DataAccess::Expression(Expr::Value(0)))
            )
            .number_of_bytes(),
            Ok(3)
        );

        assert_eq!(
            Token::OpCode(
                Mnemonic::Jr,
                None,
                Some(DataAccess::Expression(Expr::Value(0)))
            )
            .number_of_bytes(),
            Ok(2)
        );

        assert_eq!(
            Token::OpCode(
                Mnemonic::Jr,
                Some(DataAccess::FlagTest(FlagTest::NC)),
                Some(DataAccess::Expression(Expr::Value(0)))
            )
            .number_of_bytes(),
            Ok(2)
        );

        assert_eq!(
            Token::OpCode(
                Mnemonic::Push,
                Some(DataAccess::Register16(Register16::De)),
                None
            )
            .number_of_bytes(),
            Ok(1)
        );

        assert_eq!(
            Token::OpCode(
                Mnemonic::Dec,
                Some(DataAccess::Register8(Register8::A)),
                None
            )
            .number_of_bytes(),
            Ok(1)
        );
    }

    #[test]
    fn test_listing() {
        let mut listing = Listing::from_str("   nop").expect("unable to assemble");
        assert_eq!(listing.estimated_duration().unwrap(), 1);
        listing.set_duration(100);
        assert_eq!(listing.estimated_duration().unwrap(), 100);
    }

    #[test]
    fn test_duration() {
        let listing = Listing::from_str(
            "
            pop de      ; 3
        ",
        )
        .expect("Unable to assemble this code");
        println!("{}", listing);
        assert_eq!(listing.estimated_duration().unwrap(), 3);

        let listing = Listing::from_str(
            "
            inc l       ; 1
        ",
        )
        .expect("Unable to assemble this code");
        println!("{}", listing);
        assert_eq!(listing.estimated_duration().unwrap(), 1);

        let listing = Listing::from_str(
            "
            ld (hl), e  ; 2
        ",
        )
        .expect("Unable to assemble this code");
        println!("{}", listing);
        assert_eq!(listing.estimated_duration().unwrap(), 2);

        let listing = Listing::from_str(
            "
            ld (hl), d  ; 2
        ",
        )
        .expect("Unable to assemble this code");
        println!("{}", listing);
        assert_eq!(listing.estimated_duration().unwrap(), 2);

        let listing = Listing::from_str(
            "
            pop de      ; 3
            inc l       ; 1
            ld (hl), e  ; 2
            inc l       ; 1
            ld (hl), d  ; 2
        ",
        )
        .expect("Unable to assemble this code");
        println!("{}", listing);
        assert_eq!(listing.estimated_duration().unwrap(), (3 + 1 + 2 + 1 + 2));
    }
}
