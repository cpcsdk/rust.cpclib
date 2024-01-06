use crate::error::AssemblerError;

/// Manage the stack of stable counters.
/// They are updated each time an opcode is visited as well
#[derive(Default, Clone)]
pub struct StableTickerCounters {
    counters: Vec<(String, usize)>
}

#[allow(missing_docs)]
impl StableTickerCounters {
    pub fn new_pass(&mut self) {
        self.counters.clear();
    }

    /// Check if a counter with the same name already exists
    pub fn has_counter<S: AsRef<str>>(&self, name: &S) -> bool {
        let name = name.as_ref();
        self.counters.iter().any(|(s, _)| s == name)
    }

    /// Add a new counter if no counter has the same name
    pub fn add_counter<S: AsRef<str>>(&mut self, name: &S) -> Result<(), AssemblerError> {
        let name: String = name.as_ref().to_owned();
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

    pub fn release_counter(&mut self, name: &str) -> Option<(String, usize)> {
        if let Some(idx) = self.counters.iter().position(|c| c.0 == name) {
            Some(self.counters.remove(idx))
        }
        else {
            return None;
        }
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
