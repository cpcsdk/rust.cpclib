use crate::assembler::assembler::SymbolsTableCaseDependent;
use std::ops::Deref;
use std::ops::DerefMut;

/// The ListingElement trati contains the public method any memeber of a listing should contain
pub trait ListingElement {
    /// Estimate the duration of the token
    fn estimated_duration(&self) -> Result<usize, String>;

    /// Return the number of bytes of the token
    fn number_of_bytes(&self) -> Result<usize, String>;

    /// Return the number of bytes given the context (needed for Align)
    fn number_of_bytes_with_context(
        &self,
        table: &mut SymbolsTableCaseDependent,
    ) -> Result<usize, String>;
}

/// A listing is simply a list of things similar to token
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseListing<T: Clone + ListingElement> {
    /// Ordered list of the tokens
    listing: Vec<T>,
    /// Duration of the listing execution. Manually set by user
    duration: Option<usize>,
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

#[allow(missing_docs)]
impl<T: Clone + ListingElement + ::std::fmt::Debug> BaseListing<T> {
    /// Create an empty listing without duration
    pub fn new() -> Self {
        Self {
            listing: Vec::new(),
            duration: None,
        }
    }

    /// Write access to listing. Should not exist but I do not know how to access to private firlds
    /// from trait implementation
    #[deprecated(note="use listing_mut instead")]
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

    /// Get the execution duration.
    /// If field `duration` is set, returns it. Otherwise, compute it
    pub fn estimated_duration(&self) -> Result<usize, String> {
        if let Some(duration) = self.duration {
            Ok(duration)
        } else {
            let mut duration = 0;
            for token in &self.listing {
                duration = duration + token.estimated_duration()?;
            }
            Ok(duration)
        }
    }

    pub fn set_duration(&mut self, duration: usize) {
        let duration = Some(duration);
        self.duration = duration;
    }

    /// Get the token at the required position
    pub fn get(&self, idx: usize) -> Option<&T> {
        self.listing.get(idx)
    }
}
