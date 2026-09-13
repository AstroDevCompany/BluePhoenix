use crate::error::{AppError, AppResult};
use crate::models::*;
use bluephoenix_ai::settings::{AiProviderKind, AiSettings};
use bluephoenix_ai::{MAX_MODEL_SLOTS, OPENROUTER_SECRET_KIND};
use bluephoenix_domain::context::{
    AiActivitySlice, AiExamSlice, AiLinkSlice, AiProjectBundle, AiTodoSlice, AiTopicSlice,
    AiVersionSlice, TodoFilter,
};
use bluephoenix_domain::ids::new_id;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::json;

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn setting(conn: &Connection, key: &str, default: &str) -> String {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        [key],
        |r| r.get(0),
    )
    .unwrap_or_else(|_| default.to_string())
}

fn set_setting(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO app_settings(key, value) VALUES(?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn load_ai_settings(conn: &Connection) -> AiSettings {
    let models: Vec<String> =
        serde_json::from_str(&setting(conn, "ai.models", "[]")).unwrap_or_default();
    let local_id = setting(conn, "ai.localModelId", "");
    AiSettings {
        enabled: setting(conn, "ai.enabled", "true") != "false",
        models,
        commit_follow_style: setting(conn, "ai.commitFollowStyle", "true") != "false",
        setup_dismissed: setting(conn, "ai.setupDismissed", "false") == "true",
        provider: AiProviderKind::parse(&setting(conn, "ai.provider", "openrouter")),
        local_model_id: if local_id.trim().is_empty() {
            None
        } else {
            Some(local_id)
        },
        local_ctx_len: setting(conn, "ai.localCtxLen", "8192")
            .parse()
            .unwrap_or(8192),
        local_gpu_offload: setting(conn, "ai.localGpuOffload", "true") != "false",
        local_idle_unload_minutes: setting(conn, "ai.localIdleUnloadMinutes", "0")
            .parse()
            .unwrap_or(0),
    }
    .normalize()
}

pub fn save_ai_settings(conn: &Connection, settings: &AiSettings) -> AppResult<AiSettings> {
    let settings = settings.clone().normalize();
    set_setting(
        conn,
        "ai.enabled",
        if settings.enabled { "true" } else { "false" },
    )?;
    set_setting(
        conn,
        "ai.models",
        &serde_json::to_string(&settings.models).unwrap_or_else(|_| "[]".into()),
    )?;
    set_setting(
        conn,
        "ai.commitFollowStyle",
        if settings.commit_follow_style {
            "true"
        } else {
            "false"
        },
    )?;
    set_setting(
        conn,
        "ai.setupDismissed",
        if settings.setup_dismissed {
            "true"
        } else {
            "false"
        },
    )?;
    set_setting(conn, "ai.provider", settings.provider.as_str())?;
    set_setting(
        conn,
        "ai.localModelId",
        settings.local_model_id.as_deref().unwrap_or(""),
    )?;
    set_setting(conn, "ai.localCtxLen", &settings.local_ctx_len.to_string())?;
    set_setting(
        conn,
        "ai.localGpuOffload",
        if settings.local_gpu_offload {
            "true"
        } else {
            "false"
        },
    )?;
    set_setting(
        conn,
        "ai.localIdleUnloadMinutes",
        &settings.local_idle_unload_minutes.to_string(),
    )?;
    Ok(settings)
}

fn map_local_model(r: &rusqlite::Row<'_>) -> rusqlite::Result<LocalModelDto> {
    Ok(LocalModelDto {
        id: r.get(0)?,
        name: r.get(1)?,
        path: r.get(2)?,
        managed: r.get::<_, i64>(3)? != 0,
        size_bytes: r.get(4)?,
        arch: r.get(5)?,
        n_ctx_train: r.get(6)?,
        added_at: r.get(7)?,
    })
}

pub fn list_local_models(conn: &Connection) -> AppResult<Vec<LocalModelDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, path, managed, size_bytes, arch, n_ctx_train, added_at
         FROM local_models ORDER BY added_at ASC",
    )?;
    let rows = stmt.query_map([], map_local_model)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_local_model(conn: &Connection, id: &str) -> AppResult<Option<LocalModelDto>> {
    conn.query_row(
        "SELECT id, name, path, managed, size_bytes, arch, n_ctx_train, added_at
         FROM local_models WHERE id=?1",
        [id],
        map_local_model,
    )
    .optional()
    .map_err(Into::into)
}

pub fn find_local_model_by_path(conn: &Connection, path: &str) -> AppResult<Option<LocalModelDto>> {
    conn.query_row(
        "SELECT id, name, path, managed, size_bytes, arch, n_ctx_train, added_at
         FROM local_models WHERE path=?1",
        [path],
        map_local_model,
    )
    .optional()
    .map_err(Into::into)
}

pub fn insert_local_model(
    conn: &Connection,
    name: &str,
    path: &str,
    managed: bool,
    size_bytes: Option<i64>,
    arch: Option<&str>,
    n_ctx_train: Option<i64>,
) -> AppResult<LocalModelDto> {
    let id = new_id();
    let added_at = now();
    conn.execute(
        "INSERT INTO local_models(id, name, path, managed, size_bytes, arch, n_ctx_train, added_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            id,
            name,
            path,
            if managed { 1 } else { 0 },
            size_bytes,
            arch,
            n_ctx_train,
            added_at
        ],
    )?;
    get_local_model(conn, &id)?.ok_or_else(|| AppError::msg("Local model missing after save"))
}

pub fn delete_local_model(conn: &Connection, id: &str) -> AppResult<Option<LocalModelDto>> {
    let row = get_local_model(conn, id)?;
    if row.is_some() {
        conn.execute("DELETE FROM local_models WHERE id=?1", [id])?;
    }
    Ok(row)
}

pub fn set_account_email(conn: &Connection, email: &str) -> AppResult<()> {
    set_setting(conn, "account.email", email)
}

#[allow(dead_code)]
pub fn account_email(conn: &Connection) -> Option<String> {
    let v = setting(conn, "account.email", "");
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

pub fn load_openrouter_envelope(
    conn: &Connection,
) -> Option<(String, String, String, serde_json::Value)> {
    conn.query_row(
        "SELECT id, ciphertext, nonce, wrap_params FROM encrypted_secrets WHERE kind=?1 AND deleted_at IS NULL LIMIT 1",
        [OPENROUTER_SECRET_KIND],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                serde_json::from_str(&r.get::<_, String>(3)?).unwrap_or(json!({})),
            ))
        },
    )
    .optional()
    .ok()
    .flatten()
}

pub fn ai_project_bundle(conn: &Connection, project_id: &str) -> AppResult<AiProjectBundle> {
    let snapshot = crate::db::project_context(conn, project_id).ok();
    let caps = snapshot
        .as_ref()
        .map(|s| s.capabilities)
        .unwrap_or_default();
    let mut bundle = AiProjectBundle {
        snapshot,
        time_total_seconds: 0,
        ..AiProjectBundle::default()
    };
    if let Some(snap) = &bundle.snapshot {
        bundle.time_total_seconds = snap.tracked_seconds;
    }
    if caps.todos {
        bundle.todos = crate::db::list_todos(
            conn,
            TodoFilter {
                project_id: Some(project_id.into()),
                category_id: None,
                status: None,
                kind_id: None,
                query: None,
            },
        )?
        .into_iter()
        .take(40)
        .map(|t| AiTodoSlice {
            id: t.id,
            title: t.title,
            description: t.description,
            priority: t.priority,
            status: t.status,
        })
        .collect();
    }
    if caps.activity {
        bundle.activity = crate::db::list_activity(conn, None, Some(project_id))?
            .into_iter()
            .take(12)
            .map(|a| AiActivitySlice {
                event_type: a.event_type,
                created_at: a.created_at,
                summary: a.payload.to_string(),
            })
            .collect();
    }
    if caps.links {
        bundle.links = crate::db::list_links(conn, project_id)?
            .into_iter()
            .take(16)
            .map(|l| AiLinkSlice {
                title: l.title,
                url: l.url,
            })
            .collect();
    }
    if caps.versions {
        bundle.versions = crate::db::list_versions(conn, project_id)?
            .into_iter()
            .take(8)
            .map(|v| AiVersionSlice {
                version: v.version,
                title: v.title,
                changelog: v.changelog,
            })
            .collect();
    }
    if caps.topics {
        bundle.topics = crate::db::list_topics(conn, project_id)?
            .into_iter()
            .take(30)
            .map(|t| AiTopicSlice {
                title: t.title,
                status: t.status,
            })
            .collect();
    }
    if caps.exams {
        bundle.exams = crate::db::list_exams(conn, Some(project_id), None)?
            .into_iter()
            .take(12)
            .map(|e| AiExamSlice {
                date: e.date,
                status: e.status,
                grade: e.grade.map(|g| g.to_string()),
            })
            .collect();
    }
    let mut stmt = conn.prepare(
        "SELECT t.name FROM tags t JOIN project_tags pt ON pt.tag_id=t.id WHERE pt.project_id=?1 ORDER BY t.name",
    )?;
    bundle.tags = stmt
        .query_map([project_id], |r| r.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(bundle)
}

pub fn compact_project_index(conn: &Connection) -> AppResult<Vec<serde_json::Value>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, c.kind, p.updated_at,
                (SELECT COUNT(*) FROM todos t WHERE t.project_id=p.id AND t.deleted_at IS NULL AND t.status IN ('open','in_progress')),
                (SELECT group_concat(t.name, ', ') FROM tags t JOIN project_tags pt ON pt.tag_id=t.id WHERE pt.project_id=p.id)
         FROM projects p JOIN categories c ON c.id=p.category_id
         WHERE p.deleted_at IS NULL AND p.archived=0
         ORDER BY p.updated_at DESC LIMIT 80",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(json!({
            "id": r.get::<_, String>(0)?,
            "name": r.get::<_, String>(1)?,
            "kind": r.get::<_, String>(2)?,
            "lastActivity": r.get::<_, String>(3)?,
            "openTodos": r.get::<_, i64>(4)?,
            "tags": r.get::<_, Option<String>>(5)?.unwrap_or_default(),
        }))
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn list_conversations(
    conn: &Connection,
    project_id: Option<&str>,
) -> AppResult<Vec<AiConversationDto>> {
    let mut sql = String::from(
        "SELECT id, project_id, title, created_at, updated_at FROM ai_conversations ORDER BY updated_at DESC LIMIT 40",
    );
    if project_id.is_some() {
        sql = "SELECT id, project_id, title, created_at, updated_at FROM ai_conversations WHERE project_id=?1 OR (project_id IS NULL AND ?1 IS NULL) ORDER BY updated_at DESC LIMIT 40".into();
    }
    let mut stmt = conn.prepare(&sql)?;
    let map = |r: &rusqlite::Row<'_>| {
        Ok(AiConversationDto {
            id: r.get(0)?,
            project_id: r.get(1)?,
            title: r.get(2)?,
            created_at: r.get(3)?,
            updated_at: r.get(4)?,
        })
    };
    let rows = if let Some(id) = project_id {
        stmt.query_map([id], map)?.filter_map(|r| r.ok()).collect()
    } else {
        stmt.query_map([], map)?.filter_map(|r| r.ok()).collect()
    };
    Ok(rows)
}

pub fn ensure_conversation(
    conn: &Connection,
    conversation_id: Option<&str>,
    project_id: Option<&str>,
) -> AppResult<String> {
    if let Some(id) = conversation_id {
        if !id.is_empty() {
            let exists: Option<String> = conn
                .query_row("SELECT id FROM ai_conversations WHERE id=?1", [id], |r| {
                    r.get(0)
                })
                .optional()?;
            if exists.is_some() {
                return Ok(id.to_string());
            }
        }
    }
    if let Some(pid) = project_id {
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM ai_conversations WHERE project_id=?1 ORDER BY updated_at DESC LIMIT 1",
                [pid],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = existing {
            return Ok(id);
        }
    }
    new_conversation(conn, project_id)
}

pub fn new_conversation(conn: &Connection, project_id: Option<&str>) -> AppResult<String> {
    let id = new_id();
    let ts = now();
    conn.execute(
        "INSERT INTO ai_conversations (id, project_id, title, created_at, updated_at) VALUES (?1,?2,'New chat',?3,?3)",
        params![id, project_id, ts],
    )?;
    Ok(id)
}

pub fn clear_conversation(conn: &Connection, conversation_id: &str) -> AppResult<()> {
    conn.execute(
        "DELETE FROM ai_messages WHERE conversation_id=?1",
        [conversation_id],
    )?;
    conn.execute(
        "UPDATE ai_conversations SET title='New chat', updated_at=?1 WHERE id=?2",
        params![now(), conversation_id],
    )?;
    Ok(())
}

pub fn list_messages(conn: &Connection, conversation_id: &str) -> AppResult<Vec<AiMessageDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, role, content, model, fallback_used, created_at FROM ai_messages WHERE conversation_id=?1 ORDER BY created_at",
    )?;
    let rows = stmt.query_map([conversation_id], |r| {
        Ok(AiMessageDto {
            id: r.get(0)?,
            conversation_id: r.get(1)?,
            role: r.get(2)?,
            content: r.get(3)?,
            model: r.get(4)?,
            fallback_used: r.get::<_, i64>(5)? != 0,
            created_at: r.get(6)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn insert_message(
    conn: &Connection,
    conversation_id: &str,
    role: &str,
    content: &str,
    model: Option<&str>,
    fallback_used: bool,
) -> AppResult<String> {
    let id = new_id();
    let ts = now();
    conn.execute(
        "INSERT INTO ai_messages (id, conversation_id, role, content, model, fallback_used, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![id, conversation_id, role, content, model, fallback_used as i64, ts],
    )?;
    conn.execute(
        "UPDATE ai_conversations SET updated_at=?1, title=CASE WHEN title='New chat' AND ?3='user' THEN substr(?4,1,48) ELSE title END WHERE id=?2",
        params![ts, conversation_id, role, content],
    )?;
    Ok(id)
}

pub fn update_todo_priority(conn: &Connection, id: &str, priority: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE todos SET priority=?1, updated_at=?2, revision=revision+1 WHERE id=?3",
        params![priority, now(), id],
    )?;
    Ok(())
}

pub fn upsert_document_record(
    conn: &Connection,
    file_id: &str,
    project_id: &str,
    status: &str,
    stage: &str,
    limitation: Option<&str>,
    parser: Option<&str>,
) -> AppResult<String> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM document_records WHERE file_id=?1",
            [file_id],
            |r| r.get(0),
        )
        .optional()?;
    let id = existing.unwrap_or_else(new_id);
    let ts = now();
    conn.execute(
        "INSERT INTO document_records (id, file_id, project_id, status, stage, limitation, parser, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
         ON CONFLICT(id) DO UPDATE SET status=excluded.status, stage=excluded.stage, limitation=excluded.limitation,
           parser=excluded.parser, updated_at=excluded.updated_at",
        params![id, file_id, project_id, status, stage, limitation, parser, ts],
    )?;
    Ok(id)
}

pub fn replace_chunks(
    conn: &Connection,
    record_id: &str,
    file_id: &str,
    project_id: &str,
    chunks: &[(Option<String>, String, Option<i64>)],
) -> AppResult<()> {
    conn.execute(
        "DELETE FROM document_chunks WHERE record_id=?1",
        [record_id],
    )?;
    conn.execute(
        "DELETE FROM document_chunks_fts WHERE record_id=?1",
        [record_id],
    )?;
    let ts = now();
    for (heading, text, page) in chunks {
        let id = new_id();
        conn.execute(
            "INSERT INTO document_chunks (id, record_id, file_id, project_id, heading, page, section, text, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?5,?7,?8)",
            params![id, record_id, file_id, project_id, heading, page, text, ts],
        )?;
        conn.execute(
            "INSERT INTO document_chunks_fts (chunk_id, record_id, project_id, heading, text) VALUES (?1,?2,?3,?4,?5)",
            params![id, record_id, project_id, heading, text],
        )?;
    }
    Ok(())
}

pub fn retrieve_chunks(
    conn: &Connection,
    project_id: &str,
    query: &str,
    limit: usize,
) -> AppResult<Vec<DocumentChunkDto>> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let mut hits = Vec::new();
    let fts = fts_query(q);
    if !fts.is_empty() {
        if let Ok(mut stmt) = conn.prepare(
            "SELECT c.id, c.file_id, f.display_name, c.heading, c.page, c.section, c.text
             FROM document_chunks_fts
             JOIN document_chunks c ON c.id = document_chunks_fts.chunk_id
             JOIN project_files f ON f.id = c.file_id
             WHERE document_chunks_fts.project_id=?1 AND document_chunks_fts MATCH ?2
             LIMIT ?3",
        ) {
            if let Ok(rows) = stmt.query_map(params![project_id, fts, limit as i64], map_chunk) {
                hits.extend(rows.filter_map(|r| r.ok()));
            }
        }
    }
    if hits.is_empty() {
        let like = format!("%{q}%");
        let mut stmt = conn.prepare(
            "SELECT c.id, c.file_id, f.display_name, c.heading, c.page, c.section, c.text
             FROM document_chunks c JOIN project_files f ON f.id=c.file_id
             WHERE c.project_id=?1 AND (c.text LIKE ?2 OR IFNULL(c.heading,'') LIKE ?2)
             LIMIT ?3",
        )?;
        hits = stmt
            .query_map(params![project_id, like, limit as i64], map_chunk)?
            .filter_map(|r| r.ok())
            .collect();
    }
    Ok(hits)
}

fn fts_query(q: &str) -> String {
    q.split_whitespace()
        .filter_map(|w| {
            let cleaned: String = w.chars().filter(|c| c.is_alphanumeric()).collect();
            if cleaned.len() < 2 {
                None
            } else {
                Some(cleaned)
            }
        })
        .collect::<Vec<_>>()
        .join(" OR ")
}

fn map_chunk(r: &rusqlite::Row<'_>) -> rusqlite::Result<DocumentChunkDto> {
    Ok(DocumentChunkDto {
        id: r.get(0)?,
        file_id: r.get(1)?,
        document_name: r.get(2)?,
        heading: r.get(3)?,
        page: r.get(4)?,
        section: r.get(5)?,
        text: r.get(6)?,
    })
}

pub fn list_document_records(
    conn: &Connection,
    project_id: &str,
) -> AppResult<Vec<DocumentRecordDto>> {
    let mut stmt = conn.prepare(
        "SELECT r.id, r.file_id, f.display_name, r.status, r.stage, r.limitation, r.parser, r.updated_at
         FROM document_records r JOIN project_files f ON f.id=r.file_id
         WHERE r.project_id=?1 ORDER BY r.updated_at DESC",
    )?;
    let rows = stmt.query_map([project_id], |r| {
        Ok(DocumentRecordDto {
            id: r.get(0)?,
            file_id: r.get(1)?,
            display_name: r.get(2)?,
            status: r.get(3)?,
            stage: r.get(4)?,
            limitation: r.get(5)?,
            parser: r.get(6)?,
            updated_at: r.get(7)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn file_binding(
    conn: &Connection,
    file_id: &str,
) -> AppResult<(String, String, Option<String>)> {
    conn.query_row(
        "SELECT f.project_id, f.display_name, b.absolute_path
         FROM project_files f
         LEFT JOIN device_file_bindings b ON b.file_id=f.id
         WHERE f.id=?1",
        [file_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .map_err(|_| AppError::msg("File not found"))
}

#[allow(dead_code)]
pub const MAX_SLOTS: usize = MAX_MODEL_SLOTS;
