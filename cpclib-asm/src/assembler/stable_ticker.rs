use std::borrow::Borrow;

use crate::error::AssemblerError;

/// Manage the stack of stable counters.
/// They are updated each time an opcode is visited
#[derive(Default, Clone)]
pub struct StableTickerCounters {
    counters: Vec<(String, usize)>
}

#[allow(missing_docs)]
impl StableTickerCounters {
    /// Check if a counter with the same name already exists
    pub fn has_counter<S: Borrow<str>>(&self, name: &S) -> bool {
        let name = name.borrow();
        self.counters.iter().any(|(s, _)| s == name)
    }

    /// Add a new counter if no counter has the same name
    pub fn add_counter<S: Borrow<str>>(&mut self, name: &S) -> Result<(), AssemblerError> {
        let name: String = name.borrow().to_owned();
        if self.has_counter(&name) {
            return Err(AssemblerError::CounterAlreadyExists { symbol: name });
        }
        self.counters.push((name, 0));
        Ok(())
    }

    /// Release the latest counter (if exists)
    pub fn release_last_counter(&mut self) -> Option<(String, usize)> {
        self.counters.pop()
    }

    /// Update each opened counters by count
    pub fn update_counters(&mut self, count: usize) {
        self.counters.iter_mut().for_each(|(_, local_count)| {
            *local_count += count;
        });
    }

    pub fn len(&self) -> usize {
        self.counters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
