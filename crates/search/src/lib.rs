use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    thread,
    time::SystemTime,
};

use anyhow::Result;
use chardetng::EncodingDetector;
use encoding_rs::{UTF_16BE, UTF_16LE};

pub fn crate_name() -> &'static str {
    "needle-search"
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFileEntry {
    pub path: PathBuf,
    pub relative_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSearchMatch {
    pub path: PathBuf,
    pub relative_path: String,
    pub line_number: usize,
    pub line_text: String,
}

#[derive(Debug, Clone)]
struct IndexedProjectFile {
    entry: ProjectFileEntry,
    lower_relative_path: String,
}

#[derive(Debug, Clone)]
pub struct ProjectIndex {
    root: PathBuf,
    show_hidden: bool,
    generation: u64,
    files: Vec<IndexedProjectFile>,
}

impl ProjectIndex {
    pub fn build(root: &Path, show_hidden: bool) -> Self {
        let mut files = Vec::new();
        let generation = scan_project_files(root, root, show_hidden, &mut files);
        files.sort_by(|a, b| a.entry.relative_path.cmp(&b.entry.relative_path));
        Self {
            root: root.to_path_buf(),
            show_hidden,
            generation,
            files,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn show_hidden(&self) -> bool {
        self.show_hidden
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn quick_open_matches(&self, query: &str, limit: usize) -> Vec<ProjectFileEntry> {
        let limit = limit.max(1);
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return self
                .files
                .iter()
                .take(limit)
                .map(|file| file.entry.clone())
                .collect();
        }

        let mut scored = self
            .files
            .iter()
            .filter_map(|file| {
                fuzzy_match_score(&file.lower_relative_path, &query)
                    .map(|score| (score, file.entry.clone()))
            })
            .collect::<Vec<_>>();
        scored.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.relative_path.cmp(&b.1.relative_path))
        });
        scored
            .into_iter()
            .map(|(_, entry)| entry)
            .take(limit)
            .collect()
    }

    pub fn search_text(&self, query: ProjectTextSearchQuery) -> Vec<ProjectSearchMatch> {
        run_text_search(
            self.files.iter().map(|file| file.entry.clone()).collect(),
            query,
            |_, _| true,
        )
        .0
    }

    pub fn spawn_text_search(
        &self,
        query: ProjectTextSearchQuery,
    ) -> Receiver<ProjectTextSearchEvent> {
        let files = self
            .files
            .iter()
            .map(|file| file.entry.clone())
            .collect::<Vec<_>>();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let (_, summary) = run_text_search(files, query, |batch, is_complete| {
                let event = if is_complete {
                    ProjectTextSearchEvent::Complete(ProjectTextSearchSummary {
                        scanned_files: batch.scanned_files,
                        total_matches: batch.total_matches,
                    })
                } else {
                    ProjectTextSearchEvent::Batch(batch)
                };
                tx.send(event).is_ok()
            });
            let _ = summary;
        });
        rx
    }
}

#[derive(Debug, Clone)]
pub struct ProjectTextSearchQuery {
    pub text: String,
    pub case_sensitive: bool,
    pub limit: usize,
    pub batch_size: usize,
}

impl ProjectTextSearchQuery {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            case_sensitive: false,
            limit: 200,
            batch_size: 32,
        }
    }

    fn needle(&self) -> Option<String> {
        let text = self.text.trim();
        if text.is_empty() {
            return None;
        }
        Some(if self.case_sensitive {
            text.to_string()
        } else {
            text.to_lowercase()
        })
    }
}

#[derive(Debug, Clone)]
pub enum ProjectTextSearchEvent {
    Batch(ProjectTextSearchBatch),
    Complete(ProjectTextSearchSummary),
}

#[derive(Debug, Clone)]
pub struct ProjectTextSearchBatch {
    pub matches: Vec<ProjectSearchMatch>,
    pub scanned_files: usize,
    pub total_matches: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectTextSearchSummary {
    pub scanned_files: usize,
    pub total_matches: usize,
}

fn run_text_search<F>(
    files: Vec<ProjectFileEntry>,
    query: ProjectTextSearchQuery,
    mut on_progress: F,
) -> (Vec<ProjectSearchMatch>, ProjectTextSearchSummary)
where
    F: FnMut(ProjectTextSearchBatch, bool) -> bool,
{
    let mut all_matches = Vec::new();
    let mut summary = ProjectTextSearchSummary {
        scanned_files: 0,
        total_matches: 0,
    };

    let Some(needle) = query.needle() else {
        let _ = on_progress(
            ProjectTextSearchBatch {
                matches: Vec::new(),
                scanned_files: 0,
                total_matches: 0,
            },
            true,
        );
        return (all_matches, summary);
    };

    let batch_size = query.batch_size.max(1);
    let limit = query.limit;
    if limit == 0 {
        let _ = on_progress(
            ProjectTextSearchBatch {
                matches: Vec::new(),
                scanned_files: 0,
                total_matches: 0,
            },
            true,
        );
        return (all_matches, summary);
    }

    let mut batch = Vec::new();
    for entry in files {
        if summary.total_matches >= limit {
            break;
        }

        summary.scanned_files += 1;
        let Ok(text) = read_text_file(&entry.path) else {
            continue;
        };

        for (index, line) in text.lines().enumerate() {
            let matched = if query.case_sensitive {
                line.contains(&needle)
            } else {
                line.to_lowercase().contains(&needle)
            };
            if !matched {
                continue;
            }

            let item = ProjectSearchMatch {
                path: entry.path.clone(),
                relative_path: entry.relative_path.clone(),
                line_number: index + 1,
                line_text: line.trim().to_string(),
            };
            summary.total_matches += 1;
            all_matches.push(item.clone());
            batch.push(item);

            if batch.len() >= batch_size {
                if !on_progress(
                    ProjectTextSearchBatch {
                        matches: std::mem::take(&mut batch),
                        scanned_files: summary.scanned_files,
                        total_matches: summary.total_matches,
                    },
                    false,
                ) {
                    return (all_matches, summary);
                }
            }

            if summary.total_matches >= limit {
                break;
            }
        }
    }

    if !batch.is_empty()
        && !on_progress(
            ProjectTextSearchBatch {
                matches: batch,
                scanned_files: summary.scanned_files,
                total_matches: summary.total_matches,
            },
            false,
        )
    {
        return (all_matches, summary);
    }

    let _ = on_progress(
        ProjectTextSearchBatch {
            matches: Vec::new(),
            scanned_files: summary.scanned_files,
            total_matches: summary.total_matches,
        },
        true,
    );
    (all_matches, summary)
}

fn scan_project_files(
    root: &Path,
    current: &Path,
    show_hidden: bool,
    out: &mut Vec<IndexedProjectFile>,
) -> u64 {
    let mut fingerprint = 0_u64;
    scan_project_files_into(root, current, show_hidden, out, &mut fingerprint);
    fingerprint
}

fn scan_project_files_into(
    root: &Path,
    current: &Path,
    show_hidden: bool,
    out: &mut Vec<IndexedProjectFile>,
    fingerprint: &mut u64,
) {
    let entries = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !show_hidden && name.starts_with('.') {
            continue;
        }

        let metadata = entry.metadata().ok();
        let len = metadata.as_ref().map(|meta| meta.len()).unwrap_or(0);
        let modified = metadata
            .as_ref()
            .and_then(|meta| meta.modified().ok())
            .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or(0);

        *fingerprint = fingerprint
            .wrapping_mul(131)
            .wrapping_add(hash_str(path.to_string_lossy().as_ref()))
            .wrapping_add(len)
            .wrapping_add(modified);

        if path.is_dir() {
            scan_project_files_into(root, &path, show_hidden, out, fingerprint);
        } else if let Ok(relative) = path.strip_prefix(root) {
            let relative_path = relative.display().to_string();
            out.push(IndexedProjectFile {
                entry: ProjectFileEntry {
                    path: path.clone(),
                    relative_path: relative_path.clone(),
                },
                lower_relative_path: relative_path.to_lowercase(),
            });
        }
    }
}

fn hash_str(value: &str) -> u64 {
    let mut hash = 1469598103934665603_u64;
    for byte in value.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

fn fuzzy_match_score(candidate: &str, query: &str) -> Option<i64> {
    if query.is_empty() {
        return Some(0);
    }

    let candidate_chars = candidate.chars().collect::<Vec<_>>();
    let mut score = 0_i64;
    let mut search_from = 0_usize;
    let mut previous_match = None;

    for query_char in query.chars() {
        let mut matched_index = None;
        for index in search_from..candidate_chars.len() {
            if candidate_chars[index] == query_char {
                matched_index = Some(index);
                break;
            }
        }
        let index = matched_index?;

        score += 10;
        if index == 0 {
            score += 20;
        } else if matches!(
            candidate_chars[index - 1],
            '/' | '\\' | '_' | '-' | ' ' | '.'
        ) {
            score += 15;
        }
        if let Some(previous) = previous_match {
            if index == previous + 1 {
                score += 12;
            } else {
                score -= (index - previous - 1) as i64;
            }
        } else {
            score -= index as i64;
        }

        previous_match = Some(index);
        search_from = index + 1;
    }

    if candidate.contains(query) {
        score += 30;
    }
    score += 20 - candidate_chars.len().min(20) as i64;
    Some(score)
}

fn read_text_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    Ok(decode_text_bytes(&bytes))
}

fn decode_text_bytes(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(&bytes[3..]).into_owned();
    }

    if bytes.starts_with(&[0xFF, 0xFE]) {
        let (text, _, _) = UTF_16LE.decode(&bytes[2..]);
        return text.into_owned();
    }

    if bytes.starts_with(&[0xFE, 0xFF]) {
        let (text, _, _) = UTF_16BE.decode(&bytes[2..]);
        return text.into_owned();
    }

    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
        return text;
    }

    let mut detector = EncodingDetector::new();
    detector.feed(bytes, true);
    let guessed = detector.guess(None, true);
    let (text, _, _) = guessed.decode(bytes);
    text.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::mpsc::TryRecvError,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    struct TempProject {
        root: PathBuf,
    }

    impl TempProject {
        fn new() -> Self {
            let unique = format!(
                "needle-search-test-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_nanos()
            );
            let root = std::env::temp_dir().join(unique);
            fs::create_dir_all(&root).expect("create temp project");
            Self { root }
        }

        fn write(&self, relative: &str, contents: &str) {
            let path = self.root.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create temp project parent");
            }
            fs::write(path, contents).expect("write temp project file");
        }
    }

    impl Drop for TempProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn project_index_hides_dotfiles_by_default() {
        let project = TempProject::new();
        project.write("src/main.rs", "fn main() {}\n");
        project.write(".needle/settings.json", "{}\n");

        let hidden_off = ProjectIndex::build(&project.root, false);
        let hidden_on = ProjectIndex::build(&project.root, true);

        assert_eq!(hidden_off.file_count(), 1);
        assert_eq!(hidden_on.file_count(), 2);
    }

    #[test]
    fn quick_open_prefers_stronger_matches() {
        let project = TempProject::new();
        project.write("src/app.rs", "fn main() {}\n");
        project.write("src/alpha_beta.rs", "fn alpha() {}\n");
        project.write("docs/readme.md", "hello\n");

        let index = ProjectIndex::build(&project.root, false);
        let matches = index.quick_open_matches("ab", 10);

        assert_eq!(
            matches.first().map(|item| item.relative_path.as_str()),
            Some("src/alpha_beta.rs")
        );
    }

    #[test]
    fn text_search_supports_case_sensitivity() {
        let project = TempProject::new();
        project.write("src/main.rs", "Needle\nneedle\n");

        let index = ProjectIndex::build(&project.root, false);
        let insensitive = index.search_text(ProjectTextSearchQuery::new("needle"));

        let mut sensitive_query = ProjectTextSearchQuery::new("needle");
        sensitive_query.case_sensitive = true;
        let sensitive = index.search_text(sensitive_query);

        assert_eq!(insensitive.len(), 2);
        assert_eq!(sensitive.len(), 1);
        assert_eq!(sensitive[0].line_number, 2);
    }

    #[test]
    fn async_search_emits_batches_and_completion() {
        let project = TempProject::new();
        project.write("src/main.rs", "alpha\nbeta\nalpha\n");

        let index = ProjectIndex::build(&project.root, false);
        let mut query = ProjectTextSearchQuery::new("alpha");
        query.batch_size = 1;
        let rx = index.spawn_text_search(query);

        let first = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("first event");
        match first {
            ProjectTextSearchEvent::Batch(batch) => {
                assert_eq!(batch.matches.len(), 1);
                assert_eq!(batch.total_matches, 1);
            }
            other => panic!("expected batch, got {other:?}"),
        }

        let second = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("second event");
        match second {
            ProjectTextSearchEvent::Batch(batch) => {
                assert_eq!(batch.matches.len(), 1);
                assert_eq!(batch.total_matches, 2);
            }
            other => panic!("expected second batch, got {other:?}"),
        }

        let final_event = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("complete event");
        match final_event {
            ProjectTextSearchEvent::Complete(summary) => {
                assert_eq!(summary.total_matches, 2);
                assert_eq!(summary.scanned_files, 1);
            }
            other => panic!("expected completion, got {other:?}"),
        }

        assert!(matches!(
            rx.try_recv(),
            Err(TryRecvError::Empty | TryRecvError::Disconnected)
        ));
    }
}
