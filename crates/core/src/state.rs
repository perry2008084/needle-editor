use std::{collections::HashMap, ops::Range};

use crate::{
    Buffer, BufferError, BufferId, EditRecord, EditTransaction, Region, View, ViewId, WindowId,
};

#[derive(Debug)]
pub struct AppState {
    app_name: String,
    active_window: WindowId,
    active_view: Option<ViewId>,
    buffers: HashMap<BufferId, Buffer>,
    views: HashMap<ViewId, View>,
    undo_stack: Vec<EditTransaction>,
    redo_stack: Vec<EditTransaction>,
}

impl AppState {
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            active_window: WindowId::next(),
            active_view: None,
            buffers: HashMap::new(),
            views: HashMap::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn app_name(&self) -> &str {
        &self.app_name
    }

    pub fn active_window(&self) -> WindowId {
        self.active_window
    }

    pub fn active_view_id(&self) -> Option<ViewId> {
        self.active_view
    }

    pub fn create_empty_buffer_and_view(&mut self) -> (BufferId, ViewId) {
        let buffer = Buffer::new_empty();
        let buffer_id = buffer.id();

        let mut view = View::new(buffer_id);
        view.sel_mut().add(Region::new(0, 0));
        let view_id = view.id();

        self.buffers.insert(buffer_id, buffer);
        self.views.insert(view_id, view);
        self.active_view = Some(view_id);

        (buffer_id, view_id)
    }

    pub fn set_active_view(&mut self, view_id: ViewId) {
        self.active_view = Some(view_id);
    }

    pub fn buffer(&self, id: BufferId) -> Option<&Buffer> {
        self.buffers.get(&id)
    }

    pub fn buffer_mut(&mut self, id: BufferId) -> Option<&mut Buffer> {
        self.buffers.get_mut(&id)
    }

    pub fn view(&self, id: ViewId) -> Option<&View> {
        self.views.get(&id)
    }

    pub fn view_mut(&mut self, id: ViewId) -> Option<&mut View> {
        self.views.get_mut(&id)
    }

    pub fn active_view(&self) -> Option<&View> {
        self.active_view.and_then(|id| self.views.get(&id))
    }

    pub fn active_buffer_id(&self) -> Option<BufferId> {
        self.active_view().map(|view| view.buffer_id())
    }

    pub fn active_buffer(&self) -> Option<&Buffer> {
        let view = self.active_view()?;
        self.buffer(view.buffer_id())
    }

    pub fn active_buffer_mut(&mut self) -> Option<&mut Buffer> {
        let buffer_id = self.active_buffer_id()?;
        self.buffer_mut(buffer_id)
    }

    pub fn active_selection_regions(&self) -> Option<Vec<Region>> {
        Some(self.active_view()?.sel().as_slice().to_vec())
    }

    pub fn set_active_selections<I>(&mut self, regions: I) -> Result<(), BufferError>
    where
        I: IntoIterator<Item = Region>,
    {
        let view_id = self.active_view_id().ok_or(BufferError::NoActiveView)?;
        let view = self.view_mut(view_id).ok_or(BufferError::NoActiveView)?;
        view.set_selections(regions);
        Ok(())
    }

    pub fn move_caret_left(&mut self) -> Result<usize, BufferError> {
        let selections = self.active_selection_regions().ok_or(BufferError::NoActiveView)?;
        let new_regions = {
            let buffer = self.active_buffer().ok_or(BufferError::NoActiveView)?;
            let mut out = Vec::with_capacity(selections.len());
            for region in selections {
                let caret = if !region.empty() {
                    region.begin()
                } else {
                    buffer.prev_char_start(region.caret())?.unwrap_or(0)
                };
                out.push(Region::new(caret, caret));
            }
            out
        };
        self.set_active_selections(new_regions)?;
        Ok(self.active_view().map(|view| view.selection_count()).unwrap_or(0))
    }

    pub fn move_caret_right(&mut self) -> Result<usize, BufferError> {
        let selections = self.active_selection_regions().ok_or(BufferError::NoActiveView)?;
        let new_regions = {
            let buffer = self.active_buffer().ok_or(BufferError::NoActiveView)?;
            let mut out = Vec::with_capacity(selections.len());
            for region in selections {
                let caret = if !region.empty() {
                    region.end()
                } else {
                    buffer
                        .next_char_end(region.caret())?
                        .unwrap_or_else(|| buffer.len())
                };
                out.push(Region::new(caret, caret));
            }
            out
        };
        self.set_active_selections(new_regions)?;
        Ok(self.active_view().map(|view| view.selection_count()).unwrap_or(0))
    }

    pub fn move_to_beginning_of_line(&mut self) -> Result<usize, BufferError> {
        let selections = self.active_selection_regions().ok_or(BufferError::NoActiveView)?;
        let new_regions = {
            let buffer = self.active_buffer().ok_or(BufferError::NoActiveView)?;
            selections
                .into_iter()
                .map(|region| {
                    let point = region.caret().min(buffer.len());
                    let line = buffer.line(point)?;
                    Ok(Region::new(line.begin(), line.begin()))
                })
                .collect::<Result<Vec<_>, BufferError>>()?
        };
        self.set_active_selections(new_regions)?;
        Ok(self.active_view().map(|view| view.selection_count()).unwrap_or(0))
    }

    pub fn move_to_end_of_line(&mut self) -> Result<usize, BufferError> {
        let selections = self.active_selection_regions().ok_or(BufferError::NoActiveView)?;
        let new_regions = {
            let buffer = self.active_buffer().ok_or(BufferError::NoActiveView)?;
            selections
                .into_iter()
                .map(|region| {
                    let point = region.caret().min(buffer.len());
                    let line = buffer.line(point)?;
                    Ok(Region::new(line.end(), line.end()))
                })
                .collect::<Result<Vec<_>, BufferError>>()?
        };
        self.set_active_selections(new_regions)?;
        Ok(self.active_view().map(|view| view.selection_count()).unwrap_or(0))
    }

    pub fn replace_active_selections(&mut self, new_text: &str) -> Result<usize, BufferError> {
        let view_id = self.active_view_id().ok_or(BufferError::NoActiveView)?;
        let view = self.view(view_id).ok_or(BufferError::NoActiveView)?;
        let buffer_id = view.buffer_id();
        let selections = if view.sel().is_empty() {
            vec![Region::new(0, 0)]
        } else {
            view.sel().as_slice().to_vec()
        };

        let edits: Vec<(Range<usize>, String)> = selections
            .iter()
            .rev()
            .map(|region| (region.begin()..region.end(), new_text.to_string()))
            .collect();

        let transaction = self.apply_edits(buffer_id, edits)?;
        let new_regions = transaction
            .iter()
            .map(|record| {
                let caret = record.range_start + record.new_text.len();
                Region::new(caret, caret)
            })
            .collect::<Vec<_>>();

        self.set_active_selections(new_regions)?;
        Ok(transaction.len())
    }

    pub fn delete_backward_from_active_selections(&mut self) -> Result<usize, BufferError> {
        let view_id = self.active_view_id().ok_or(BufferError::NoActiveView)?;
        let view = self.view(view_id).ok_or(BufferError::NoActiveView)?;
        let buffer_id = view.buffer_id();
        let buffer = self.buffer(buffer_id).ok_or(BufferError::NoActiveView)?;

        let mut edits: Vec<(Range<usize>, String)> = Vec::new();
        for region in view.sel().as_slice().iter().rev() {
            let range = if !region.empty() {
                region.begin()..region.end()
            } else if let Some(prev) = buffer.prev_char_start(region.caret())? {
                prev..region.caret()
            } else {
                continue;
            };
            edits.push((range, String::new()));
        }

        if edits.is_empty() {
            return Ok(0);
        }

        let transaction = self.apply_edits(buffer_id, edits)?;
        let new_regions = transaction
            .iter()
            .map(|record| Region::new(record.range_start, record.range_start))
            .collect::<Vec<_>>();
        self.set_active_selections(new_regions)?;
        Ok(transaction.len())
    }

    pub fn delete_forward_from_active_selections(&mut self) -> Result<usize, BufferError> {
        let view_id = self.active_view_id().ok_or(BufferError::NoActiveView)?;
        let view = self.view(view_id).ok_or(BufferError::NoActiveView)?;
        let buffer_id = view.buffer_id();
        let buffer = self.buffer(buffer_id).ok_or(BufferError::NoActiveView)?;

        let mut edits: Vec<(Range<usize>, String)> = Vec::new();
        for region in view.sel().as_slice().iter().rev() {
            let range = if !region.empty() {
                region.begin()..region.end()
            } else if let Some(next_end) = buffer.next_char_end(region.caret())? {
                region.caret()..next_end
            } else {
                continue;
            };
            edits.push((range, String::new()));
        }

        if edits.is_empty() {
            return Ok(0);
        }

        let transaction = self.apply_edits(buffer_id, edits)?;
        let new_regions = transaction
            .iter()
            .map(|record| Region::new(record.range_start, record.range_start))
            .collect::<Vec<_>>();
        self.set_active_selections(new_regions)?;
        Ok(transaction.len())
    }

    pub fn insert_line_after(&mut self, text: &str) -> Result<usize, BufferError> {
        let view_id = self.active_view_id().ok_or(BufferError::NoActiveView)?;
        let view = self.view(view_id).ok_or(BufferError::NoActiveView)?;
        let buffer_id = view.buffer_id();
        let buffer = self.buffer(buffer_id).ok_or(BufferError::NoActiveView)?;

        let mut targets: Vec<(usize, String, usize)> = view
            .sel()
            .as_slice()
            .iter()
            .map(|region| {
                let point = region.caret().min(buffer.len());
                let line = buffer.line(point)?;
                let full_line = buffer.full_line(point)?;
                let has_newline = full_line.end() > line.end();
                let insert_at = full_line.end();
                let (insert_text, caret) = if has_newline {
                    if text.is_empty() {
                        ("\n".to_string(), insert_at)
                    } else {
                        (text.to_string(), insert_at + text.len())
                    }
                } else if text.is_empty() {
                    ("\n".to_string(), insert_at + 1)
                } else {
                    (format!("\n{text}"), insert_at + 1 + text.len())
                };
                Ok((insert_at, insert_text, caret))
            })
            .collect::<Result<Vec<_>, BufferError>>()?;

        targets.sort_by_key(|(insert_at, _, _)| *insert_at);
        targets.dedup_by(|a, b| a.0 == b.0);

        let edits = targets
            .iter()
            .rev()
            .map(|(insert_at, insert_text, _)| (*insert_at..*insert_at, insert_text.clone()))
            .collect::<Vec<_>>();
        let _transaction = self.apply_edits(buffer_id, edits)?;

        let carets = targets
            .into_iter()
            .map(|(_, _, caret)| Region::new(caret, caret))
            .collect::<Vec<_>>();
        self.set_active_selections(carets)?;
        Ok(self.active_view().map(|view| view.selection_count()).unwrap_or(0))
    }

    pub fn insert_line_before(&mut self, text: &str) -> Result<usize, BufferError> {
        let view_id = self.active_view_id().ok_or(BufferError::NoActiveView)?;
        let view = self.view(view_id).ok_or(BufferError::NoActiveView)?;
        let buffer_id = view.buffer_id();
        let buffer = self.buffer(buffer_id).ok_or(BufferError::NoActiveView)?;

        let mut targets: Vec<(usize, String, usize)> = view
            .sel()
            .as_slice()
            .iter()
            .map(|region| {
                let point = region.caret().min(buffer.len());
                let line = buffer.line(point)?;
                let insert_at = line.begin();
                let insert_text = if text.is_empty() {
                    "\n".to_string()
                } else {
                    format!("{text}\n")
                };
                let caret = insert_at + text.len();
                Ok((insert_at, insert_text, caret))
            })
            .collect::<Result<Vec<_>, BufferError>>()?;

        targets.sort_by_key(|(insert_at, _, _)| *insert_at);
        targets.dedup_by(|a, b| a.0 == b.0);

        let edits = targets
            .iter()
            .rev()
            .map(|(insert_at, insert_text, _)| (*insert_at..*insert_at, insert_text.clone()))
            .collect::<Vec<_>>();
        let _transaction = self.apply_edits(buffer_id, edits)?;

        let carets = targets
            .into_iter()
            .map(|(_, _, caret)| Region::new(caret, caret))
            .collect::<Vec<_>>();
        self.set_active_selections(carets)?;
        Ok(self.active_view().map(|view| view.selection_count()).unwrap_or(0))
    }

    pub fn select_all(&mut self) -> Result<Region, BufferError> {
        let buffer = self.active_buffer().ok_or(BufferError::NoActiveView)?;
        let region = Region::new(0, buffer.len());
        self.set_active_selections([region])?;
        Ok(region)
    }

    pub fn apply_edit(
        &mut self,
        buffer_id: BufferId,
        range: Range<usize>,
        new_text: &str,
    ) -> Result<EditRecord, BufferError> {
        let transaction = self.apply_edits(buffer_id, vec![(range, new_text.to_string())])?;
        Ok(transaction.into_iter().next().expect("single edit transaction"))
    }

    pub fn apply_edits(
        &mut self,
        buffer_id: BufferId,
        edits: Vec<(Range<usize>, String)>,
    ) -> Result<EditTransaction, BufferError> {
        let mut transaction = Vec::with_capacity(edits.len());

        for (range, new_text) in edits {
            let old_text = self
                .buffer(buffer_id)
                .ok_or(BufferError::InvalidRange {
                    start: range.start,
                    end: range.end,
                    len: 0,
                })?
                .substr(range.clone())?
                .to_string();

            self.apply_edit_without_history(buffer_id, range.clone(), &new_text)?;

            transaction.push(EditRecord {
                buffer_id,
                range_start: range.start,
                old_text,
                new_text,
            });
        }

        if !transaction.is_empty() {
            self.undo_stack.push(transaction.clone());
            self.redo_stack.clear();
        }

        Ok(transaction)
    }

    pub fn undo(&mut self) -> Result<Option<EditTransaction>, BufferError> {
        let Some(transaction) = self.undo_stack.pop() else {
            return Ok(None);
        };

        for record in transaction.iter().rev() {
            self.apply_edit_without_history(record.buffer_id, record.undo_range(), &record.old_text)?;
        }
        self.redo_stack.push(transaction.clone());
        Ok(Some(transaction))
    }

    pub fn redo(&mut self) -> Result<Option<EditTransaction>, BufferError> {
        let Some(transaction) = self.redo_stack.pop() else {
            return Ok(None);
        };

        for record in &transaction {
            self.apply_edit_without_history(record.buffer_id, record.redo_range(), &record.new_text)?;
        }
        self.undo_stack.push(transaction.clone());
        Ok(Some(transaction))
    }

    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.redo_stack.len()
    }

    pub fn buffer_count(&self) -> usize {
        self.buffers.len()
    }

    pub fn view_count(&self) -> usize {
        self.views.len()
    }

    fn apply_edit_without_history(
        &mut self,
        buffer_id: BufferId,
        range: Range<usize>,
        new_text: &str,
    ) -> Result<(), BufferError> {
        self.buffer_mut(buffer_id)
            .ok_or(BufferError::InvalidRange {
                start: range.start,
                end: range.end,
                len: 0,
            })?
            .replace(range, new_text)
    }
}

#[cfg(test)]
mod tests {
    use super::AppState;
    use crate::Region;

    #[test]
    fn app_state_undo_redo_roundtrip() {
        let mut state = AppState::new("Needle");
        let (buffer_id, _view_id) = state.create_empty_buffer_and_view();

        state.apply_edit(buffer_id, 0..0, "hello").unwrap();
        assert_eq!(state.buffer(buffer_id).unwrap().content(), "hello");

        state.undo().unwrap();
        assert_eq!(state.buffer(buffer_id).unwrap().content(), "");

        state.redo().unwrap();
        assert_eq!(state.buffer(buffer_id).unwrap().content(), "hello");
    }

    #[test]
    fn replace_active_selections_updates_all_carets() {
        let mut state = AppState::new("Needle");
        let (buffer_id, view_id) = state.create_empty_buffer_and_view();
        state.apply_edit(buffer_id, 0..0, "abc\ndef").unwrap();
        let view = state.view_mut(view_id).unwrap();
        view.set_selections([Region::new(1, 1), Region::new(5, 5)]);

        state.replace_active_selections("X").unwrap();

        assert_eq!(state.buffer(buffer_id).unwrap().content(), "aXbc\ndXef");
        let sels = state.active_selection_regions().unwrap();
        assert_eq!(sels, vec![Region::new(2, 2), Region::new(6, 6)]);
    }

    #[test]
    fn delete_backward_from_empty_selection_deletes_previous_char() {
        let mut state = AppState::new("Needle");
        let (buffer_id, view_id) = state.create_empty_buffer_and_view();
        state.apply_edit(buffer_id, 0..0, "ab").unwrap();
        state
            .view_mut(view_id)
            .unwrap()
            .set_single_selection(Region::new(2, 2));

        state.delete_backward_from_active_selections().unwrap();

        assert_eq!(state.buffer(buffer_id).unwrap().content(), "a");
        assert_eq!(state.active_selection_regions().unwrap(), vec![Region::new(1, 1)]);
    }

    #[test]
    fn delete_forward_from_empty_selection_deletes_next_char() {
        let mut state = AppState::new("Needle");
        let (buffer_id, view_id) = state.create_empty_buffer_and_view();
        state.apply_edit(buffer_id, 0..0, "ab").unwrap();
        state
            .view_mut(view_id)
            .unwrap()
            .set_single_selection(Region::new(0, 0));

        state.delete_forward_from_active_selections().unwrap();

        assert_eq!(state.buffer(buffer_id).unwrap().content(), "b");
        assert_eq!(state.active_selection_regions().unwrap(), vec![Region::new(0, 0)]);
    }

    #[test]
    fn move_to_line_boundaries_updates_caret() {
        let mut state = AppState::new("Needle");
        let (buffer_id, view_id) = state.create_empty_buffer_and_view();
        state.apply_edit(buffer_id, 0..0, "abc\ndef").unwrap();
        state
            .view_mut(view_id)
            .unwrap()
            .set_single_selection(Region::new(5, 5));

        state.move_to_beginning_of_line().unwrap();
        assert_eq!(state.active_selection_regions().unwrap(), vec![Region::new(4, 4)]);

        state.move_to_end_of_line().unwrap();
        assert_eq!(state.active_selection_regions().unwrap(), vec![Region::new(7, 7)]);
    }

    #[test]
    fn insert_line_before_and_after_work() {
        let mut state = AppState::new("Needle");
        let (buffer_id, view_id) = state.create_empty_buffer_and_view();
        state.apply_edit(buffer_id, 0..0, "abc").unwrap();
        state
            .view_mut(view_id)
            .unwrap()
            .set_single_selection(Region::new(1, 1));

        state.insert_line_before("").unwrap();
        assert_eq!(state.buffer(buffer_id).unwrap().content(), "\nabc");
        assert_eq!(state.active_selection_regions().unwrap(), vec![Region::new(0, 0)]);

        state.insert_line_after("").unwrap();
        assert_eq!(state.buffer(buffer_id).unwrap().content(), "\n\nabc");
        assert_eq!(state.active_selection_regions().unwrap(), vec![Region::new(1, 1)]);
    }
}
