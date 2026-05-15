use serde_json::Value;

use crate::{
    AppState, CommandBus, CommandError, CommandHandler, CommandInvocation, CommandOutput,
    CommandSpec, CommandTarget,
};

#[derive(Default)]
pub struct NeedleAppBuilder {
    name: String,
}

impl NeedleAppBuilder {
    pub fn new() -> Self {
        Self {
            name: "Project Needle".to_string(),
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn build(self) -> NeedleApp {
        NeedleApp::new(self.name)
    }
}

pub struct NeedleApp {
    name: String,
    state: AppState,
    commands: CommandBus,
}

impl NeedleApp {
    pub fn builder() -> NeedleAppBuilder {
        NeedleAppBuilder::new()
    }

    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let mut app = Self {
            name: name.clone(),
            state: AppState::new(name),
            commands: CommandBus::new(),
        };
        app.register_builtin_commands();
        app
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut AppState {
        &mut self.state
    }

    pub fn command_bus(&self) -> &CommandBus {
        &self.commands
    }

    pub fn open_scratch_buffer(&mut self) {
        self.state.create_empty_buffer_and_view();
    }

    pub fn execute_command(
        &mut self,
        name: &str,
        invocation: CommandInvocation,
    ) -> Result<CommandOutput, CommandError> {
        let handler = self
            .commands
            .handler(name)
            .ok_or_else(|| CommandError::NotFound(name.to_string()))?;

        handler(&mut self.state, invocation)
    }

    fn register_builtin_commands(&mut self) {
        self.register_builtin(
            CommandSpec::new("new_file", "Create a new empty buffer and view"),
            std::sync::Arc::new(|state, _invocation| {
                let (_buffer_id, view_id) = state.create_empty_buffer_and_view();
                Ok(CommandOutput::ok(format!("created view {}", view_id.0)))
            }),
        );

        self.register_builtin(
            CommandSpec::new("insert_text", "Insert text into the active selection(s)"),
            std::sync::Arc::new(|state, invocation| {
                let text = invocation
                    .args
                    .get("text")
                    .and_then(Value::as_str)
                    .ok_or_else(|| CommandError::MissingArgument("text".into()))?;

                let edited = state
                    .replace_active_selections(text)
                    .map_err(map_buffer_error)?;

                Ok(CommandOutput::ok(format!(
                    "updated {} selection(s) with {} bytes",
                    edited,
                    text.len()
                )))
            }),
        );

        self.register_builtin(
            CommandSpec::new("delete_backward", "Delete backwards from the active selection(s)"),
            std::sync::Arc::new(|state, _invocation| {
                let edited = state
                    .delete_backward_from_active_selections()
                    .map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!("deleted across {} selection(s)", edited)))
            }),
        );

        self.register_builtin(
            CommandSpec::new("delete_forward", "Delete forwards from the active selection(s)"),
            std::sync::Arc::new(|state, _invocation| {
                let edited = state
                    .delete_forward_from_active_selections()
                    .map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!("forward-deleted across {} selection(s)", edited)))
            }),
        );

        self.register_builtin(
            CommandSpec::new("move_left", "Move the active caret(s) left"),
            std::sync::Arc::new(|state, _invocation| {
                let moved = state.move_caret_left().map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!("moved {} selection(s) left", moved)))
            }),
        );

        self.register_builtin(
            CommandSpec::new("move_right", "Move the active caret(s) right"),
            std::sync::Arc::new(|state, _invocation| {
                let moved = state.move_caret_right().map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!("moved {} selection(s) right", moved)))
            }),
        );

        self.register_builtin(
            CommandSpec::new("move_to_beginning_of_line", "Move caret(s) to beginning of line"),
            std::sync::Arc::new(|state, _invocation| {
                let moved = state
                    .move_to_beginning_of_line()
                    .map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!(
                    "moved {} selection(s) to line start",
                    moved
                )))
            }),
        );

        self.register_builtin(
            CommandSpec::new("move_to_end_of_line", "Move caret(s) to end of line"),
            std::sync::Arc::new(|state, _invocation| {
                let moved = state.move_to_end_of_line().map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!(
                    "moved {} selection(s) to line end",
                    moved
                )))
            }),
        );

        self.register_builtin(
            CommandSpec::new("move_up", "Move the active caret(s) up"),
            std::sync::Arc::new(|state, _invocation| {
                let moved = state.move_caret_up().map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!("moved {} selection(s) up", moved)))
            }),
        );

        self.register_builtin(
            CommandSpec::new("move_down", "Move the active caret(s) down"),
            std::sync::Arc::new(|state, _invocation| {
                let moved = state.move_caret_down().map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!("moved {} selection(s) down", moved)))
            }),
        );

        self.register_builtin(
            CommandSpec::new("select_line", "Select the current line(s)"),
            std::sync::Arc::new(|state, _invocation| {
                let selected = state.select_current_lines().map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!("selected {} line(s)", selected)))
            }),
        );

        self.register_builtin(
            CommandSpec::new("duplicate_line", "Duplicate the current line(s)"),
            std::sync::Arc::new(|state, _invocation| {
                let duplicated = state.duplicate_current_lines().map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!(
                    "duplicated {} line selection(s)",
                    duplicated
                )))
            }),
        );

        self.register_builtin(
            CommandSpec::new("delete_to_beginning_of_line", "Delete to the beginning of line"),
            std::sync::Arc::new(|state, _invocation| {
                let deleted = state.delete_to_beginning_of_line().map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!(
                    "deleted to line start for {} selection(s)",
                    deleted
                )))
            }),
        );

        self.register_builtin(
            CommandSpec::new("delete_to_end_of_line", "Delete to the end of line"),
            std::sync::Arc::new(|state, _invocation| {
                let deleted = state.delete_to_end_of_line().map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!(
                    "deleted to line end for {} selection(s)",
                    deleted
                )))
            }),
        );

        self.register_builtin(
            CommandSpec::new("insert_line_after", "Insert a new line after the current line(s)"),
            std::sync::Arc::new(|state, invocation| {
                let text = invocation
                    .args
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let inserted = state.insert_line_after(text).map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!(
                    "inserted line after for {} selection(s)",
                    inserted
                )))
            }),
        );

        self.register_builtin(
            CommandSpec::new("insert_line_before", "Insert a new line before the current line(s)"),
            std::sync::Arc::new(|state, invocation| {
                let text = invocation
                    .args
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let inserted = state.insert_line_before(text).map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!(
                    "inserted line before for {} selection(s)",
                    inserted
                )))
            }),
        );

        self.register_builtin(
            CommandSpec::new("move_lines_up", "Move selected line(s) up"),
            std::sync::Arc::new(|state, _invocation| {
                let moved = state.move_selected_lines_up().map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!("moved {} line block(s) up", moved)))
            }),
        );

        self.register_builtin(
            CommandSpec::new("move_lines_down", "Move selected line(s) down"),
            std::sync::Arc::new(|state, _invocation| {
                let moved = state.move_selected_lines_down().map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!("moved {} line block(s) down", moved)))
            }),
        );

        self.register_builtin(
            CommandSpec::new("split_selection_into_lines", "Split current selection into line carets"),
            std::sync::Arc::new(|state, _invocation| {
                let split = state.split_selection_into_lines().map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!("split into {} caret(s)", split)))
            }),
        );

        self.register_builtin(
            CommandSpec::new("goto_line", "Move cursor to a 1-based line number"),
            std::sync::Arc::new(|state, invocation| {
                let line = invocation
                    .args
                    .get("line")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| CommandError::MissingArgument("line".into()))? as usize;
                let moved = state.goto_line(line).map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!("moved {} caret(s) to line {line}", moved)))
            }),
        );

        self.register_builtin(
            CommandSpec::new("select_all", "Select the full active buffer"),
            std::sync::Arc::new(|state, _invocation| {
                let region = state.select_all().map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!(
                    "selected {} bytes",
                    region.end() - region.begin()
                )))
            }),
        );

        self.register_builtin(
            CommandSpec::new("undo", "Undo last buffer edit"),
            std::sync::Arc::new(|state, _invocation| match state.undo() {
                Ok(Some(transaction)) => Ok(CommandOutput::ok(format!(
                    "undid {} edit(s)",
                    transaction.len()
                ))),
                Ok(None) => Ok(CommandOutput::ok("nothing to undo")),
                Err(err) => Err(map_buffer_error(err)),
            }),
        );

        self.register_builtin(
            CommandSpec::new("redo", "Redo last undone buffer edit"),
            std::sync::Arc::new(|state, _invocation| match state.redo() {
                Ok(Some(transaction)) => Ok(CommandOutput::ok(format!(
                    "redid {} edit(s)",
                    transaction.len()
                ))),
                Ok(None) => Ok(CommandOutput::ok("nothing to redo")),
                Err(err) => Err(map_buffer_error(err)),
            }),
        );

        self.register_builtin(
            CommandSpec::new("buffer_info", "Show active buffer summary"),
            std::sync::Arc::new(|state, _invocation| {
                let buffer = state.active_buffer().ok_or(CommandError::NoActiveView)?;
                let view = state.active_view().ok_or(CommandError::NoActiveView)?;
                let caret = view.sel().last().map(|region| region.caret()).unwrap_or(0);
                let (row, col) = buffer.row_col(caret).map_err(map_buffer_error)?;
                Ok(CommandOutput::ok(format!(
                    "len={}, lines={}, selections={}, caret=({}, {})",
                    buffer.len(),
                    buffer.line_count(),
                    view.selection_count(),
                    row,
                    col
                )))
            }),
        );
    }

    fn register_builtin(&mut self, spec: CommandSpec, handler: CommandHandler) {
        self.commands.register(spec, handler);
    }

    pub fn default_invocation(&self) -> CommandInvocation {
        match self.state.active_view_id() {
            Some(view_id) => CommandInvocation::new(CommandTarget::View(view_id)),
            None => CommandInvocation::new(CommandTarget::Application),
        }
    }
}

fn map_buffer_error(error: crate::BufferError) -> CommandError {
    match error {
        crate::BufferError::NoActiveView => CommandError::NoActiveView,
        other => CommandError::Buffer(other.to_string()),
    }
}
