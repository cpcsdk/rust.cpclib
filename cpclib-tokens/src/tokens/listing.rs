use crate::symbols::SymbolsTableCaseDependent;
use std::ops::Deref;
use std::ops::DerefMut;

use std::iter::FromIterator;
use core::fmt::Debug;

/// The ListingElement trati contains the public method any memeber of a listing should contain
/// ATM there is nothing really usefull
pub trait ListingElement {

    
}

/// A listing is simply a list of things similar to token
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseListing<T: Clone + ListingElement> {
    /// Ordered list of the tokens
    pub(crate) listing: Vec<T>,
    /// Duration of the listing execution. Manually set by user
    pub(crate) duration: Option<usize>,
}

impl<T: Clone + ListingElement> From<Vec<T>> for BaseListing<T> {
    fn from(listing: Vec<T>) -> Self {
        Self {
            listing,
            duration: None,
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
            duration: None,
        }
    }
}



impl<T: Clone + ListingElement + Debug> FromIterator<T> for BaseListing<T> {
    fn from_iter<I: IntoIterator<Item=T>>(src: I) -> Self {
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
