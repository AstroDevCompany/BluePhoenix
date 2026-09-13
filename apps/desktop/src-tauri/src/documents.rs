use crate::ai_store;
use crate::error::AppResult;
use crate::state::AppState;
use bluephoenix_ai::parsers::{parse_document, semantic_chunks};
use serde_json::Value;
use std::path::Path;

pub fn index_document(state: &AppState, payload: &str) -> AppResult<()> {
    let value: Value = serde_json::from_str(payload).unwrap_or(Value::Null);
    let Some(file_id) = value.get("fileId").and_then(|v| v.as_str()) else {
        return Ok(());
    };
    let (project_id, _name, path) = state.db.with(|c| ai_store::file_binding(c, file_id))?;
    let Some(path) = path else {
        state.db.with(|c| {
            ai_store::upsert_document_record(
                c,
                file_id,
                &project_id,
                "failed",
                "Reading",
                Some("Original file path is missing on this device"),
                None,
            )
        })?;
        return Ok(());
    };
    set_stage(
        state,
        file_id,
        &project_id,
        "processing",
        "Reading",
        None,
        None,
    )?;
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => {
            set_stage(
                state,
                file_id,
                &project_id,
                "failed",
                "Reading",
                Some("Could not read the original file. It is still available on disk if the path exists."),
                None,
            )?;
            return Ok(());
        }
    };
    set_stage(
        state,
        file_id,
        &project_id,
        "processing",
        "Extracting",
        None,
        None,
    )?;
    let parsed = parse_document(Path::new(&path), &bytes);
    set_stage(
        state,
        file_id,
        &project_id,
        "processing",
        "Detecting sections",
        parsed.limitation.as_deref(),
        Some(parsed.parser.as_str()),
    )?;
    let raw_chunks = semantic_chunks(&parsed, 900, 200);
    let truncated = raw_chunks.len() == 200 && parsed.text.len() > 900 * 200;
    let chunks: Vec<(Option<String>, String, Option<i64>)> =
        raw_chunks.into_iter().map(|(h, t)| (h, t, None)).collect();
    set_stage(
        state,
        file_id,
        &project_id,
        "processing",
        "Building index",
        parsed.limitation.as_deref(),
        Some(parsed.parser.as_str()),
    )?;
    let record_id = state.db.with(|c| {
        ai_store::upsert_document_record(
            c,
            file_id,
            &project_id,
            "processing",
            "Building index",
            parsed.limitation.as_deref(),
            Some(parsed.parser.as_str()),
        )
    })?;
    state
        .db
        .with(|c| ai_store::replace_chunks(c, &record_id, file_id, &project_id, &chunks))?;
    let status = if parsed.limitation.is_some() && chunks.is_empty() {
        "failed"
    } else if parsed.limitation.is_some() || truncated {
        "partial"
    } else {
        "completed"
    };
    let stage = if status == "failed" {
        "Extracting"
    } else {
        "Ready"
    };
    set_stage(
        state,
        file_id,
        &project_id,
        status,
        stage,
        parsed.limitation.as_deref(),
        Some(parsed.parser.as_str()),
    )?;
    Ok(())
}

fn set_stage(
    state: &AppState,
    file_id: &str,
    project_id: &str,
    status: &str,
    stage: &str,
    limitation: Option<&str>,
    parser: Option<&str>,
) -> AppResult<()> {
    state.db.with(|c| {
        ai_store::upsert_document_record(c, file_id, project_id, status, stage, limitation, parser)
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use bluephoenix_ai::parsers::parse_document;
    use std::path::Path;

    #[test]
    fn unanswerable_unsupported_file_has_no_text() {
        let parsed = parse_document(Path::new("scan.xyz"), b"\x00\x01");
        assert!(parsed.text.is_empty());
        assert!(parsed.limitation.unwrap().contains("not indexed"));
    }
}
