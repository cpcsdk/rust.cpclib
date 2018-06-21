use std::ops::Deref;
use std::ops::DerefMut;
use assembler::assembler::SymbolsTable;



/// The ListingElement trati contains the public method any memeber of a listing should contain
pub trait ListingElement {
    /// Estimate the duration of the token
    fn estimated_duration(&self) -> usize;

    /// Return the number of bytes of the token
    fn number_of_bytes(&self) -> Result<usize, String>;


    /// Return the number of bytes given the context (needed for Align)
    fn number_of_bytes_with_context(&self, table: &SymbolsTable) -> Result<usize, String>;
}






/// A listing is simply a list of things similar to token
#[derive(Debug, Clone, PartialEq)]
pub struct BaseListing<T: Clone + ListingElement> {

    /// Ordered list of the tokens
    listing: Vec<T>,
    /// Duration of the listing execution. Manually set by user
    duration: Option<usize>
}





impl<T: Clone + ListingElement> Deref for BaseListing<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.listing
    }
}


impl<T: Clone + ListingElement> DerefMut for BaseListing<T> {
    fn deref_mut(&mut self) -> &mut Vec<T>{
        &mut self.listing
    }
}


impl<T: Clone + ListingElement + ::std::fmt::Debug> BaseListing<T> {

    /// Create an empty listing without duration
    pub fn new() -> Self{
        BaseListing::<T>{
            listing: Vec::new(),
            duration: None
        }
    }

    /// Write access to listing. Should not exist but I do not know how to access to private firlds
    /// from trait implementation
    pub fn mut_listing(&mut self) -> &mut Vec<T> {
       &mut self.listing
    }

    pub fn listing(& self) -> & Vec<T> {
       & self.listing
    }




    /// Add a new token to the listing
    pub fn add(&mut self, token:T) {
        self.listing.push(token);
    }



    /// Consume another listing by injecting it
    pub fn inject_listing(&mut self, other: &Self) {
        self.listing.extend_from_slice(&other.listing);
    }






    /// Get the execution duration.
    /// If field `duration` is set, returns it. Otherwise, compute it
    pub fn estimated_duration(&self) -> usize {
        if self.duration.is_some() {
            self.duration.unwrap()
        }
        else {
            self.listing
                .iter()
                .map(|token|{token.estimated_duration()})
                .sum()
        }
    }


    pub fn set_duration(&mut self, duration: usize) {
        self.duration = Some(duration);
    }

    /// Get the token at the required position
    pub fn get(&self, idx: usize) -> Option<&T> {
        self.listing.get(idx)
    }
}



