use std::collections::HashMap;

use crate::error::AssemblerError;

/// Manage the stack of stable counters.
/// They are updated each time an opcode is visited as well.
/// Keep insertion order to support releasing the last counter.
#[derive(Default, Clone)]
pub struct StableTickerCounters {
    order: Vec<String>,
    counters: HashMap<String, usize>
}

#[allow(missing_docs)]
impl StableTickerCounters {
    pub fn new_pass(&mut self) {
        self.order.clear();
        self.counters.clear();
    }

    /// Check if a counter with the same name already exists
    pub fn has_counter<S: AsRef<str>>(&self, name: &S) -> bool {
        self.counters.contains_key(name.as_ref())
    }

    /// Add a new counter if no counter has the same name
    pub fn add_counter<S: AsRef<str>>(&mut self, name: &S) -> Result<(), Box<AssemblerError>> {
        let name: String = name.as_ref().to_owned();
        if self.has_counter(&name) {
            return Err(Box::new(AssemblerError::CounterAlreadyExists {
                symbol: name
            }));
        }
        self.order.push(name.clone());
        self.counters.insert(name, 0);
        Ok(())
    }

    /// Release the latest counter (if exists)
    pub fn release_last_counter(&mut self) -> Option<(String, usize)> {
        let key = self.order.pop()?;
        let value = self.counters.remove(&key)?;
        Some((key, value))
    }

    pub fn release_counter(&mut self, name: &str) -> Option<(String, usize)> {
        let removed = self.counters.remove(name).map(|v| (name.to_owned(), v));
        if removed.is_some() {
            self.order.retain(|k| k != name);
        }
        removed
    }

    /// Update each opened counters by count
    pub fn update_counters(&mut self, count: usize) {
        self.counters.values_mut().for_each(|local_count| {
            *local_count += count;
        });
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}
