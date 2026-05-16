use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant, SystemTime},
};

use anyhow::{anyhow, Result};
use arboard::Clipboard;
use chardetng::EncodingDetector;
use eframe::{egui, App, Frame, NativeOptions};
use egui::text::{CCursor, CCursorRange};
use encoding_rs::{Encoding, UTF_16BE, UTF_16LE};
use needle_core::{BufferError, CommandSpec, NeedleApp, Region, ViewId};
use needle_search::{
    ProjectFileEntry, ProjectIndex, ProjectSearchMatch, ProjectTextSearchEvent,
    ProjectTextSearchHandle, ProjectTextSearchQuery,
};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;
use serde_json::{Map, Value};
use tracing::warn;

#[derive(Debug, Clone, Deserialize, Default)]
struct KeyBindingConfig {
    command: String,
    key: String,
    #[serde(default)]
    modifiers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ProjectSettingsFile {
    show_hidden_files: Option<bool>,
    sidebar_width: Option<f32>,
    #[serde(default)]
    keybindings: Vec<KeyBindingConfig>,
}

#[derive(Debug, Clone)]
struct ResolvedSettings {
    show_hidden_files: bool,
    sidebar_width: f32,
    keybindings: Vec<KeyBindingConfig>,
}

impl Default for ResolvedSettings {
    fn default() -> Self {
        Self {
            show_hidden_files: false,
            sidebar_width: 220.0,
            keybindings: Vec::new(),
        }
    }
}

struct ProjectSearchTask {
    handle: ProjectTextSearchHandle,
    scanned_files: usize,
    total_matches: usize,
}

#[derive(Debug, Clone, Copy)]
enum TextFileEncoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    Legacy(&'static Encoding),
}

impl TextFileEncoding {
    fn label(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf8Bom => "UTF-8 with BOM",
            Self::Utf16Le => "UTF-16 LE",
            Self::Utf16Be => "UTF-16 BE",
            Self::Legacy(encoding) => encoding.name(),
        }
    }

    fn encode(self, text: &str) -> Vec<u8> {
        match self {
            Self::Utf8 => text.as_bytes().to_vec(),
            Self::Utf8Bom => {
                let mut bytes = vec![0xEF, 0xBB, 0xBF];
                bytes.extend_from_slice(text.as_bytes());
                bytes
            }
            Self::Utf16Le => {
                let mut bytes = vec![0xFF, 0xFE];
                for unit in text.encode_utf16() {
                    bytes.extend_from_slice(&unit.to_le_bytes());
                }
                bytes
            }
            Self::Utf16Be => {
                let mut bytes = vec![0xFE, 0xFF];
                for unit in text.encode_utf16() {
                    bytes.extend_from_slice(&unit.to_be_bytes());
                }
                bytes
            }
            Self::Legacy(encoding) => {
                let (encoded, _, _) = encoding.encode(text);
                encoded.into_owned()
            }
        }
    }
}

struct DecodedTextFile {
    text: String,
    encoding: TextFileEncoding,
}

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
    last_synced_view: Option<ViewId>,
    status_message: String,
    show_command_palette: bool,
    command_palette_query: String,
    show_quick_open_panel: bool,
    quick_open_query: String,
    show_recent_projects_panel: bool,
    recent_projects: Vec<PathBuf>,
    show_project_search_panel: bool,
    project_search_query: String,
    project_search_case_sensitive: bool,
    cached_project_search_query: String,
    cached_project_search_case_sensitive: bool,
    cached_project_search_generation: u64,
    cached_project_search_results: Vec<ProjectSearchMatch>,
    show_find_panel: bool,
    find_query: String,
    replace_query: String,
    show_goto_line_panel: bool,
    goto_line_query: String,
    pending_close_view: Option<ViewId>,
    clipboard: Option<Clipboard>,
    project_root: Option<PathBuf>,
    user_settings_path: Option<PathBuf>,
    user_settings_mtime: Option<SystemTime>,
    user_settings: ProjectSettingsFile,
    project_settings_path: Option<PathBuf>,
    project_settings_mtime: Option<SystemTime>,
    project_settings: ProjectSettingsFile,
    resolved_settings: ResolvedSettings,
    sidebar_width: f32,
    expanded_dirs: BTreeMap<PathBuf, bool>,
    project_index: Option<ProjectIndex>,
    project_search_task: Option<ProjectSearchTask>,
    project_search_status: String,
    last_project_scan: Option<Instant>,
    project_watcher: Option<RecommendedWatcher>,
    project_watcher_rx: Option<Receiver<notify::Result<Event>>>,
    file_encodings: BTreeMap<PathBuf, TextFileEncoding>,
    desired_editor_selection: Option<(usize, usize)>,
    suppress_selection_sync_once: bool,
}

impl Default for NeedleEditorApp {
    fn default() -> Self {
        let mut core = NeedleApp::builder().name("Project Needle").build();
        core.open_scratch_buffer();
        let mut app = Self {
            core,
            editor_text: String::new(),
            last_synced_revision: 0,
            last_synced_view: None,
            status_message: "Ready".to_string(),
            show_command_palette: false,
            command_palette_query: String::new(),
            show_quick_open_panel: false,
            quick_open_query: String::new(),
            show_recent_projects_panel: false,
            recent_projects: load_recent_projects(),
            show_project_search_panel: false,
            project_search_query: String::new(),
            project_search_case_sensitive: false,
            cached_project_search_query: String::new(),
            cached_project_search_case_sensitive: false,
            cached_project_search_generation: 0,
            cached_project_search_results: Vec::new(),
            show_find_panel: false,
            find_query: String::new(),
            replace_query: String::new(),
            show_goto_line_panel: false,
            goto_line_query: String::new(),
            pending_close_view: None,
            clipboard: Clipboard::new().ok(),
            project_root: None,
            user_settings_path: default_user_settings_path(),
            user_settings_mtime: None,
            user_settings: ProjectSettingsFile::default(),
            project_settings_path: None,
            project_settings_mtime: None,
            project_settings: ProjectSettingsFile::default(),
            resolved_settings: ResolvedSettings::default(),
            sidebar_width: 220.0,
            expanded_dirs: BTreeMap::new(),
            project_index: None,
            project_search_task: None,
            project_search_status: "Idle".to_string(),
            last_project_scan: None,
            project_watcher: None,
            project_watcher_rx: None,
            file_encodings: BTreeMap::new(),
            desired_editor_selection: None,
            suppress_selection_sync_once: false,
        };
        app.reload_user_settings_if_needed(true);
        app.sync_from_core(true);
        app
    }
}

impl NeedleEditorApp {
    fn sync_from_core(&mut self, force: bool) {
        let active_view = self.core.state().active_view_id();
        if let Some(buffer) = self.core.state().active_buffer() {
            if force
                || active_view != self.last_synced_view
                || buffer.revision() != self.last_synced_revision
            {
                self.editor_text = buffer.content().to_string();
                self.last_synced_revision = buffer.revision();
                self.last_synced_view = active_view;
                self.desired_editor_selection = self
                    .core
                    .state()
                    .active_selection_regions()
                    .and_then(|regions| {
                        regions.last().map(|region| (region.begin(), region.end()))
                    });
            }
        }
    }

    fn apply_editor_text_to_core(&mut self) {
        let (buffer_id, previous_text) = match self.core.state().active_buffer_id() {
            Some(buffer_id) => {
                let text = self
                    .core
                    .state()
                    .buffer(buffer_id)
                    .map(|buffer| buffer.content().to_string())
                    .unwrap_or_default();
                (buffer_id, text)
            }
            None => return,
        };

        let Some((range, replacement)) =
            compute_minimal_text_edit(&previous_text, &self.editor_text)
        else {
            return;
        };

        match self
            .core
            .state_mut()
            .apply_edit(buffer_id, range, replacement)
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

    fn active_keybindings(&self) -> Vec<KeyBindingConfig> {
        let mut merged: Vec<KeyBindingConfig> = Vec::new();
        let mut by_shortcut: HashMap<String, usize> = HashMap::new();

        for binding in self.resolved_settings.keybindings.iter().cloned() {
            let signature = keybinding_signature(&binding);
            if let Some(existing) = by_shortcut.get(&signature).copied() {
                merged[existing] = binding;
            } else {
                by_shortcut.insert(signature, merged.len());
                merged.push(binding);
            }
        }

        merged
    }

    fn apply_resolved_settings(&mut self) {
        self.resolved_settings = resolve_settings(&self.user_settings, &self.project_settings);
        self.sidebar_width = self.resolved_settings.sidebar_width.clamp(140.0, 480.0);
    }

    fn command(&mut self, name: &str) {
        match self
            .core
            .execute_command(name, self.core.default_invocation())
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

    fn command_with_args(&mut self, name: &str, json_args: Map<String, Value>) {
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

    fn command_with_text(&mut self, name: &str, text: &str) {
        let mut json_args = Map::new();
        json_args.insert("text".to_string(), Value::String(text.to_string()));
        self.command_with_args(name, json_args);
    }

    fn execute_palette_command(&mut self, name: &str) {
        self.command(name);
        self.show_command_palette = false;
    }

    fn render_top_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New").clicked() {
                    self.new_file();
                    ui.close();
                }
                if ui.button("Open").clicked() {
                    self.open_file_dialog();
                    ui.close();
                }
                if ui.button("Open Folder").clicked() {
                    self.open_folder_dialog();
                    ui.close();
                }
                if ui.button("Recent Projects").clicked() {
                    self.show_recent_projects_panel = true;
                    ui.close();
                }
                ui.separator();
                if ui.button("Save").clicked() {
                    self.save_active_file();
                    ui.close();
                }
                if ui.button("Save As").clicked() {
                    self.save_active_file_as();
                    ui.close();
                }
                if ui.button("Close").clicked() {
                    if let Some(view_id) = self.core.state().active_view_id() {
                        self.request_close_view(view_id);
                    }
                    ui.close();
                }
            });

            ui.menu_button("Edit", |ui| {
                if ui.button("Undo").clicked() {
                    self.command("undo");
                    ui.close();
                }
                if ui.button("Redo").clicked() {
                    self.command("redo");
                    ui.close();
                }
                ui.separator();
                if ui.button("Copy").clicked() {
                    self.copy_selection_to_clipboard();
                    ui.close();
                }
                if ui.button("Cut").clicked() {
                    self.cut_selection_to_clipboard();
                    ui.close();
                }
                if ui.button("Paste").clicked() {
                    self.paste_from_clipboard();
                    ui.close();
                }
                ui.separator();
                if ui.button("Select All").clicked() {
                    self.command("select_all");
                    ui.close();
                }
            });

            ui.menu_button("Search", |ui| {
                if ui.button("Command Palette").clicked() {
                    self.show_command_palette = true;
                    ui.close();
                }
                if ui.button("Quick Open").clicked() {
                    self.show_quick_open_panel = true;
                    ui.close();
                }
                if ui.button("Find").clicked() {
                    self.show_find_panel = true;
                    ui.close();
                }
                if ui.button("Find in Project").clicked() {
                    self.show_project_search_panel = true;
                    ui.close();
                }
                if ui.button("Goto Line").clicked() {
                    self.show_goto_line_panel = true;
                    ui.close();
                }
            });

            ui.menu_button("Navigate", |ui| {
                if ui.button("Left").clicked() {
                    self.command("move_left");
                    ui.close();
                }
                if ui.button("Right").clicked() {
                    self.command("move_right");
                    ui.close();
                }
                if ui.button("Up").clicked() {
                    self.command("move_up");
                    ui.close();
                }
                if ui.button("Down").clicked() {
                    self.command("move_down");
                    ui.close();
                }
                ui.separator();
                if ui.button("Home").clicked() {
                    self.command("move_to_beginning_of_line");
                    ui.close();
                }
                if ui.button("End").clicked() {
                    self.command("move_to_end_of_line");
                    ui.close();
                }
                if ui.button("Select Line").clicked() {
                    self.command("select_line");
                    ui.close();
                }
            });

            ui.menu_button("Lines", |ui| {
                if ui.button("Split Selection Into Lines").clicked() {
                    self.command("split_selection_into_lines");
                    ui.close();
                }
                if ui.button("Duplicate Line").clicked() {
                    self.command("duplicate_line");
                    ui.close();
                }
                if ui.button("Move Lines Up").clicked() {
                    self.command("move_lines_up");
                    ui.close();
                }
                if ui.button("Move Lines Down").clicked() {
                    self.command("move_lines_down");
                    ui.close();
                }
                ui.separator();
                if ui.button("Delete To Beginning Of Line").clicked() {
                    self.command("delete_to_beginning_of_line");
                    ui.close();
                }
                if ui.button("Delete To End Of Line").clicked() {
                    self.command("delete_to_end_of_line");
                    ui.close();
                }
                ui.separator();
                if ui.button("Insert Line Below").clicked() {
                    self.command_with_text("insert_line_after", "");
                    ui.close();
                }
                if ui.button("Insert Line Above").clicked() {
                    self.command_with_text("insert_line_before", "");
                    ui.close();
                }
            });
        });
    }

    fn palette_matches(&self) -> Vec<CommandSpec> {
        let specs = self.core.command_bus().specs();
        let query = self.command_palette_query.trim().to_lowercase();
        if query.is_empty() {
            return specs;
        }
        specs
            .into_iter()
            .filter(|spec| {
                spec.name.to_lowercase().contains(&query)
                    || spec.description.to_lowercase().contains(&query)
            })
            .collect()
    }

    fn quick_open_matches(&self) -> Vec<ProjectFileEntry> {
        self.project_index
            .as_ref()
            .map(|index| index.quick_open_matches(&self.quick_open_query, 200))
            .unwrap_or_default()
    }

    fn project_search_matches(&self) -> Vec<ProjectSearchMatch> {
        self.cached_project_search_results.clone()
    }

    fn project_index_generation(&self) -> u64 {
        self.project_index
            .as_ref()
            .map(|index| index.generation())
            .unwrap_or(0)
    }

    fn indexed_project_file_count(&self) -> usize {
        self.project_index
            .as_ref()
            .map(|index| index.file_count())
            .unwrap_or(0)
    }

    fn cancel_current_project_search(&mut self, status: impl Into<String>) {
        if let Some(task) = self.project_search_task.take() {
            task.handle.cancel();
        }
        self.project_search_status = status.into();
    }

    fn poll_project_search_task(&mut self) {
        let mut next_status = None;
        let mut clear_task = false;

        if let Some(task) = self.project_search_task.as_mut() {
            loop {
                match task.handle.receiver.try_recv() {
                    Ok(ProjectTextSearchEvent::Batch(batch)) => {
                        task.scanned_files = batch.scanned_files;
                        task.total_matches = batch.total_matches;
                        self.cached_project_search_results.extend(batch.matches);
                        next_status = Some(format!(
                            "Searching… {} match(es) in {} file(s)",
                            task.total_matches, task.scanned_files
                        ));
                    }
                    Ok(ProjectTextSearchEvent::Complete(summary)) => {
                        task.scanned_files = summary.scanned_files;
                        task.total_matches = summary.total_matches;
                        next_status = Some(if summary.total_matches == 0 {
                            format!("No matches in {} file(s)", summary.scanned_files)
                        } else {
                            format!(
                                "Found {} match(es) in {} file(s)",
                                summary.total_matches, summary.scanned_files
                            )
                        });
                        clear_task = true;
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        next_status = Some(format!(
                            "Search stopped at {} match(es) in {} file(s)",
                            task.total_matches, task.scanned_files
                        ));
                        clear_task = true;
                        break;
                    }
                }
            }
        }

        if let Some(status) = next_status {
            self.project_search_status = status;
        }
        if clear_task {
            self.project_search_task = None;
        }
    }

    fn refresh_project_index_if_needed(&mut self, force: bool, announce: bool) {
        let Some(root) = self.project_root.as_deref() else {
            self.project_index = None;
            self.cancel_current_project_search("Idle");
            self.cached_project_search_results.clear();
            self.last_project_scan = None;
            self.project_watcher = None;
            self.project_watcher_rx = None;
            return;
        };

        let should_scan = force
            || self
                .last_project_scan
                .map(|last| last.elapsed() >= Duration::from_secs(10))
                .unwrap_or(true);
        if !should_scan {
            return;
        }

        self.last_project_scan = Some(Instant::now());
        let new_index = ProjectIndex::build(root, self.resolved_settings.show_hidden_files);
        let file_count = new_index.file_count();
        let changed = self
            .project_index
            .as_ref()
            .map(|current| {
                current.root() != new_index.root()
                    || current.show_hidden() != new_index.show_hidden()
                    || current.generation() != new_index.generation()
            })
            .unwrap_or(true);

        self.project_index = Some(new_index);

        if force || changed {
            self.cancel_current_project_search(if self.project_search_query.trim().is_empty() {
                "Idle"
            } else {
                "Search cancelled: index changed"
            });
            self.cached_project_search_generation = 0;
            if !self.project_search_query.trim().is_empty() {
                self.project_search_status = "Search ready to refresh…".to_string();
            }
            if announce {
                self.set_status(format!("Indexed {} project file(s)", file_count));
            }
        }
    }

    fn install_project_watcher(&mut self) {
        self.project_watcher = None;
        self.project_watcher_rx = None;

        let Some(root) = self.project_root.clone() else {
            return;
        };

        let (tx, rx) = mpsc::channel();
        let watcher = RecommendedWatcher::new(
            move |result| {
                let _ = tx.send(result);
            },
            Config::default(),
        );

        match watcher {
            Ok(mut watcher) => match watcher.watch(&root, RecursiveMode::Recursive) {
                Ok(()) => {
                    self.project_watcher = Some(watcher);
                    self.project_watcher_rx = Some(rx);
                }
                Err(err) => {
                    self.set_status(format!("Project watcher failed: {err}"));
                }
            },
            Err(err) => self.set_status(format!("Project watcher failed: {err}")),
        }
    }

    fn drain_project_watcher_events(&mut self) {
        let Some(rx) = self.project_watcher_rx.as_ref() else {
            return;
        };

        let mut saw_change = false;
        let mut last_error = None;
        while let Ok(event) = rx.try_recv() {
            match event {
                Ok(_) => saw_change = true,
                Err(err) => last_error = Some(err.to_string()),
            }
        }

        if let Some(err) = last_error {
            self.set_status(format!("Project watcher error: {err}"));
        }
        if saw_change {
            self.refresh_project_index_if_needed(true, false);
        }
    }

    fn refresh_project_search_results_if_needed(&mut self) {
        let query = self.project_search_query.trim().to_string();
        let generation = self.project_index_generation();
        if query.is_empty() {
            self.cached_project_search_results.clear();
            self.cached_project_search_query.clear();
            self.cached_project_search_generation = generation;
            self.cached_project_search_case_sensitive = self.project_search_case_sensitive;
            self.cancel_current_project_search("Idle");
            return;
        }

        let needs_refresh = self.cached_project_search_query != query
            || self.cached_project_search_case_sensitive != self.project_search_case_sensitive
            || self.cached_project_search_generation != generation;
        if !needs_refresh {
            return;
        }

        self.cancel_current_project_search("Search cancelled: superseded by new query");
        self.cached_project_search_query = query.clone();
        self.cached_project_search_case_sensitive = self.project_search_case_sensitive;
        self.cached_project_search_generation = generation;
        self.cached_project_search_results.clear();

        let mut search_query = ProjectTextSearchQuery::new(query.clone());
        search_query.case_sensitive = self.project_search_case_sensitive;
        search_query.limit = 200;
        search_query.batch_size = 24;

        let Some(handle) = self
            .project_index
            .as_ref()
            .map(|index| index.spawn_text_search(search_query))
        else {
            return;
        };

        self.project_search_task = Some(ProjectSearchTask {
            handle,
            scanned_files: 0,
            total_matches: 0,
        });
        self.project_search_status = "Searching…".to_string();
    }

    fn active_path(&self) -> Option<PathBuf> {
        self.core
            .state()
            .active_buffer()
            .and_then(|buffer| buffer.path().cloned())
    }

    fn current_file_encoding_label(&self) -> &'static str {
        self.active_path()
            .as_ref()
            .and_then(|path| self.file_encodings.get(path).copied())
            .unwrap_or(TextFileEncoding::Utf8)
            .label()
    }

    fn path_for_view(&self, view_id: ViewId) -> Option<PathBuf> {
        let state = self.core.state();
        let view = state.view(view_id)?;
        let buffer = state.buffer(view.buffer_id())?;
        buffer.path().cloned()
    }

    fn title_for_view(&self, view_id: ViewId) -> String {
        if let Some(path) = self.path_for_view(view_id) {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string())
        } else {
            format!("Untitled {}", view_id.0)
        }
    }

    fn active_title(&self) -> String {
        if let Some(path) = self.active_path() {
            path.display().to_string()
        } else if let Some(view_id) = self.core.state().active_view_id() {
            self.title_for_view(view_id)
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

    fn file_dirty_for_view(&self, view_id: ViewId) -> bool {
        let state = self.core.state();
        let Some(view) = state.view(view_id) else {
            return false;
        };
        state
            .buffer(view.buffer_id())
            .map(|buffer| buffer.is_dirty())
            .unwrap_or(false)
    }

    fn switch_to_view(&mut self, view_id: ViewId) {
        self.core.state_mut().set_active_view(view_id);
        self.sync_from_core(true);
        self.set_status(format!("Switched to {}", self.title_for_view(view_id)));
    }

    fn request_close_view(&mut self, view_id: ViewId) {
        if self.file_dirty_for_view(view_id) {
            self.pending_close_view = Some(view_id);
        } else {
            self.close_view(view_id);
        }
    }

    fn close_view(&mut self, view_id: ViewId) {
        let title = self.title_for_view(view_id);
        let next_active = self.core.state_mut().close_view(view_id);
        if next_active.is_none() {
            self.core.open_scratch_buffer();
        }
        self.pending_close_view = None;
        self.sync_from_core(true);
        self.set_status(format!("Closed {title}"));
    }

    fn goto_line(&mut self) {
        let Ok(line) = self.goto_line_query.trim().parse::<u64>() else {
            self.set_status("Goto Line expects a positive integer");
            return;
        };
        let mut args = Map::new();
        args.insert("line".to_string(), Value::Number(line.into()));
        self.command_with_args("goto_line", args);
        self.show_goto_line_panel = false;
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
        self.open_file_path(path);
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
        let previous_path = self.active_path();
        let encoding = previous_path
            .as_ref()
            .and_then(|current| self.file_encodings.get(current).copied())
            .or_else(|| self.file_encodings.get(&path).copied())
            .unwrap_or(TextFileEncoding::Utf8);

        match fs::write(&path, encoding.encode(&contents)) {
            Ok(()) => {
                if let Some(previous_path) = previous_path.as_ref() {
                    if previous_path != &path {
                        self.file_encodings.remove(previous_path);
                    }
                }
                self.file_encodings.insert(path.clone(), encoding);
                if let Some(buffer) = self.core.state_mut().active_buffer_mut() {
                    buffer.set_path(path.clone());
                    buffer.set_dirty(false);
                }
                self.set_status(format!("Saved {} ({})", path.display(), encoding.label()));
            }
            Err(err) => self.set_status(format!("Save failed: {err}")),
        }
    }

    fn open_folder_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        self.set_project_root(path);
    }

    fn set_project_root(&mut self, path: PathBuf) {
        self.project_root = Some(path.clone());
        self.project_settings_path = Some(path.join(".needle").join("settings.json"));
        self.project_settings_mtime = None;
        self.quick_open_query.clear();
        self.project_search_query.clear();
        self.cached_project_search_query.clear();
        self.cached_project_search_results.clear();
        self.cancel_current_project_search("Idle");
        self.expanded_dirs.clear();
        self.expanded_dirs.insert(path.clone(), true);
        self.push_recent_project(path.clone());
        self.reload_project_settings_if_needed(true);
        self.refresh_project_index_if_needed(true, false);
        self.install_project_watcher();
        self.set_status(format!("Opened folder {}", path.display()));
    }

    fn push_recent_project(&mut self, path: PathBuf) {
        self.recent_projects.retain(|existing| existing != &path);
        self.recent_projects.insert(0, path);
        self.recent_projects.truncate(12);
        save_recent_projects(&self.recent_projects);
    }

    fn find_view_by_path(&self, path: &Path) -> Option<ViewId> {
        self.core.state().view_ids().into_iter().find(|view_id| {
            self.path_for_view(*view_id)
                .as_deref()
                .map(|candidate| candidate == path)
                .unwrap_or(false)
        })
    }

    fn open_file_path(&mut self, path: PathBuf) {
        if let Some(view_id) = self.find_view_by_path(&path) {
            self.switch_to_view(view_id);
            return;
        }

        match read_text_file(&path) {
            Ok(decoded) => {
                let encoding = decoded.encoding;
                let contents = decoded.text;
                let (buffer_id, _view_id) = self.core.state_mut().create_empty_buffer_and_view();
                if let Some(buffer) = self.core.state_mut().buffer_mut(buffer_id) {
                    if let Err(err) = buffer.replace(0..0, &contents) {
                        self.set_status(format!("Open failed: {err}"));
                        return;
                    }
                    buffer.set_path(path.clone());
                    buffer.set_dirty(false);
                }
                self.file_encodings.insert(path.clone(), encoding);
                self.sync_from_core(true);
                self.set_status(format!("Opened {} ({})", path.display(), encoding.label()));
            }
            Err(err) => self.set_status(format!("Open failed: {err}")),
        }
    }

    fn reload_user_settings_if_needed(&mut self, force: bool) {
        let Some(path) = self.user_settings_path.clone() else {
            return;
        };

        let old_show_hidden = self.resolved_settings.show_hidden_files;
        let metadata = fs::metadata(&path).ok();
        let modified = metadata.as_ref().and_then(|meta| meta.modified().ok());
        let changed = force || modified != self.user_settings_mtime;
        if !changed {
            return;
        }

        self.user_settings_mtime = modified;
        if metadata.is_none() {
            self.user_settings = ProjectSettingsFile::default();
            self.apply_resolved_settings();
            if self.project_root.is_some()
                && old_show_hidden != self.resolved_settings.show_hidden_files
            {
                self.refresh_project_index_if_needed(true, false);
            }
            return;
        }

        match read_settings_file(&path) {
            Ok(settings) => {
                self.user_settings = settings;
                self.apply_resolved_settings();
                if self.project_root.is_some()
                    && old_show_hidden != self.resolved_settings.show_hidden_files
                {
                    self.refresh_project_index_if_needed(true, false);
                }
            }
            Err(err) => self.set_status(format!("Failed to load {}: {err}", path.display())),
        }
    }

    fn reload_project_settings_if_needed(&mut self, force: bool) {
        let Some(path) = self.project_settings_path.clone() else {
            return;
        };

        let old_show_hidden = self.resolved_settings.show_hidden_files;
        let metadata = fs::metadata(&path).ok();
        let modified = metadata.as_ref().and_then(|meta| meta.modified().ok());
        let changed = force || modified != self.project_settings_mtime;
        if !changed {
            return;
        }

        self.project_settings_mtime = modified;
        if metadata.is_none() {
            self.project_settings = ProjectSettingsFile::default();
            self.apply_resolved_settings();
            if self.project_root.is_some()
                && old_show_hidden != self.resolved_settings.show_hidden_files
            {
                self.refresh_project_index_if_needed(true, false);
            }
            return;
        }

        match read_settings_file(&path) {
            Ok(settings) => {
                self.project_settings = settings;
                self.apply_resolved_settings();
                if self.project_root.is_some()
                    && (force || old_show_hidden != self.resolved_settings.show_hidden_files)
                {
                    self.refresh_project_index_if_needed(true, false);
                }
            }
            Err(err) => self.set_status(format!("Failed to load {}: {err}", path.display())),
        }
    }

    fn selected_texts(&self) -> Result<Vec<String>, BufferError> {
        let state = self.core.state();
        let buffer = state.active_buffer().ok_or(BufferError::NoActiveView)?;
        let regions = state
            .active_selection_regions()
            .ok_or(BufferError::NoActiveView)?;
        let mut out = Vec::new();
        for region in regions {
            if region.empty() {
                continue;
            }
            out.push(buffer.substr(region.begin()..region.end())?.to_string());
        }
        Ok(out)
    }

    fn copy_selection_to_clipboard(&mut self) {
        let pieces = match self.selected_texts() {
            Ok(pieces) => pieces,
            Err(err) => {
                self.set_status(format!("Copy failed: {err}"));
                return;
            }
        };
        if pieces.is_empty() {
            self.set_status("Nothing selected to copy");
            return;
        }
        let text = pieces.join("\n");
        match self.clipboard.as_mut() {
            Some(clipboard) => match clipboard.set_text(text) {
                Ok(()) => self.set_status("Copied selection"),
                Err(err) => self.set_status(format!("Copy failed: {err}")),
            },
            None => self.set_status("Clipboard unavailable"),
        }
    }

    fn cut_selection_to_clipboard(&mut self) {
        let pieces = match self.selected_texts() {
            Ok(pieces) => pieces,
            Err(err) => {
                self.set_status(format!("Cut failed: {err}"));
                return;
            }
        };
        if pieces.is_empty() {
            self.set_status("Nothing selected to cut");
            return;
        }
        let text = pieces.join("\n");
        let clipboard_result = match self.clipboard.as_mut() {
            Some(clipboard) => clipboard.set_text(text),
            None => {
                self.set_status("Clipboard unavailable");
                return;
            }
        };
        if let Err(err) = clipboard_result {
            self.set_status(format!("Cut failed: {err}"));
            return;
        }
        match self.core.state_mut().replace_active_selections("") {
            Ok(count) => {
                self.sync_from_core(true);
                self.set_status(format!("Cut {} selection(s)", count));
            }
            Err(err) => self.set_status(format!("Cut failed: {err}")),
        }
    }

    fn paste_from_clipboard(&mut self) {
        let text = match self.clipboard.as_mut() {
            Some(clipboard) => match clipboard.get_text() {
                Ok(text) => text,
                Err(err) => {
                    self.set_status(format!("Paste failed: {err}"));
                    return;
                }
            },
            None => {
                self.set_status("Clipboard unavailable");
                return;
            }
        };
        match self.core.state_mut().replace_active_selections(&text) {
            Ok(count) => {
                self.sync_from_core(true);
                self.set_status(format!("Pasted into {} selection(s)", count));
            }
            Err(err) => self.set_status(format!("Paste failed: {err}")),
        }
    }

    fn update_selection_from_egui(&mut self, primary_char: usize, secondary_char: usize) {
        if self.suppress_selection_sync_once {
            self.suppress_selection_sync_once = false;
            return;
        }

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

    fn editor_widget_id(&self) -> egui::Id {
        egui::Id::new("needle-editor-text")
    }

    fn set_editor_selection(&mut self, ctx: &egui::Context, start_byte: usize, end_byte: usize) {
        let start_char = byte_index_to_char_index(&self.editor_text, start_byte);
        let end_char = byte_index_to_char_index(&self.editor_text, end_byte);
        let id = self.editor_widget_id();
        let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
        state.cursor.set_char_range(Some(CCursorRange::two(
            CCursor::new(end_char),
            CCursor::new(start_char),
        )));
        egui::TextEdit::store_state(ctx, id, state);
        self.suppress_selection_sync_once = true;
        ctx.memory_mut(|mem| mem.request_focus(id));
    }

    fn select_match(&mut self, ctx: &egui::Context, start: usize, end: usize) {
        if let Err(err) = self
            .core
            .state_mut()
            .set_active_selections([Region::new(start, end)])
        {
            self.set_status(format!("Find failed: {err}"));
            return;
        }
        self.set_editor_selection(ctx, start, end);
        let match_text = self
            .editor_text
            .get(start..end)
            .unwrap_or("")
            .replace('\n', "\\n");
        self.set_status(format!("Matched: {match_text}"));
    }

    fn find_next(&mut self, ctx: &egui::Context) {
        let query = self.find_query.clone();
        if query.is_empty() {
            self.set_status("Find query is empty");
            return;
        }
        let start_from = self
            .core
            .state()
            .active_selection_regions()
            .and_then(|regions| regions.last().map(|region| region.end()))
            .unwrap_or(0)
            .min(self.editor_text.len());

        let haystack = &self.editor_text;
        let found = haystack[start_from..]
            .find(&query)
            .map(|offset| start_from + offset)
            .or_else(|| haystack[..start_from].find(&query));

        match found {
            Some(start) => self.select_match(ctx, start, start + query.len()),
            None => self.set_status("No match found"),
        }
    }

    fn find_previous(&mut self, ctx: &egui::Context) {
        let query = self.find_query.clone();
        if query.is_empty() {
            self.set_status("Find query is empty");
            return;
        }
        let end_at = self
            .core
            .state()
            .active_selection_regions()
            .and_then(|regions| regions.last().map(|region| region.begin()))
            .unwrap_or(self.editor_text.len())
            .min(self.editor_text.len());

        let haystack = &self.editor_text;
        let found = haystack[..end_at].rfind(&query).or_else(|| {
            haystack[end_at..]
                .rfind(&query)
                .map(|offset| end_at + offset)
        });

        match found {
            Some(start) => self.select_match(ctx, start, start + query.len()),
            None => self.set_status("No match found"),
        }
    }

    fn replace_current(&mut self, ctx: &egui::Context) {
        if self.find_query.is_empty() {
            self.set_status("Find query is empty");
            return;
        }

        let selected_text = match self.selected_texts() {
            Ok(parts) => parts.join("\n"),
            Err(err) => {
                self.set_status(format!("Replace failed: {err}"));
                return;
            }
        };

        if selected_text != self.find_query {
            self.find_next(ctx);
            return;
        }

        match self
            .core
            .state_mut()
            .replace_active_selections(&self.replace_query)
        {
            Ok(count) => {
                self.sync_from_core(true);
                self.set_status(format!("Replaced in {} selection(s)", count));
            }
            Err(err) => self.set_status(format!("Replace failed: {err}")),
        }
    }

    fn replace_all(&mut self) {
        if self.find_query.is_empty() {
            self.set_status("Find query is empty");
            return;
        }
        let Some(buffer_id) = self.core.state().active_buffer_id() else {
            self.set_status("Replace failed: no active buffer");
            return;
        };
        let original = self.editor_text.clone();
        let replacement = original.replace(&self.find_query, &self.replace_query);
        if replacement == original {
            self.set_status("No match found");
            return;
        }
        let replaced_count = original.matches(&self.find_query).count();
        match self
            .core
            .state_mut()
            .apply_edit(buffer_id, 0..original.len(), &replacement)
        {
            Ok(_) => {
                self.sync_from_core(true);
                self.set_status(format!("Replaced {replaced_count} occurrence(s)"));
            }
            Err(err) => self.set_status(format!("Replace all failed: {err}")),
        }
    }

    fn find_match_ranges(&self) -> Vec<std::ops::Range<usize>> {
        if self.find_query.is_empty() {
            return Vec::new();
        }

        self.editor_text
            .match_indices(&self.find_query)
            .map(|(start, matched)| start..start + matched.len())
            .collect()
    }

    fn current_find_match_index(&self, matches: &[std::ops::Range<usize>]) -> Option<usize> {
        let active = self
            .core
            .state()
            .active_selection_regions()
            .and_then(|regions| regions.last().cloned())?;
        matches
            .iter()
            .position(|range| range.start == active.begin() && range.end == active.end())
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

    fn execute_ui_or_core_command(&mut self, name: &str) {
        match name {
            "new_file" => self.new_file(),
            "open_file" => self.open_file_dialog(),
            "open_folder" => self.open_folder_dialog(),
            "save" => self.save_active_file(),
            "save_as" => self.save_active_file_as(),
            "copy" => self.copy_selection_to_clipboard(),
            "cut" => self.cut_selection_to_clipboard(),
            "paste" => self.paste_from_clipboard(),
            "show_command_palette" => self.show_command_palette = true,
            "show_find" => self.show_find_panel = true,
            "show_project_search" => self.show_project_search_panel = true,
            "goto_line_panel" => self.show_goto_line_panel = true,
            other => self.command(other),
        }
    }

    fn key_matches(input: &egui::InputState, binding: &KeyBindingConfig) -> bool {
        let Some(key) = parse_key(&binding.key) else {
            return false;
        };
        if !input.key_pressed(key) {
            return false;
        }

        let wants_command = binding
            .modifiers
            .iter()
            .any(|m| eq_mod(m, "command") || eq_mod(m, "ctrl"));
        let wants_shift = binding.modifiers.iter().any(|m| eq_mod(m, "shift"));
        let wants_alt = binding
            .modifiers
            .iter()
            .any(|m| eq_mod(m, "alt") || eq_mod(m, "option"));

        input.modifiers.command == wants_command
            && input.modifiers.shift == wants_shift
            && input.modifiers.alt == wants_alt
    }

    fn render_sidebar(&mut self, ctx: &egui::Context) {
        let Some(project_root) = self.project_root.clone() else {
            return;
        };

        egui::SidePanel::left("project_sidebar")
            .resizable(true)
            .default_width(self.sidebar_width)
            .width_range(140.0..=480.0)
            .show(ctx, |ui| {
                self.sidebar_width = ui.available_width();
                ui.heading("Project");
                ui.label(project_root.display().to_string());
                ui.label(format!(
                    "{} indexed file(s)",
                    self.indexed_project_file_count()
                ));
                ui.label(if self.project_watcher.is_some() {
                    "Watcher: active"
                } else {
                    "Watcher: unavailable"
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Refresh Settings").clicked() {
                        self.reload_project_settings_if_needed(true);
                    }
                    if ui.button("Refresh Index").clicked() {
                        self.refresh_project_index_if_needed(true, true);
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.render_path_node(ui, &project_root, 0);
                });
            });
    }

    fn render_path_node(&mut self, ui: &mut egui::Ui, path: &Path, depth: usize) {
        let name = if depth == 0 {
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string())
        } else {
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string())
        };

        if path.is_dir() {
            let is_open = *self
                .expanded_dirs
                .entry(path.to_path_buf())
                .or_insert(depth == 0);
            ui.horizontal(|ui| {
                ui.add_space((depth as f32) * 12.0);
                let arrow = if is_open { "▾" } else { "▸" };
                if ui.small_button(arrow).clicked() {
                    self.expanded_dirs.insert(path.to_path_buf(), !is_open);
                }
                if ui.selectable_label(false, format!("📁 {name}")).clicked() {
                    self.expanded_dirs.insert(path.to_path_buf(), !is_open);
                }
            });

            if is_open {
                let mut children: Vec<PathBuf> = match fs::read_dir(path) {
                    Ok(entries) => entries.filter_map(|e| e.ok().map(|x| x.path())).collect(),
                    Err(_) => Vec::new(),
                };
                children.retain(|child| {
                    self.resolved_settings.show_hidden_files
                        || child
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|n| !n.starts_with('.'))
                            .unwrap_or(true)
                });
                children.sort_by(|a, b| {
                    let a_dir = a.is_dir();
                    let b_dir = b.is_dir();
                    b_dir.cmp(&a_dir).then_with(|| a.cmp(b))
                });
                for child in children {
                    self.render_path_node(ui, &child, depth + 1);
                }
            }
        } else {
            ui.horizontal(|ui| {
                ui.add_space((depth as f32) * 12.0 + 18.0);
                if ui.selectable_label(false, format!("📄 {name}")).clicked() {
                    self.open_file_path(path.to_path_buf());
                }
            });
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let mut new_file = false;
        let mut open = false;
        let mut save = false;
        let mut save_as = false;
        let mut open_folder = false;
        let mut undo = false;
        let mut redo = false;
        let mut palette = false;
        let mut quick_open = false;
        let mut project_search = false;
        let mut find = false;
        let mut goto = false;
        let mut copy = false;
        let mut cut = false;
        let mut paste = false;
        let mut select_all = false;
        let mut close = false;
        let mut custom_commands: Vec<String> = Vec::new();

        ctx.input(|input| {
            let command = input.modifiers.command;
            if command && input.key_pressed(egui::Key::N) {
                new_file = true;
            }
            if command && input.key_pressed(egui::Key::O) {
                open = true;
            }
            if command && input.key_pressed(egui::Key::S) && input.modifiers.shift {
                save_as = true;
            } else if command && input.key_pressed(egui::Key::S) {
                save = true;
            }
            if command && input.modifiers.shift && input.key_pressed(egui::Key::O) {
                open_folder = true;
            }
            if command && input.key_pressed(egui::Key::Z) && input.modifiers.shift {
                redo = true;
            } else if command && input.key_pressed(egui::Key::Z) {
                undo = true;
            }
            if command && input.key_pressed(egui::Key::Y) {
                redo = true;
            }
            if command && input.modifiers.shift && input.key_pressed(egui::Key::P) {
                palette = true;
            } else if command && input.key_pressed(egui::Key::P) {
                quick_open = true;
            }
            if command && input.modifiers.shift && input.key_pressed(egui::Key::F) {
                project_search = true;
            } else if command && input.key_pressed(egui::Key::F) {
                find = true;
            }
            if command && input.key_pressed(egui::Key::G) {
                goto = true;
            }
            if command && input.key_pressed(egui::Key::C) {
                copy = true;
            }
            if command && input.key_pressed(egui::Key::X) {
                cut = true;
            }
            if command && input.key_pressed(egui::Key::V) {
                paste = true;
            }
            if command && input.key_pressed(egui::Key::A) {
                select_all = true;
            }
            if command && input.key_pressed(egui::Key::W) {
                close = true;
            }
            for binding in self.active_keybindings() {
                if Self::key_matches(input, &binding) {
                    custom_commands.push(binding.command.clone());
                }
            }
        });

        if new_file {
            self.new_file();
        }
        if open {
            self.open_file_dialog();
        }
        if save {
            self.save_active_file();
        }
        if save_as {
            self.save_active_file_as();
        }
        if open_folder {
            self.open_folder_dialog();
        }
        if undo {
            self.command("undo");
        }
        if redo {
            self.command("redo");
        }
        if palette {
            self.show_command_palette = true;
        }
        if quick_open {
            self.show_quick_open_panel = true;
        }
        if project_search {
            self.show_project_search_panel = true;
        }
        if find {
            self.show_find_panel = true;
        }
        if goto {
            self.show_goto_line_panel = true;
        }
        if copy {
            self.copy_selection_to_clipboard();
        }
        if cut {
            self.cut_selection_to_clipboard();
        }
        if paste {
            self.paste_from_clipboard();
        }
        if select_all {
            self.command("select_all");
        }
        if close {
            if let Some(view_id) = self.core.state().active_view_id() {
                self.request_close_view(view_id);
            }
        }
        for command in custom_commands {
            self.execute_ui_or_core_command(&command);
        }
    }

    fn render_tabs(&mut self, ctx: &egui::Context) {
        let active_view = self.core.state().active_view_id();
        let view_ids = self.core.state().view_ids();
        let mut to_close = None;

        egui::TopBottomPanel::top("tabs_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for view_id in view_ids {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            let mut label = self.title_for_view(view_id);
                            if self.file_dirty_for_view(view_id) {
                                label.push('*');
                            }
                            let selected = active_view == Some(view_id);
                            if ui.selectable_label(selected, label).clicked() {
                                self.switch_to_view(view_id);
                            }
                            if ui.small_button("×").clicked() {
                                to_close = Some(view_id);
                            }
                        });
                    });
                }
            });
        });

        if let Some(view_id) = to_close {
            self.request_close_view(view_id);
        }
    }

    fn render_command_palette(&mut self, ctx: &egui::Context) {
        if !self.show_command_palette {
            return;
        }

        let mut open = true;
        let matches = self.palette_matches();
        let first_name = matches.first().map(|spec| spec.name.clone());

        egui::Window::new("Command Palette")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.command_palette_query)
                        .hint_text("Type a command name…"),
                );
                response.request_focus();

                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                if enter_pressed {
                    if let Some(name) = first_name.clone() {
                        self.execute_palette_command(&name);
                    }
                }

                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for spec in &matches {
                            let button = ui.selectable_label(
                                false,
                                format!("{} — {}", spec.name, spec.description),
                            );
                            if button.clicked() {
                                self.execute_palette_command(&spec.name);
                            }
                        }
                        if matches.is_empty() {
                            ui.label("No matching commands");
                        }
                    });
            });

        self.show_command_palette = open;
    }

    fn render_quick_open_panel(&mut self, ctx: &egui::Context) {
        if !self.show_quick_open_panel {
            return;
        }

        let mut open = true;
        let matches = self.quick_open_matches();
        let first_path = matches.first().map(|entry| entry.path.clone());

        egui::Window::new("Quick Open")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                if self.project_root.is_none() {
                    ui.label("Open a folder first.");
                    return;
                }
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.quick_open_query)
                        .hint_text("Type a file name…"),
                );
                response.request_focus();

                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                if enter_pressed {
                    if let Some(path) = first_path.clone() {
                        self.open_file_path(path);
                        self.show_quick_open_panel = false;
                    }
                }

                ui.label(format!("{} match(es)", matches.len()));
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for entry in &matches {
                            if ui.selectable_label(false, &entry.relative_path).clicked() {
                                self.open_file_path(entry.path.clone());
                                self.show_quick_open_panel = false;
                            }
                        }
                        if matches.is_empty() {
                            ui.label("No matching files");
                        }
                    });
            });

        self.show_quick_open_panel = open;
    }

    fn render_recent_projects_panel(&mut self, ctx: &egui::Context) {
        if !self.show_recent_projects_panel {
            return;
        }

        let mut open = true;
        let projects = self.recent_projects.clone();

        egui::Window::new("Recent Projects")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                if projects.is_empty() {
                    ui.label("No recent projects yet.");
                    return;
                }
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for path in &projects {
                            if ui
                                .selectable_label(false, path.display().to_string())
                                .clicked()
                            {
                                self.set_project_root(path.clone());
                                self.show_recent_projects_panel = false;
                            }
                        }
                    });
            });

        self.show_recent_projects_panel = open;
    }

    fn render_project_search_panel(&mut self, ctx: &egui::Context) {
        if !self.show_project_search_panel {
            return;
        }

        self.refresh_project_search_results_if_needed();
        let mut open = true;
        let results = self.project_search_matches();

        egui::Window::new("Find in Project")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(720.0)
            .show(ctx, |ui| {
                if self.project_root.is_none() {
                    ui.label("Open a folder first.");
                    return;
                }
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.project_search_query)
                        .hint_text("Type text to search in project…"),
                );
                response.request_focus();
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.project_search_case_sensitive, "Case sensitive");
                    if self.project_search_task.is_some()
                        && ui.button("Cancel Search").clicked()
                    {
                        self.cancel_current_project_search("Search cancelled by user");
                    }
                });
                ui.label(&self.project_search_status);

                let first_result = results.first().cloned();
                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                if enter_pressed {
                    if let Some(result) = first_result {
                        self.open_file_path(result.path.clone());
                        let mut json_args = Map::new();
                        json_args
                            .insert("line".to_string(), Value::from(result.line_number as u64));
                        self.command_with_args("goto_line", json_args);
                        self.show_project_search_panel = false;
                    }
                }

                ui.label(format!("{} match(es)", results.len()));
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(360.0)
                    .show(ui, |ui| {
                        for result in &results {
                            let label = format!(
                                "{}:{}  {}",
                                result.relative_path, result.line_number, result.line_text
                            );
                            if ui.selectable_label(false, label).clicked() {
                                self.open_file_path(result.path.clone());
                                let mut json_args = Map::new();
                                json_args.insert(
                                    "line".to_string(),
                                    Value::from(result.line_number as u64),
                                );
                                self.command_with_args("goto_line", json_args);
                                self.show_project_search_panel = false;
                            }
                        }
                        if results.is_empty() {
                            ui.label("No matches yet");
                        }
                    });
            });

        self.show_project_search_panel = open;
    }

    fn render_find_panel(&mut self, ctx: &egui::Context) {
        if !self.show_find_panel {
            return;
        }

        let match_ranges = self.find_match_ranges();
        let current_match_index = self.current_find_match_index(&match_ranges);
        let mut open = true;
        egui::Window::new("Find / Replace")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.label("Find");
                let find_response = ui.add(
                    egui::TextEdit::singleline(&mut self.find_query).hint_text("Find text..."),
                );
                find_response.request_focus();

                if self.find_query.is_empty() {
                    ui.label("0 matches");
                } else if let Some(index) = current_match_index {
                    ui.label(format!(
                        "{} matches · current {}/{}",
                        match_ranges.len(),
                        index + 1,
                        match_ranges.len()
                    ));
                } else {
                    ui.label(format!("{} matches", match_ranges.len()));
                }

                ui.label("Replace");
                ui.add(
                    egui::TextEdit::singleline(&mut self.replace_query)
                        .hint_text("Replace with..."),
                );

                ui.horizontal(|ui| {
                    if ui.button("Next").clicked() {
                        self.find_next(ctx);
                    }
                    if ui.button("Previous").clicked() {
                        self.find_previous(ctx);
                    }
                    if ui.button("Replace").clicked() {
                        self.replace_current(ctx);
                    }
                    if ui.button("Replace All").clicked() {
                        self.replace_all();
                    }
                });
            });

        self.show_find_panel = open;
    }

    fn render_goto_line_panel(&mut self, ctx: &egui::Context) {
        if !self.show_goto_line_panel {
            return;
        }

        let mut open = true;
        egui::Window::new("Goto Line")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(260.0)
            .show(ctx, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.goto_line_query)
                        .hint_text("1-based line number"),
                );
                response.request_focus();

                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                if enter_pressed {
                    self.goto_line();
                }

                if ui.button("Go").clicked() {
                    self.goto_line();
                }
            });

        self.show_goto_line_panel = open;
    }

    fn render_close_confirm(&mut self, ctx: &egui::Context) {
        let Some(view_id) = self.pending_close_view else {
            return;
        };

        let mut open = true;
        let title = self.title_for_view(view_id);
        egui::Window::new("Unsaved Changes")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(360.0)
            .show(ctx, |ui| {
                ui.label(format!("Discard unsaved changes in {title}?"));
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.pending_close_view = None;
                    }
                    if ui.button("Save & Close").clicked() {
                        self.switch_to_view(view_id);
                        self.save_active_file();
                        self.close_view(view_id);
                    }
                    if ui.button("Discard").clicked() {
                        self.close_view(view_id);
                    }
                });
            });

        if !open {
            self.pending_close_view = None;
        }
    }
}

impl App for NeedleEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.reload_user_settings_if_needed(false);
        self.reload_project_settings_if_needed(false);
        self.drain_project_watcher_events();
        self.refresh_project_index_if_needed(false, false);
        self.poll_project_search_task();
        self.sync_from_core(false);
        self.handle_shortcuts(ctx);
        self.render_tabs(ctx);
        self.render_sidebar(ctx);

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            self.render_top_menu_bar(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(self.active_title());
            ui.label(if self.file_dirty() {
                "Modified"
            } else {
                "Saved"
            });
            ui.add_space(4.0);

            let editor_id = self.editor_widget_id();
            let output = egui::TextEdit::multiline(&mut self.editor_text)
                .id_source(editor_id)
                .desired_rows(32)
                .desired_width(f32::INFINITY)
                .font(egui::TextStyle::Monospace)
                .code_editor()
                .show(ui);

            if let Some((start, end)) = self.desired_editor_selection.take() {
                self.set_editor_selection(ctx, start, end);
            }

            if let Some(cursor_range) = &output.cursor_range {
                self.update_selection_from_egui(
                    cursor_range.primary.index,
                    cursor_range.secondary.index,
                );
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
                ui.label(format!("Encoding: {}", self.current_file_encoding_label()));
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

        if self.project_root.is_some() {
            let repaint_after_ms = if self.project_search_task.is_some() {
                50
            } else {
                500
            };
            ctx.request_repaint_after(Duration::from_millis(repaint_after_ms));
        }

        self.render_command_palette(ctx);
        self.render_quick_open_panel(ctx);
        self.render_recent_projects_panel(ctx);
        self.render_project_search_panel(ctx);
        self.render_find_panel(ctx);
        self.render_goto_line_panel(ctx);
        self.render_close_confirm(ctx);
    }
}

fn default_user_settings_path() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|dir| dir.join("needle").join("settings.json"))
}

fn read_settings_file(path: &Path) -> Result<ProjectSettingsFile, String> {
    let text = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let settings =
        serde_json::from_str::<ProjectSettingsFile>(&text).map_err(|err| err.to_string())?;
    validate_settings_file(&settings)?;
    Ok(settings)
}

fn validate_settings_file(settings: &ProjectSettingsFile) -> Result<(), String> {
    if let Some(width) = settings.sidebar_width {
        if !width.is_finite() {
            return Err("sidebar_width must be a finite number".to_string());
        }
    }

    for binding in &settings.keybindings {
        if binding.command.trim().is_empty() {
            return Err("keybinding command must not be empty".to_string());
        }
        if parse_key(&binding.key).is_none() {
            return Err(format!("unsupported keybinding key: {}", binding.key));
        }
        for modifier in &binding.modifiers {
            if !matches!(
                modifier.to_ascii_lowercase().as_str(),
                "command" | "ctrl" | "shift" | "alt" | "option"
            ) {
                return Err(format!("unsupported keybinding modifier: {modifier}"));
            }
        }
    }

    Ok(())
}

fn resolve_settings(user: &ProjectSettingsFile, project: &ProjectSettingsFile) -> ResolvedSettings {
    let mut keybindings = user.keybindings.clone();
    keybindings.extend(project.keybindings.clone());

    ResolvedSettings {
        show_hidden_files: project
            .show_hidden_files
            .or(user.show_hidden_files)
            .unwrap_or(false),
        sidebar_width: project
            .sidebar_width
            .or(user.sidebar_width)
            .unwrap_or(220.0),
        keybindings,
    }
}

fn keybinding_signature(binding: &KeyBindingConfig) -> String {
    let mut modifiers = binding
        .modifiers
        .iter()
        .map(|modifier| modifier.to_ascii_lowercase())
        .collect::<Vec<_>>();
    modifiers.sort();
    format!(
        "{}::{}",
        modifiers.join("+"),
        binding.key.to_ascii_uppercase()
    )
}

fn shared_prefix_len(old: &str, new: &str) -> usize {
    let mut prefix = 0;
    for (old_ch, new_ch) in old.chars().zip(new.chars()) {
        if old_ch != new_ch {
            break;
        }
        prefix += old_ch.len_utf8();
    }
    prefix
}

fn shared_suffix_len(old: &str, new: &str) -> usize {
    let mut suffix = 0;
    for (old_ch, new_ch) in old.chars().rev().zip(new.chars().rev()) {
        if suffix + old_ch.len_utf8() > old.len() || suffix + new_ch.len_utf8() > new.len() {
            break;
        }
        if old_ch != new_ch {
            break;
        }
        suffix += old_ch.len_utf8();
    }
    suffix
}

fn compute_minimal_text_edit<'a>(
    old: &str,
    new: &'a str,
) -> Option<(std::ops::Range<usize>, &'a str)> {
    if old == new {
        return None;
    }

    let prefix = shared_prefix_len(old, new);
    let old_remaining = &old[prefix..];
    let new_remaining = &new[prefix..];
    let suffix = shared_suffix_len(old_remaining, new_remaining);

    let old_end = old.len() - suffix;
    let new_end = new.len() - suffix;
    Some((prefix..old_end, &new[prefix..new_end]))
}

fn char_index_to_byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}

fn byte_index_to_char_index(text: &str, byte_index: usize) -> usize {
    text[..byte_index.min(text.len())].chars().count()
}

fn eq_mod(input: &str, target: &str) -> bool {
    input.eq_ignore_ascii_case(target)
}

fn parse_key(key: &str) -> Option<egui::Key> {
    match key.to_ascii_uppercase().as_str() {
        "A" => Some(egui::Key::A),
        "B" => Some(egui::Key::B),
        "C" => Some(egui::Key::C),
        "D" => Some(egui::Key::D),
        "E" => Some(egui::Key::E),
        "F" => Some(egui::Key::F),
        "G" => Some(egui::Key::G),
        "H" => Some(egui::Key::H),
        "I" => Some(egui::Key::I),
        "J" => Some(egui::Key::J),
        "K" => Some(egui::Key::K),
        "L" => Some(egui::Key::L),
        "M" => Some(egui::Key::M),
        "N" => Some(egui::Key::N),
        "O" => Some(egui::Key::O),
        "P" => Some(egui::Key::P),
        "Q" => Some(egui::Key::Q),
        "R" => Some(egui::Key::R),
        "S" => Some(egui::Key::S),
        "T" => Some(egui::Key::T),
        "U" => Some(egui::Key::U),
        "V" => Some(egui::Key::V),
        "W" => Some(egui::Key::W),
        "X" => Some(egui::Key::X),
        "Y" => Some(egui::Key::Y),
        "Z" => Some(egui::Key::Z),
        "ARROWUP" | "UP" => Some(egui::Key::ArrowUp),
        "ARROWDOWN" | "DOWN" => Some(egui::Key::ArrowDown),
        "ARROWLEFT" | "LEFT" => Some(egui::Key::ArrowLeft),
        "ARROWRIGHT" | "RIGHT" => Some(egui::Key::ArrowRight),
        _ => None,
    }
}

fn recent_projects_file_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join(".needle")
            .join("recent_projects.json"),
    )
}

fn load_recent_projects() -> Vec<PathBuf> {
    let Some(path) = recent_projects_file_path() else {
        return Vec::new();
    };
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(paths) = serde_json::from_str::<Vec<String>>(&text) else {
        return Vec::new();
    };
    paths.into_iter().map(PathBuf::from).collect()
}

fn save_recent_projects(projects: &[PathBuf]) {
    let Some(path) = recent_projects_file_path() else {
        return;
    };
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let payload = projects
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    let Ok(text) = serde_json::to_string_pretty(&payload) else {
        return;
    };
    let _ = fs::write(path, text);
}

fn read_text_file(path: &Path) -> Result<DecodedTextFile> {
    let bytes = fs::read(path)?;
    Ok(decode_text_bytes(&bytes))
}

fn decode_text_bytes(bytes: &[u8]) -> DecodedTextFile {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return DecodedTextFile {
            text: String::from_utf8_lossy(&bytes[3..]).into_owned(),
            encoding: TextFileEncoding::Utf8Bom,
        };
    }

    if bytes.starts_with(&[0xFF, 0xFE]) {
        let (text, _, _) = UTF_16LE.decode(&bytes[2..]);
        return DecodedTextFile {
            text: text.into_owned(),
            encoding: TextFileEncoding::Utf16Le,
        };
    }

    if bytes.starts_with(&[0xFE, 0xFF]) {
        let (text, _, _) = UTF_16BE.decode(&bytes[2..]);
        return DecodedTextFile {
            text: text.into_owned(),
            encoding: TextFileEncoding::Utf16Be,
        };
    }

    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
        return DecodedTextFile {
            text,
            encoding: TextFileEncoding::Utf8,
        };
    }

    let mut detector = EncodingDetector::new();
    detector.feed(bytes, true);
    let guessed = detector.guess(None, true);
    let encoding = if guessed == UTF_16LE {
        TextFileEncoding::Utf16Le
    } else if guessed == UTF_16BE {
        TextFileEncoding::Utf16Be
    } else {
        TextFileEncoding::Legacy(guessed)
    };
    let (text, _, _) = guessed.decode(bytes);
    DecodedTextFile {
        text: text.into_owned(),
        encoding,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        compute_minimal_text_edit, decode_text_bytes, resolve_settings, validate_settings_file,
        KeyBindingConfig, ProjectSettingsFile, TextFileEncoding,
    };

    #[test]
    fn computes_minimal_text_edit_for_insertion() {
        let (range, replacement) =
            compute_minimal_text_edit("hello world", "hello brave world").unwrap();
        assert_eq!(range, 6..6);
        assert_eq!(replacement, "brave ");
    }

    #[test]
    fn computes_minimal_text_edit_for_deletion() {
        let (range, replacement) =
            compute_minimal_text_edit("hello brave world", "hello world").unwrap();
        assert_eq!(range, 6..12);
        assert_eq!(replacement, "");
    }

    #[test]
    fn computes_minimal_text_edit_for_replacement_with_utf8() {
        let (range, replacement) = compute_minimal_text_edit("你好，世界", "你好，Rust").unwrap();
        assert_eq!(range, "你好，".len().."你好，世界".len());
        assert_eq!(replacement, "Rust");
    }

    #[test]
    fn returns_none_when_text_is_unchanged() {
        assert!(compute_minimal_text_edit("same", "same").is_none());
    }

    #[test]
    fn resolves_settings_with_project_overrides() {
        let user = ProjectSettingsFile {
            show_hidden_files: Some(true),
            sidebar_width: Some(300.0),
            keybindings: vec![KeyBindingConfig {
                command: "show_find".to_string(),
                key: "F".to_string(),
                modifiers: vec!["command".to_string()],
            }],
        };
        let project = ProjectSettingsFile {
            show_hidden_files: Some(false),
            sidebar_width: Some(260.0),
            keybindings: vec![KeyBindingConfig {
                command: "show_project_search".to_string(),
                key: "F".to_string(),
                modifiers: vec!["command".to_string(), "shift".to_string()],
            }],
        };

        let resolved = resolve_settings(&user, &project);
        assert!(!resolved.show_hidden_files);
        assert_eq!(resolved.sidebar_width, 260.0);
        assert_eq!(resolved.keybindings.len(), 2);
    }

    #[test]
    fn rejects_invalid_keybinding_modifier() {
        let settings = ProjectSettingsFile {
            show_hidden_files: None,
            sidebar_width: None,
            keybindings: vec![KeyBindingConfig {
                command: "show_find".to_string(),
                key: "F".to_string(),
                modifiers: vec!["hyper".to_string()],
            }],
        };

        let err = validate_settings_file(&settings).unwrap_err();
        assert!(err.contains("unsupported keybinding modifier"));
    }

    #[test]
    fn decodes_utf8_text() {
        let decoded = decode_text_bytes("中文 UTF-8".as_bytes());
        assert_eq!(decoded.text, "中文 UTF-8");
        assert!(matches!(decoded.encoding, TextFileEncoding::Utf8));
    }

    #[test]
    fn decodes_gbk_text() {
        let (encoded, _, _) = encoding_rs::GBK.encode("中文内容");
        let decoded = decode_text_bytes(encoded.as_ref());
        assert_eq!(decoded.text, "中文内容");
        assert!(
            matches!(decoded.encoding, TextFileEncoding::Legacy(enc) if enc == encoding_rs::GBK)
        );
    }
}
