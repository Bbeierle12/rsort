//! Arena-based record storage with memory tracking
//!
//! For Phase 1, this is a simple wrapper around Vec.
//! Future phases will add memory budgeting and external sort triggers.

/// Stores records with memory tracking
pub struct Arena {
    /// Raw record data
    records: Vec<Vec<u8>>,
    /// Total bytes stored
    total_bytes: usize,
}

impl Arena {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            total_bytes: 0,
        }
    }

    /// Add a record to the arena
    pub fn push(&mut self, record: Vec<u8>) {
        self.total_bytes += record.len();
        self.records.push(record);
    }

    /// Get total bytes stored
    pub fn bytes_used(&self) -> usize {
        self.total_bytes
    }

    /// Get number of records
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Check if arena is empty
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Get mutable access to records for sorting
    pub fn records_mut(&mut self) -> &mut Vec<Vec<u8>> {
        &mut self.records
    }

    /// Consume arena and return records
    pub fn into_records(self) -> Vec<Vec<u8>> {
        self.records
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<Vec<u8>> for Arena {
    fn from_iter<I: IntoIterator<Item = Vec<u8>>>(iter: I) -> Self {
        let mut arena = Arena::new();
        for record in iter {
            arena.push(record);
        }
        arena
    }
}
