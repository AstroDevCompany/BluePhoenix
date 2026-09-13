use crate::db;
use crate::error::AppResult;
use crate::native;
use crate::state::AppState;
use serde_json::Value;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Manager};

pub fn spawn(handle: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(1500));
        let Some(state) = handle.try_state::<AppState>() else {
            continue;
        };
        if let Err(err) = tick(&state) {
            tracing::warn!("background job: {err}");
        }
    });
}

fn tick(state: &AppState) -> AppResult<()> {
    let job = state.db.with(db::take_job)?;
    let Some((id, kind, payload)) = job else {
        return Ok(());
    };
    let error = match kind.as_str() {
        "index_files" => index_files(state, &payload).err().map(|e| e.to_string()),
        "index_documents" => crate::documents::index_document(state, &payload)
            .err()
            .map(|e| e.to_string()),
        "ai_generate" => None,
        other => Some(format!("Unknown job kind: {other}")),
    };
    state.db.with(|c| db::finish_job(c, &id, error))
}

fn index_files(state: &AppState, payload: &str) -> AppResult<()> {
    let value: Value = serde_json::from_str(payload).unwrap_or(Value::Null);
    let Some(project_id) = value.get("projectId").and_then(|v| v.as_str()) else {
        return Ok(());
    };
    let path = state.db.with(|c| {
        let (local, _) = db::project_binding(c, &state.db.device_id, project_id);
        Ok(local)
    })?;
    let Some(path) = path else {
        return Ok(());
    };
    let entries = native::list_dir_shallow(Path::new(&path))?;
    state
        .db
        .with(|c| db::cache_files(c, project_id, &entries))?;
    Ok(())
}
