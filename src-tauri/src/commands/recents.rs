use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::error::AppError;

/// Recent documents/places are capped at this many *unpinned* entries each,
/// so the list stays useful instead of growing forever; pinned entries are
/// always kept regardless of count.
const MAX_UNPINNED: usize = 15;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RecentEntry {
    pub path: String,
    pub last_used: u64,
    pub pinned: bool,
}

#[derive(Serialize, Deserialize, Default)]
struct RecentsFile {
    documents: Vec<RecentEntry>,
    places: Vec<RecentEntry>,
}

fn recents_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    let dir = app.path().app_data_dir().map_err(|e| AppError::RecentsFailed(e.to_string()))?;
    fs::create_dir_all(&dir).map_err(|e| AppError::RecentsFailed(e.to_string()))?;
    Ok(dir.join("recents.json"))
}

fn load(app: &AppHandle) -> Result<RecentsFile, AppError> {
    let path = recents_path(app)?;
    let Ok(bytes) = fs::read(&path) else {
        return Ok(RecentsFile::default());
    };
    // A corrupt recents file is not fatal -- untrusted/damaged local state,
    // same principle CLAUDE.md applies to PDFs, shouldn't take the window
    // down. Fall back to starting fresh.
    Ok(serde_json::from_slice(&bytes).unwrap_or_default())
}

fn save(app: &AppHandle, file: &RecentsFile) -> Result<(), AppError> {
    let path = recents_path(app)?;
    let bytes = serde_json::to_vec_pretty(file).map_err(|e| AppError::RecentsFailed(e.to_string()))?;
    fs::write(path, bytes).map_err(|e| AppError::RecentsFailed(e.to_string()))
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Moves (or inserts) `path` to the front of `entries` with a fresh
/// timestamp, preserving its pinned flag, then trims unpinned entries down
/// to `MAX_UNPINNED`.
fn touch(entries: &mut Vec<RecentEntry>, path: &str) {
    let pinned = entries.iter().find(|e| e.path == path).map(|e| e.pinned).unwrap_or(false);
    entries.retain(|e| e.path != path);
    entries.push(RecentEntry { path: path.to_string(), last_used: now(), pinned });
    sort(entries);

    let mut kept_unpinned = 0;
    entries.retain(|e| {
        if e.pinned {
            true
        } else {
            kept_unpinned += 1;
            kept_unpinned <= MAX_UNPINNED
        }
    });
}

/// Pinned entries first, then most-recently-used first within each group.
fn sort(entries: &mut [RecentEntry]) {
    entries.sort_by(|a, b| b.pinned.cmp(&a.pinned).then(b.last_used.cmp(&a.last_used)));
}

/// Records that `document_path` was just opened, in both the recent
/// documents list and (via its parent directory) the recent places list.
/// Called from `commands::doc::open_document` -- not a standalone command.
pub fn record_opened(app: &AppHandle, document_path: &Path) -> Result<(), AppError> {
    let mut file = load(app)?;

    touch(&mut file.documents, &document_path.to_string_lossy());

    if let Some(parent) = document_path.parent() {
        if !parent.as_os_str().is_empty() {
            touch(&mut file.places, &parent.to_string_lossy());
        }
    }

    save(app, &file)
}

#[tauri::command]
pub fn list_recent_documents(app: AppHandle) -> Result<Vec<RecentEntry>, AppError> {
    Ok(load(&app)?.documents)
}

#[tauri::command]
pub fn list_recent_places(app: AppHandle) -> Result<Vec<RecentEntry>, AppError> {
    Ok(load(&app)?.places)
}

#[tauri::command]
pub fn set_recent_document_pinned(app: AppHandle, path: String, pinned: bool) -> Result<(), AppError> {
    let mut file = load(&app)?;
    if let Some(entry) = file.documents.iter_mut().find(|e| e.path == path) {
        entry.pinned = pinned;
    }
    sort(&mut file.documents);
    save(&app, &file)
}

#[tauri::command]
pub fn set_recent_place_pinned(app: AppHandle, path: String, pinned: bool) -> Result<(), AppError> {
    let mut file = load(&app)?;
    if let Some(entry) = file.places.iter_mut().find(|e| e.path == path) {
        entry.pinned = pinned;
    }
    sort(&mut file.places);
    save(&app, &file)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, last_used: u64, pinned: bool) -> RecentEntry {
        RecentEntry { path: path.to_string(), last_used, pinned }
    }

    #[test]
    fn touch_moves_existing_entry_to_front_and_keeps_pinned_flag() {
        let mut entries =
            vec![entry("/a", 1, false), entry("/b", 2, true), entry("/c", 3, false)];

        touch(&mut entries, "/a");

        assert_eq!(entries.len(), 3);
        // /b stays first (pinned beats recency), /a should now be the most
        // recently used among the unpinned ones.
        assert_eq!(entries[0].path, "/b");
        assert!(entries[0].pinned);
        let a = entries.iter().find(|e| e.path == "/a").unwrap();
        assert!(!a.pinned, "touching an unpinned entry must not pin it");
    }

    #[test]
    fn touch_caps_unpinned_entries_but_keeps_all_pinned() {
        let mut entries: Vec<RecentEntry> =
            (0..MAX_UNPINNED).map(|i| entry(&format!("/old{i}"), i as u64, false)).collect();
        entries.push(entry("/pinned", 999, true));

        touch(&mut entries, "/new");

        let unpinned_count = entries.iter().filter(|e| !e.pinned).count();
        assert_eq!(unpinned_count, MAX_UNPINNED, "unpinned entries should be capped");
        assert!(entries.iter().any(|e| e.path == "/pinned"), "pinned entry must survive capping");
        assert!(entries.iter().any(|e| e.path == "/new"), "the just-touched entry must survive capping");
    }

    #[test]
    fn sort_puts_pinned_first_then_most_recent() {
        let mut entries =
            vec![entry("/old-pinned", 1, true), entry("/recent", 10, false), entry("/new-pinned", 5, true)];

        sort(&mut entries);

        assert_eq!(entries[0].path, "/new-pinned");
        assert_eq!(entries[1].path, "/old-pinned");
        assert_eq!(entries[2].path, "/recent");
    }
}
