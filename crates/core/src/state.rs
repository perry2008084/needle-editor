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

    pub fn view_ids(&self) -> Vec<ViewId> {
        let mut ids: Vec<_> = self.views.keys().copied().collect();
        ids.sort();
        ids
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

    pub fn move_caret_up(&mut self) -> Result<usize, BufferError> {
        let selections = self.active_selection_regions().ok_or(BufferError::NoActiveView)?;
        let new_regions = {
            let buffer = self.active_buffer().ok_or(BufferError::NoActiveView)?;
            let mut out = Vec::with_capacity(selections.len());
            for region in selections {
                let point = region.caret().min(buffer.len());
                let (row, col) = buffer.row_col(point)?;
                if row == 0 {
                    out.push(Region::new(0, 0));
                } else {
                    let target = buffer.text_point(row - 1, col)?;
                    out.push(Region::new(target, target));
                }
            }
            out
        };
        self.set_active_selections(new_regions)?;
        Ok(self.active_view().map(|view| view.selection_count()).unwrap_or(0))
    }

    pub fn move_caret_down(&mut self) -> Result<usize, BufferError> {
        let selections = self.active_selection_regions().ok_or(BufferError::NoActiveView)?;
        let new_regions = {
            let buffer = self.active_buffer().ok_or(BufferError::NoActiveView)?;
            let mut out = Vec::with_capacity(selections.len());
            for region in selections {
                let point = region.caret().min(buffer.len());
                let (row, col) = buffer.row_col(point)?;
                if row + 1 >= buffer.line_count() {
                    out.push(Region::new(buffer.len(), buffer.len()));
                } else {
                    let target = buffer.text_point(row + 1, col)?;
                    out.push(Region::new(target, target));
                }
            }
            out
        };
        self.set_active_selections(new_regions)?;
        Ok(self.active_view().map(|view| view.selection_count()).unwrap_or(0))
    }

    pub fn select_current_lines(&mut self) -> Result<usize, BufferError> {
        let selections = self.active_selection_regions().ok_or(BufferError::NoActiveView)?;
        let new_regions = {
            let buffer = self.active_buffer().ok_or(BufferError::NoActiveView)?;
            selections
                .into_iter()
                .map(|region| {
                    let point = region.caret().min(buffer.len());
                    buffer.full_line(point)
                })
                .collect::<Result<Vec<_>, BufferError>>()?
        };
        self.set_active_selections(new_regions)?;
        Ok(self.active_view().map(|view| view.selection_count()).unwrap_or(0))
    }

    pub fn duplicate_current_lines(&mut self) -> Result<usize, BufferError> {
        let view_id = self.active_view_id().ok_or(BufferError::NoActiveView)?;
        let view = self.view(view_id).ok_or(BufferError::NoActiveView)?;
        let buffer_id = view.buffer_id();
        let buffer = self.buffer(buffer_id).ok_or(BufferError::NoActiveView)?;

        let mut targets: Vec<(usize, String, Region)> = view
            .sel()
            .as_slice()
            .iter()
            .map(|region| {
                let point = region.caret().min(buffer.len());
                let line = buffer.line(point)?;
                let full_line = buffer.full_line(point)?;
                let line_text = buffer.substr(line.begin()..line.end())?;
                let full_text = buffer.substr(full_line.begin()..full_line.end())?;
                let insert_text = if full_line.end() > line.end() {
                    full_text.to_string()
                } else {
                    format!("\n{line_text}")
                };
                let new_line = Region::new(full_line.end(), full_line.end() + insert_text.len());
                Ok((full_line.end(), insert_text, new_line))
            })
            .collect::<Result<Vec<_>, BufferError>>()?;

        targets.sort_by_key(|(insert_at, _, _)| *insert_at);
        targets.dedup_by(|a, b| a.0 == b.0);

        let edits = targets
            .iter()
            .rev()
            .map(|(insert_at, insert_text, _)| (*insert_at..*insert_at, insert_text.clone()))
            .collect::<Vec<_>>();
        self.apply_edits(buffer_id, edits)?;

        self.set_active_selections(targets.into_iter().map(|(_, _, region)| region))?;
        Ok(self.active_view().map(|view| view.selection_count()).unwrap_or(0))
    }

    pub fn delete_to_beginning_of_line(&mut self) -> Result<usize, BufferError> {
        let view_id = self.active_view_id().ok_or(BufferError::NoActiveView)?;
        let view = self.view(view_id).ok_or(BufferError::NoActiveView)?;
        let buffer_id = view.buffer_id();
        let buffer = self.buffer(buffer_id).ok_or(BufferError::NoActiveView)?;

        let mut edits: Vec<(Range<usize>, String)> = Vec::new();
        for region in view.sel().as_slice().iter().rev() {
            let range = if !region.empty() {
                region.begin()..region.end()
            } else {
                let point = region.caret().min(buffer.len());
                let line = buffer.line(point)?;
                line.begin()..point
            };
            if !range.is_empty() {
                edits.push((range, String::new()));
            }
        }

        if edits.is_empty() {
            return Ok(0);
        }

        let transaction = self.apply_edits(buffer_id, edits)?;
        self.set_active_selections(
            transaction
                .iter()
                .map(|record| Region::new(record.range_start, record.range_start)),
        )?;
        Ok(transaction.len())
    }

    pub fn delete_to_end_of_line(&mut self) -> Result<usize, BufferError> {
        let view_id = self.active_view_id().ok_or(BufferError::NoActiveView)?;
        let view = self.view(view_id).ok_or(BufferError::NoActiveView)?;
        let buffer_id = view.buffer_id();
        let buffer = self.buffer(buffer_id).ok_or(BufferError::NoActiveView)?;

        let mut edits: Vec<(Range<usize>, String)> = Vec::new();
        for region in view.sel().as_slice().iter().rev() {
            let range = if !region.empty() {
                region.begin()..region.end()
            } else {
                let point = region.caret().min(buffer.len());
                let line = buffer.line(point)?;
                point..line.end()
            };
            if !range.is_empty() {
                edits.push((range, String::new()));
            }
        }

        if edits.is_empty() {
            return Ok(0);
        }

        let transaction = self.apply_edits(buffer_id, edits)?;
        self.set_active_selections(
            transaction
                .iter()
                .map(|record| Region::new(record.range_start, record.range_start)),
        )?;
        Ok(transaction.len())
    }

    pub fn goto_line(&mut self, one_based_line: usize) -> Result<usize, BufferError> {
        if one_based_line == 0 {
            return Err(BufferError::InvalidRow { row: 0 });
        }
        let buffer = self.active_buffer().ok_or(BufferError::NoActiveView)?;
        let point = buffer.line_start(one_based_line - 1)?;
        self.set_active_selections([Region::new(point, point)])?;
        Ok(1)
    }

    pub fn split_selection_into_lines(&mut self) -> Result<usize, BufferError> {
        let selections = self.active_selection_regions().ok_or(BufferError::NoActiveView)?;
        let new_regions = {
            let buffer = self.active_buffer().ok_or(BufferError::NoActiveView)?;
            let mut out = Vec::new();
            for region in selections {
                if region.empty() {
                    out.push(region);
                    continue;
                }
                let mut point = region.begin();
                while point < region.end() {
                    let line = buffer.line(point)?;
                    out.push(Region::new(line.begin(), line.begin()));
                    let full_line = buffer.full_line(point)?;
                    if full_line.end() <= point || full_line.end() >= region.end() {
                        break;
                    }
                    point = full_line.end();
                }
            }
            out
        };
        self.set_active_selections(new_regions)?;
        Ok(self.active_view().map(|view| view.selection_count()).unwrap_or(0))
    }

    pub fn move_selected_lines_up(&mut self) -> Result<usize, BufferError> {
        let view_id = self.active_view_id().ok_or(BufferError::NoActiveView)?;
        let view = self.view(view_id).ok_or(BufferError::NoActiveView)?;
        let buffer_id = view.buffer_id();
        let buffer = self.buffer(buffer_id).ok_or(BufferError::NoActiveView)?;
        let blocks = self.selected_line_blocks(buffer)?;

        let mut edits: Vec<(Range<usize>, String)> = Vec::new();
        let mut new_regions = Vec::new();

        for block in blocks {
            if block.begin() == 0 {
                new_regions.push(block);
                continue;
            }
            let prev_line = buffer.full_line(block.begin() - 1)?;
            let prev_text = buffer.substr(prev_line.begin()..prev_line.end())?.to_string();
            let block_text = buffer.substr(block.begin()..block.end())?.to_string();
            edits.push((
                prev_line.begin()..block.end(),
                format!("{}{}", block_text, prev_text),
            ));
            new_regions.push(Region::new(
                prev_line.begin(),
                prev_line.begin() + block_text.len(),
            ));
        }

        if edits.is_empty() {
            return Ok(0);
        }

        edits.sort_by(|a, b| b.0.start.cmp(&a.0.start));
        self.apply_edits(buffer_id, edits)?;
        self.set_active_selections(new_regions)?;
        Ok(self.active_view().map(|view| view.selection_count()).unwrap_or(0))
    }

    pub fn move_selected_lines_down(&mut self) -> Result<usize, BufferError> {
        let view_id = self.active_view_id().ok_or(BufferError::NoActiveView)?;
        let view = self.view(view_id).ok_or(BufferError::NoActiveView)?;
        let buffer_id = view.buffer_id();
        let buffer = self.buffer(buffer_id).ok_or(BufferError::NoActiveView)?;
        let blocks = self.selected_line_blocks(buffer)?;

        let mut edits: Vec<(Range<usize>, String)> = Vec::new();
        let mut new_regions = Vec::new();

        for block in blocks.into_iter().rev() {
            if block.end() >= buffer.len() {
                new_regions.push(block);
                continue;
            }
            let next_line = buffer.full_line(block.end())?;
            let next_text = buffer.substr(next_line.begin()..next_line.end())?.to_string();
            let block_text = buffer.substr(block.begin()..block.end())?.to_string();
            edits.push((
                block.begin()..next_line.end(),
                format!("{}{}", next_text, block_text),
            ));
            new_regions.push(Region::new(
                block.begin() + next_text.len(),
                block.begin() + next_text.len() + block_text.len(),
            ));
        }

        if edits.is_empty() {
            return Ok(0);
        }

        edits.sort_by(|a, b| b.0.start.cmp(&a.0.start));
        self.apply_edits(buffer_id, edits)?;
        new_regions.sort_by_key(|r| (r.begin(), r.end()));
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

    pub fn close_view(&mut self, view_id: ViewId) -> Option<ViewId> {
        let buffer_id = self.views.remove(&view_id)?.buffer_id();
        self.buffers.remove(&buffer_id);

        let next_active = if self.active_view == Some(view_id) {
            let mut ids = self.view_ids();
            ids.sort();
            ids.first().copied()
        } else {
            self.active_view
        };
        self.active_view = next_active;
        next_active
    }

    fn selected_line_blocks(&self, buffer: &Buffer) -> Result<Vec<Region>, BufferError> {
        let selections = self.active_selection_regions().ok_or(BufferError::NoActiveView)?;
        let mut lines = Vec::new();

        for region in selections {
            if region.empty() {
                lines.push(buffer.full_line(region.caret().min(buffer.len()))?);
                continue;
            }

            let mut point = region.begin();
            while point < region.end() {
                let full_line = buffer.full_line(point)?;
                lines.push(full_line);
                if full_line.end() <= point || full_line.end() >= region.end() {
                    break;
                }
                point = full_line.end();
            }
        }

        lines.sort_by_key(|r| (r.begin(), r.end()));
        lines.dedup_by(|a, b| a.begin() == b.begin() && a.end() == b.end());

        let mut blocks: Vec<Region> = Vec::new();
        for line in lines {
            match blocks.last_mut() {
                Some(last) if last.end() == line.begin() => {
                    *last = Region::new(last.begin(), line.end());
                }
                _ => blocks.push(line),
            }
        }
        Ok(blocks)
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

    #[test]
    fn move_up_down_and_select_line_work() {
        let mut state = AppState::new("Needle");
        let (buffer_id, view_id) = state.create_empty_buffer_and_view();
        state.apply_edit(buffer_id, 0..0, "abc\ndef\nxyz").unwrap();
        state
            .view_mut(view_id)
            .unwrap()
            .set_single_selection(Region::new(5, 5));

        state.move_caret_up().unwrap();
        assert_eq!(state.active_selection_regions().unwrap(), vec![Region::new(1, 1)]);

        state.move_caret_down().unwrap();
        assert_eq!(state.active_selection_regions().unwrap(), vec![Region::new(5, 5)]);

        state.select_current_lines().unwrap();
        assert_eq!(state.active_selection_regions().unwrap(), vec![Region::new(4, 8)]);
    }

    #[test]
    fn duplicate_and_delete_to_line_edges_work() {
        let mut state = AppState::new("Needle");
        let (buffer_id, view_id) = state.create_empty_buffer_and_view();
        state.apply_edit(buffer_id, 0..0, "abc\ndef").unwrap();
        state
            .view_mut(view_id)
            .unwrap()
            .set_single_selection(Region::new(1, 1));

        state.duplicate_current_lines().unwrap();
        assert_eq!(state.buffer(buffer_id).unwrap().content(), "abc\nabc\ndef");

        state
            .view_mut(view_id)
            .unwrap()
            .set_single_selection(Region::new(5, 5));
        state.delete_to_beginning_of_line().unwrap();
        assert_eq!(state.buffer(buffer_id).unwrap().content(), "abc\nbc\ndef");

        state
            .view_mut(view_id)
            .unwrap()
            .set_single_selection(Region::new(4, 4));
        state.delete_to_end_of_line().unwrap();
        assert_eq!(state.buffer(buffer_id).unwrap().content(), "abc\n\ndef");
    }

    #[test]
    fn goto_line_and_split_selection_into_lines_work() {
        let mut state = AppState::new("Needle");
        let (buffer_id, view_id) = state.create_empty_buffer_and_view();
        state.apply_edit(buffer_id, 0..0, "abc\ndef\nxyz").unwrap();

        state.goto_line(3).unwrap();
        assert_eq!(state.active_selection_regions().unwrap(), vec![Region::new(8, 8)]);

        state
            .view_mut(view_id)
            .unwrap()
            .set_single_selection(Region::new(1, 9));
        state.split_selection_into_lines().unwrap();
        assert_eq!(
            state.active_selection_regions().unwrap(),
            vec![Region::new(0, 0), Region::new(4, 4), Region::new(8, 8)]
        );
    }

    #[test]
    fn move_selected_lines_up_and_down_work() {
        let mut state = AppState::new("Needle");
        let (buffer_id, view_id) = state.create_empty_buffer_and_view();
        state.apply_edit(buffer_id, 0..0, "a\nb\nc\n").unwrap();
        state
            .view_mut(view_id)
            .unwrap()
            .set_single_selection(Region::new(2, 2));

        state.move_selected_lines_down().unwrap();
        assert_eq!(state.buffer(buffer_id).unwrap().content(), "a\nc\nb\n");

        state.move_selected_lines_up().unwrap();
        assert_eq!(state.buffer(buffer_id).unwrap().content(), "a\nb\nc\n");
    }
}
