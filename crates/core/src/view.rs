use crate::{BufferId, Region, SelectionSet, Settings, ViewId};

#[derive(Debug, Clone)]
pub struct View {
    id: ViewId,
    buffer_id: BufferId,
    name: Option<String>,
    scratch: bool,
    local_settings: Settings,
    selections: SelectionSet,
    viewport: (f32, f32),
    syntax: Option<String>,
}

impl View {
    pub fn new(buffer_id: BufferId) -> Self {
        Self {
            id: ViewId::next(),
            buffer_id,
            name: None,
            scratch: false,
            local_settings: Settings::new(),
            selections: SelectionSet::new(),
            viewport: (0.0, 0.0),
            syntax: None,
        }
    }

    pub fn id(&self) -> ViewId {
        self.id
    }

    pub fn buffer_id(&self) -> BufferId {
        self.buffer_id
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = Some(name.into());
    }

    pub fn is_scratch(&self) -> bool {
        self.scratch
    }

    pub fn set_scratch(&mut self, scratch: bool) {
        self.scratch = scratch;
    }

    pub fn settings(&self) -> &Settings {
        &self.local_settings
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.local_settings
    }

    pub fn sel(&self) -> &SelectionSet {
        &self.selections
    }

    pub fn sel_mut(&mut self) -> &mut SelectionSet {
        &mut self.selections
    }

    pub fn set_single_selection(&mut self, region: Region) {
        self.selections.clear();
        self.selections.add(region);
    }

    pub fn set_selections<I>(&mut self, regions: I)
    where
        I: IntoIterator<Item = Region>,
    {
        self.selections.clear();
        self.selections.add_all(regions);
    }

    pub fn selection_count(&self) -> usize {
        self.selections.len()
    }

    pub fn viewport(&self) -> (f32, f32) {
        self.viewport
    }

    pub fn set_viewport(&mut self, x: f32, y: f32) {
        self.viewport = (x, y);
    }

    pub fn syntax(&self) -> Option<&str> {
        self.syntax.as_deref()
    }

    pub fn set_syntax(&mut self, syntax: impl Into<String>) {
        self.syntax = Some(syntax.into());
    }
}
