use std::{ops::Range, path::PathBuf};

use thiserror::Error;

use crate::{BufferId, Region};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BufferError {
    #[error("no active view")]
    NoActiveView,
    #[error("buffer is read-only")]
    ReadOnly,
    #[error("range {start}..{end} is out of bounds for buffer length {len}")]
    InvalidRange { start: usize, end: usize, len: usize },
    #[error("row {row} is out of bounds")]
    InvalidRow { row: usize },
}

#[derive(Debug, Clone)]
pub struct Buffer {
    id: BufferId,
    path: Option<PathBuf>,
    content: String,
    revision: u64,
    dirty: bool,
    read_only: bool,
}

impl Buffer {
    pub fn new_empty() -> Self {
        Self {
            id: BufferId::next(),
            path: None,
            content: String::new(),
            revision: 0,
            dirty: false,
            read_only: false,
        }
    }

    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            content: text.into(),
            ..Self::new_empty()
        }
    }

    pub fn id(&self) -> BufferId {
        self.id
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub fn set_path(&mut self, path: impl Into<PathBuf>) {
        self.path = Some(path.into());
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn set_dirty(&mut self, dirty: bool) {
        self.dirty = dirty;
    }

    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    pub fn set_read_only(&mut self, read_only: bool) {
        self.read_only = read_only;
    }

    pub fn len(&self) -> usize {
        self.content.len()
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn line_count(&self) -> usize {
        if self.content.is_empty() {
            1
        } else {
            self.content.bytes().filter(|byte| *byte == b'\n').count() + 1
        }
    }

    pub fn insert(&mut self, point: usize, text: &str) -> Result<(), BufferError> {
        self.replace(point..point, text)
    }

    pub fn delete(&mut self, range: Range<usize>) -> Result<(), BufferError> {
        self.replace(range, "")
    }

    pub fn replace(&mut self, range: Range<usize>, text: &str) -> Result<(), BufferError> {
        if self.read_only {
            return Err(BufferError::ReadOnly);
        }

        self.validate_range(&range)?;
        self.content.replace_range(range, text);
        self.revision += 1;
        self.dirty = true;
        Ok(())
    }

    pub fn substr(&self, range: Range<usize>) -> Result<&str, BufferError> {
        self.validate_range(&range)?;
        Ok(&self.content[range])
    }

    pub fn row_col(&self, point: usize) -> Result<(usize, usize), BufferError> {
        self.validate_point(point)?;

        let mut row = 0;
        let mut line_start = 0;
        for (idx, ch) in self.content.char_indices() {
            if idx >= point {
                break;
            }
            if ch == '\n' {
                row += 1;
                line_start = idx + ch.len_utf8();
            }
        }

        Ok((row, point - line_start))
    }

    pub fn text_point(&self, target_row: usize, col: usize) -> Result<usize, BufferError> {
        let line_start = self.line_start(target_row)?;
        let line_region = self.line(line_start)?;
        Ok((line_start + col).min(line_region.end()))
    }

    pub fn line(&self, point: usize) -> Result<Region, BufferError> {
        self.validate_point(point)?;
        let start = self.line_start_for_point(point)?;
        let end = self.line_end_for_point(point, false)?;
        Ok(Region::new(start, end))
    }

    pub fn full_line(&self, point: usize) -> Result<Region, BufferError> {
        self.validate_point(point)?;
        let start = self.line_start_for_point(point)?;
        let end = self.line_end_for_point(point, true)?;
        Ok(Region::new(start, end))
    }

    pub fn line_text(&self, point: usize) -> Result<&str, BufferError> {
        let region = self.line(point)?;
        self.substr(region.begin()..region.end())
    }

    pub fn line_start(&self, target_row: usize) -> Result<usize, BufferError> {
        if target_row == 0 {
            return Ok(0);
        }

        let mut row = 0;
        for (idx, ch) in self.content.char_indices() {
            if ch == '\n' {
                row += 1;
                if row == target_row {
                    return Ok(idx + ch.len_utf8());
                }
            }
        }

        Err(BufferError::InvalidRow { row: target_row })
    }

    pub fn prev_char_start(&self, point: usize) -> Result<Option<usize>, BufferError> {
        self.validate_point(point)?;
        if point == 0 {
            return Ok(None);
        }
        Ok(self.content[..point].char_indices().last().map(|(idx, _)| idx))
    }

    pub fn next_char_end(&self, point: usize) -> Result<Option<usize>, BufferError> {
        self.validate_point(point)?;
        if point == self.content.len() {
            return Ok(None);
        }
        Ok(self
            .content[point..]
            .chars()
            .next()
            .map(|ch| point + ch.len_utf8()))
    }

    fn line_start_for_point(&self, point: usize) -> Result<usize, BufferError> {
        self.validate_point(point)?;
        Ok(self.content[..point].rfind('\n').map(|idx| idx + 1).unwrap_or(0))
    }

    fn line_end_for_point(&self, point: usize, include_newline: bool) -> Result<usize, BufferError> {
        self.validate_point(point)?;
        let suffix = &self.content[point..];
        match suffix.find('\n') {
            Some(rel_idx) if include_newline => Ok(point + rel_idx + 1),
            Some(rel_idx) => Ok(point + rel_idx),
            None => Ok(self.content.len()),
        }
    }

    fn validate_point(&self, point: usize) -> Result<(), BufferError> {
        self.validate_range(&(point..point))
    }

    fn validate_range(&self, range: &Range<usize>) -> Result<(), BufferError> {
        if range.start > range.end || range.end > self.content.len() {
            return Err(BufferError::InvalidRange {
                start: range.start,
                end: range.end,
                len: self.content.len(),
            });
        }

        if !self.content.is_char_boundary(range.start) || !self.content.is_char_boundary(range.end) {
            return Err(BufferError::InvalidRange {
                start: range.start,
                end: range.end,
                len: self.content.len(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Buffer;
    use crate::Region;

    #[test]
    fn buffer_replace_updates_revision() {
        let mut buffer = Buffer::from_text("hello world");
        buffer.replace(6..11, "needle").unwrap();

        assert_eq!(buffer.content(), "hello needle");
        assert_eq!(buffer.revision(), 1);
        assert!(buffer.is_dirty());
    }

    #[test]
    fn buffer_row_col_and_text_point_roundtrip() {
        let buffer = Buffer::from_text("abc\ndef\nxyz");
        let point = buffer.text_point(1, 2).unwrap();
        assert_eq!(point, 6);
        assert_eq!(buffer.row_col(point).unwrap(), (1, 2));
    }

    #[test]
    fn buffer_line_ranges_work() {
        let buffer = Buffer::from_text("abc\ndef\n");
        assert_eq!(buffer.line(5).unwrap(), Region::new(4, 7));
        assert_eq!(buffer.full_line(5).unwrap(), Region::new(4, 8));
    }

    #[test]
    fn buffer_prev_and_next_char_helpers_work() {
        let buffer = Buffer::from_text("a好b");
        assert_eq!(buffer.prev_char_start(4).unwrap(), Some(1));
        assert_eq!(buffer.next_char_end(1).unwrap(), Some(4));
    }
}
