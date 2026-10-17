//! In-memory git status cache plus the background refresher.
//!
//! Project cards read from this cache and never spawn git themselves, so listing
//! projects stays fast no matter how many repositories are tracked. Statuses are
//! refreshed off the UI thread and pushed to the frontend via `git-status-updated`.

use crate::db;
use crate::models::{GitStatusDto, GitStatusUpdate};
use crate::state::AppState;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

pub const STATUS_EVENT: &str = "git-status-updated";
const REFRESH_PARALLELISM: usize = 4;
const PERIODIC_REFRESH: Duration = Duration::from_secs(90);

static CACHE: OnceLock<Mutex<HashMap<PathBuf, GitStatusDto>>> = OnceLock::new();
static WINDOW_FOCUSED: AtomicBool = AtomicBool::new(true);
static REFRESH_REQUESTED: AtomicBool = AtomicBool::new(true);

fn cache() -> &'static Mutex<HashMap<PathBuf, GitStatusDto>> {
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Returns the last known status for a folder, or a `pending` placeholder.
pub fn cached(path: &Path) -> GitStatusDto {
    cache()
        .lock()
        .ok()
        .and_then(|m| m.get(path).cloned())
        .unwrap_or_else(|| {
            REFRESH_REQUESTED.store(true, Ordering::Relaxed);
            GitStatusDto {
                available: true,
                pending: true,
                ..GitStatusDto::default()
            }
        })
}

pub fn store(path: &Path, status: GitStatusDto) {
    if let Ok(mut m) = cache().lock() {
        m.insert(path.to_path_buf(), status);
    }
}

/// Runs `git status` for the folder now, updates the cache and returns the result.
pub fn refresh_now(git: &str, path: &Path) -> GitStatusDto {
    let mut status = crate::git::status(git, path);
    status.checked_at = Some(chrono::Utc::now().to_rfc3339());
    store(path, status.clone());
    status
}

/// Refreshes many folders with bounded parallelism, emitting an event per project.
pub fn refresh_many(
    app: &AppHandle,
    git: &str,
    targets: Vec<(String, PathBuf)>,
) -> Vec<GitStatusUpdate> {
    let queue = Mutex::new(targets.into_iter());
    let results = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..REFRESH_PARALLELISM {
            scope.spawn(|| loop {
                let next = queue.lock().ok().and_then(|mut q| q.next());
                let Some((project_id, path)) = next else {
                    break;
                };
                let git_status = refresh_now(git, &path);
                let update = GitStatusUpdate {
                    project_id,
                    git: git_status,
                };
                let _ = app.emit(STATUS_EVENT, &update);
                if let Ok(mut r) = results.lock() {
                    r.push(update);
                }
            });
        }
    });
    results.into_inner().unwrap_or_default()
}

/// Resolves the git-enabled software projects bound to a local folder on this device.
pub fn tracked_targets(
    conn: &rusqlite::Connection,
    device_id: &str,
    only: Option<&[String]>,
) -> Vec<(String, PathBuf)> {
    let Ok(mut stmt) = conn.prepare(
        "SELECT b.project_id, b.local_path FROM device_project_bindings b
         JOIN projects p ON p.id = b.project_id AND p.deleted_at IS NULL
         JOIN categories c ON c.id = p.category_id
         WHERE b.device_id = ?1 AND b.local_path IS NOT NULL AND b.local_path <> '' AND c.kind = 'software'",
    ) else {
        return Vec::new();
    };
    let rows = stmt
        .query_map([device_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
        .unwrap_or_default();
    rows.into_iter()
        .filter(|(id, _)| only.map(|o| o.contains(id)).unwrap_or(true))
        .map(|(id, p)| (id, PathBuf::from(p)))
        .collect()
}

/// Call from the window event handler so regaining focus triggers a refresh.
pub fn set_window_focused(focused: bool) {
    WINDOW_FOCUSED.store(focused, Ordering::Relaxed);
    if focused {
        REFRESH_REQUESTED.store(true, Ordering::Relaxed);
    }
}

/// Background loop: refresh on startup, on window focus, and periodically while focused.
pub fn spawn_refresher(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_run = Instant::now() - PERIODIC_REFRESH;
        loop {
            std::thread::sleep(Duration::from_millis(1500));
            let focused = WINDOW_FOCUSED.load(Ordering::Relaxed);
            let requested = REFRESH_REQUESTED.swap(false, Ordering::Relaxed);
            let periodic = focused && last_run.elapsed() >= PERIODIC_REFRESH;
            if !(requested || periodic) {
                continue;
            }
            last_run = Instant::now();
            let state = app.state::<AppState>();
            let (git, targets) = match state.db.with(|c| {
                Ok((
                    db::load_settings(c).git_path,
                    tracked_targets(c, &state.db.device_id, None),
                ))
            }) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if !targets.is_empty() {
                refresh_many(&app, &git, targets);
            }
        }
    });
}
