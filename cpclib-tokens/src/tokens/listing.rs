use core::fmt::Debug;

use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};

use cpclib_common::smallvec::SmallVec;

use crate::{
    BinaryTransformation, CrunchType, ExprElement, MacroParamElement,
    TestKindElement
};
/// The ListingElement trait contains the public method any memeber of a listing should contain
/// ATM there is nothing really usefull
pub trait ListingElement: Debug + Sized {
    type MacroParam: MacroParamElement;
    type TestKind: TestKindElement;
    type Expr: ExprElement;

    fn is_for(&self) -> bool;
    fn for_listing(&self) -> &[Self];
    fn for_label(&self) -> &str;
    fn for_start(&self) -> &Self::Expr;
    fn for_stop(&self) -> &Self::Expr;
    fn for_step(&self) -> Option<&Self::Expr>;


    fn is_repeat_until(&self) -> bool;
    fn repeat_until_listing(&self) -> &[Self];
    fn repeat_until_condition(&self) -> &Self::Expr;


    fn is_repeat(&self) -> bool;
    fn repeat_listing(&self) -> &[Self];
    fn repeat_count(&self) -> &Self::Expr;
    fn repeat_counter_name(&self) -> Option<&str>;
    fn repeat_counter_start(&self) -> Option<&Self::Expr>;

    fn is_crunched_section(&self) -> bool;
    fn crunched_section_listing(&self) -> &[Self];
    fn crunched_section_kind(&self) -> &CrunchType;

    fn is_macro_definition(&self) -> bool;
    fn macro_definition_name(&self) -> &str;
    fn macro_definition_arguments(&self) -> SmallVec<[&str; 4]>;
    fn macro_definition_code(&self) -> &str;

    fn is_call_macro_or_build_struct(&self) -> bool;
    fn macro_call_name(&self) -> &str;
    fn macro_call_arguments(&self) -> &[Self::MacroParam];

    fn is_if(&self) -> bool;
    fn if_nb_tests(&self) -> usize;
    fn if_test(&self, idx: usize) -> (&Self::TestKind, &[Self]);
    fn if_else(&self) -> Option<&[Self]>;

    fn is_incbin(&self) -> bool;
    fn incbin_fname(&self) -> &str;
    fn incbin_offset(&self) -> Option<&Self::Expr>;
    fn incbin_length(&self) -> Option<&Self::Expr>;
    fn incbin_transformation(&self) -> &BinaryTransformation;

    fn is_include(&self) -> bool;
    fn include_fname(&self) -> &str;
    fn include_namespace(&self) -> Option<&str>;
    fn include_once(&self) -> bool;

    fn is_function_definition(&self) -> bool;
    fn function_definition_name(&self) -> &str;
    fn function_definition_params(&self) -> SmallVec<[&str; 4]>;
    fn function_definition_inner(&self) -> &[Self];
}
/// A listing is simply a list of things similar to token
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseListing<T: Clone + ListingElement> {
    /// Ordered list of the tokens
    pub(crate) listing: Vec<T>,
    /// Duration of the listing execution. Manually set by user
    pub(crate) duration: Option<usize>
}

impl<T: Clone + ListingElement> From<Vec<T>> for BaseListing<T> {
    fn from(listing: Vec<T>) -> Self {
        Self {
            listing,
            duration: None
        }
    }
}

impl<T: Clone + ListingElement> Deref for BaseListing<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.listing
    }
}

impl<T: Clone + ListingElement> DerefMut for BaseListing<T> {
    fn deref_mut(&mut self) -> &mut Vec<T> {
        &mut self.listing
    }
}

impl<T: Clone + ListingElement> Default for BaseListing<T> {
    fn default() -> Self {
        Self {
            listing: Vec::new(),
            duration: None
        }
    }
}

impl<T: Clone + Debug + ListingElement> From<T> for BaseListing<T> {
    fn from(token: T) -> Self {
        let mut lst = Self::default();
        lst.add(token);
        lst
    }
}

impl<T: Clone + ListingElement + Debug> FromIterator<T> for BaseListing<T> {
    fn from_iter<I: IntoIterator<Item = T>>(src: I) -> Self {
        Self::new_with(&src.into_iter().collect::<Vec<T>>())
    }
}

#[allow(missing_docs)]
impl<T: Clone + ListingElement + ::std::fmt::Debug> BaseListing<T> {
    /// Create an empty listing without duration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new  listing based on the provided Ts
    pub fn new_with(arg: &[T]) -> Self {
        let mut new = Self::default();
        new.listing = arg.to_vec();
        new
    }

    /// Write access to listing. Should not exist but I do not know how to access to private firlds
    /// from trait implementation
    #[deprecated(note = "use listing_mut instead")]
    pub fn mut_listing(&mut self) -> &mut Vec<T> {
        &mut self.listing
    }

    pub fn listing_mut(&mut self) -> &mut Vec<T> {
        &mut self.listing
    }

    pub fn listing(&self) -> &[T] {
        &self.listing
    }

    /// Add a new token to the listing
    pub fn add(&mut self, token: T) {
        self.listing.push(token);
    }

    /// Consume another listing by injecting it
    pub fn inject_listing(&mut self, other: &Self) {
        self.listing.extend_from_slice(&other.listing);
    }

    pub fn set_duration(&mut self, duration: usize) {
        let duration = Some(duration);
        self.duration = duration;
    }

    pub fn duration(&self) -> Option<usize> {
        self.duration
    }

    /// Get the token at the required position
    pub fn get(&self, idx: usize) -> Option<&T> {
        self.listing.get(idx)
    }
}
