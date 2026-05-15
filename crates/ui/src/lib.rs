use std::{fs, path::PathBuf};

use anyhow::{anyhow, Result};
use eframe::{egui, App, Frame, NativeOptions};
use needle_core::{BufferError, NeedleApp, Region};
use serde_json::{Map, Value};
use tracing::warn;

pub fn run_native() -> Result<()> {
    let options = NativeOptions::default();
    eframe::run_native(
        "Project Needle",
        options,
        Box::new(|_cc| Ok(Box::new(NeedleEditorApp::default()))),
    )
    .map_err(|err| anyhow!("failed to start native eframe application: {err}"))?;
    Ok(())
}

pub struct NeedleEditorApp {
    core: NeedleApp,
    editor_text: String,
    last_synced_revision: u64,
    status_message: String,
}

impl Default for NeedleEditorApp {
    fn default() -> Self {
        let mut core = NeedleApp::builder().name("Project Needle").build();
        core.open_scratch_buffer();
        let mut app = Self {
            core,
            editor_text: String::new(),
            last_synced_revision: 0,
            status_message: "Ready".to_string(),
        };
        app.sync_from_core(true);
        app
    }
}

impl NeedleEditorApp {
    fn sync_from_core(&mut self, force: bool) {
        if let Some(buffer) = self.core.state().active_buffer() {
            if force || buffer.revision() != self.last_synced_revision {
                self.editor_text = buffer.content().to_string();
                self.last_synced_revision = buffer.revision();
            }
        }
    }

    fn apply_editor_text_to_core(&mut self) {
        let (buffer_id, current_len) = match self.core.state().active_buffer_id() {
            Some(buffer_id) => {
                let len = self
                    .core
                    .state()
                    .buffer(buffer_id)
                    .map(|buffer| buffer.len())
                    .unwrap_or(0);
                (buffer_id, len)
            }
            None => return,
        };

        match self
            .core
            .state_mut()
            .apply_edit(buffer_id, 0..current_len, &self.editor_text)
        {
            Ok(_) => {
                if let Some(buffer) = self.core.state().buffer(buffer_id) {
                    self.last_synced_revision = buffer.revision();
                }
            }
            Err(err) => self.set_status(format!("Edit failed: {err}")),
        }
    }

    fn set_status(&mut self, status: impl Into<String>) {
        self.status_message = status.into();
    }

    fn command(&mut self, name: &str) {
        match self.core.execute_command(name, self.core.default_invocation()) {
            Ok(output) => {
                if let Some(message) = output.message {
                    self.set_status(message);
                }
                self.sync_from_core(true);
            }
            Err(err) => self.set_status(format!("Command failed: {err}")),
        }
    }

    fn command_with_text(&mut self, name: &str, text: &str) {
        let mut json_args = Map::new();
        json_args.insert("text".to_string(), Value::String(text.to_string()));

        match self
            .core
            .execute_command(name, self.core.default_invocation().with_args(json_args))
        {
            Ok(output) => {
                if let Some(message) = output.message {
                    self.set_status(message);
                }
                self.sync_from_core(true);
            }
            Err(err) => self.set_status(format!("Command failed: {err}")),
        }
    }

    fn active_path(&self) -> Option<PathBuf> {
        self.core
            .state()
            .active_buffer()
            .and_then(|buffer| buffer.path().cloned())
    }

    fn active_title(&self) -> String {
        if let Some(path) = self.active_path() {
            path.display().to_string()
        } else {
            "Untitled".to_string()
        }
    }

    fn file_dirty(&self) -> bool {
        self.core
            .state()
            .active_buffer()
            .map(|buffer| buffer.is_dirty())
            .unwrap_or(false)
    }

    fn new_file(&mut self) {
        self.core.open_scratch_buffer();
        self.sync_from_core(true);
        self.set_status("Created new buffer");
    }

    fn open_file_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new().pick_file() else {
            return;
        };

        match fs::read_to_string(&path) {
            Ok(contents) => {
                let (buffer_id, _view_id) = self.core.state_mut().create_empty_buffer_and_view();
                if let Some(buffer) = self.core.state_mut().buffer_mut(buffer_id) {
                    if let Err(err) = buffer.replace(0..0, &contents) {
                        self.set_status(format!("Open failed: {err}"));
                        return;
                    }
                    buffer.set_path(path.clone());
                    buffer.set_dirty(false);
                }
                self.sync_from_core(true);
                self.set_status(format!("Opened {}", path.display()));
            }
            Err(err) => self.set_status(format!("Open failed: {err}")),
        }
    }

    fn save_active_file(&mut self) {
        if let Some(path) = self.active_path() {
            self.save_to_path(path);
        } else {
            self.save_active_file_as();
        }
    }

    fn save_active_file_as(&mut self) {
        let Some(path) = rfd::FileDialog::new().save_file() else {
            return;
        };
        self.save_to_path(path);
    }

    fn save_to_path(&mut self, path: PathBuf) {
        let contents = self
            .core
            .state()
            .active_buffer()
            .map(|buffer| buffer.content().to_string())
            .unwrap_or_default();

        match fs::write(&path, contents) {
            Ok(()) => {
                if let Some(buffer) = self.core.state_mut().active_buffer_mut() {
                    buffer.set_path(path.clone());
                    buffer.set_dirty(false);
                }
                self.set_status(format!("Saved {}", path.display()));
            }
            Err(err) => self.set_status(format!("Save failed: {err}")),
        }
    }

    fn update_selection_from_egui(&mut self, primary_char: usize, secondary_char: usize) {
        let primary = char_index_to_byte_index(&self.editor_text, primary_char);
        let secondary = char_index_to_byte_index(&self.editor_text, secondary_char);
        if let Err(err) = self
            .core
            .state_mut()
            .set_active_selections([Region::new(secondary, primary)])
        {
            warn!(?err, "failed to sync selection from egui");
        }
    }

    fn selection_status(&self) -> String {
        let Some(buffer) = self.core.state().active_buffer() else {
            return "Ln 1, Col 1".to_string();
        };
        let Some(view) = self.core.state().active_view() else {
            return "Ln 1, Col 1".to_string();
        };
        let caret = view
            .sel()
            .last()
            .map(|region| region.caret())
            .unwrap_or(0)
            .min(buffer.len());
        match buffer.row_col(caret) {
            Ok((row, col)) => format!("Ln {}, Col {}", row + 1, col + 1),
            Err(BufferError::NoActiveView) => "Ln 1, Col 1".to_string(),
            Err(err) => format!("Pos err: {err}"),
        }
    }
}

impl App for NeedleEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.sync_from_core(false);

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("New").clicked() {
                    self.new_file();
                }
                if ui.button("Open").clicked() {
                    self.open_file_dialog();
                }
                if ui.button("Save").clicked() {
                    self.save_active_file();
                }
                if ui.button("Save As").clicked() {
                    self.save_active_file_as();
                }
                ui.separator();
                if ui.button("Undo").clicked() {
                    self.command("undo");
                }
                if ui.button("Redo").clicked() {
                    self.command("redo");
                }
                if ui.button("Select All").clicked() {
                    self.command("select_all");
                }
                if ui.button("←").clicked() {
                    self.command("move_left");
                }
                if ui.button("→").clicked() {
                    self.command("move_right");
                }
                if ui.button("Home").clicked() {
                    self.command("move_to_beginning_of_line");
                }
                if ui.button("End").clicked() {
                    self.command("move_to_end_of_line");
                }
                if ui.button("+Line Below").clicked() {
                    self.command_with_text("insert_line_after", "");
                }
                if ui.button("+Line Above").clicked() {
                    self.command_with_text("insert_line_before", "");
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(self.active_title());
            ui.label(if self.file_dirty() { "Modified" } else { "Saved" });
            ui.add_space(4.0);

            let output = egui::TextEdit::multiline(&mut self.editor_text)
                .desired_rows(32)
                .desired_width(f32::INFINITY)
                .font(egui::TextStyle::Monospace)
                .code_editor()
                .show(ui);

            if let Some(cursor_range) = &output.cursor_range {
                self.update_selection_from_egui(cursor_range.primary.index, cursor_range.secondary.index);
            }

            if output.response.changed() {
                self.apply_editor_text_to_core();
            }
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
                ui.separator();
                ui.label(self.selection_status());
                ui.separator();
                ui.label(format!(
                    "{} chars",
                    self.core
                        .state()
                        .active_buffer()
                        .map(|buffer| buffer.len())
                        .unwrap_or(0)
                ));
            });
        });
    }
}

fn char_index_to_byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}
