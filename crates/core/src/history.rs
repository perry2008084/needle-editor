use std::ops::Range;

use crate::BufferId;

#[derive(Debug, Clone)]
pub struct EditRecord {
    pub buffer_id: BufferId,
    pub range_start: usize,
    pub old_text: String,
    pub new_text: String,
}

pub type EditTransaction = Vec<EditRecord>;

impl EditRecord {
    pub fn undo_range(&self) -> Range<usize> {
        self.range_start..(self.range_start + self.new_text.len())
    }

    pub fn redo_range(&self) -> Range<usize> {
        self.range_start..(self.range_start + self.old_text.len())
    }
}
