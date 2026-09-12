use crate::error::{AppError, AppResult};
use crate::models::*;
use bluephoenix_domain::achievements::{self, AchievementSnapshot};
use bluephoenix_domain::attendance::{self, LessonView};
use bluephoenix_domain::capabilities::{CapabilityOverrides, CapabilitySet};
use bluephoenix_domain::commands_safety::is_dangerous;
use bluephoenix_domain::context::{todo_matches, ProjectContext, TodoFilter};
use bluephoenix_domain::exams::{self, ExamAttemptView};
use bluephoenix_domain::gpa::{self, CourseGradeInput, GpaConfig};
use bluephoenix_domain::ids::{
    new_id, CategoryKind, ExamStatus, FITNESS_CATEGORY_ID, PERSONAL_CATEGORY_ID,
    PHOTOGRAPHY_CATEGORY_ID, SOFTWARE_CATEGORY_ID, TagKind, UNIVERSITY_CATEGORY_ID,
};
use bluephoenix_domain::semver_check::is_at_least_one;
use bluephoenix_domain::terminology::{self, default_todo_kinds};
use bluephoenix_domain::time::{elapsed_seconds, TimeRange};
use bluephoenix_domain::validation::{optional_url, required_name, validate_cfu};
use bluephoenix_domain::xp::{DefaultXpFormula, XpFormula};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const SCHEMA: &str = include_str!("../migrations/001_init.sql");

pub struct Db {
    conn: Mutex<Connection>,
    pub device_id: String,
    #[allow(dead_code)]
    pub path: PathBuf,
}

impl Db {
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;")?;
        conn.execute_batch(SCHEMA)?;
        let mut db = Self {
            conn: Mutex::new(conn),
            device_id: String::new(),
            path: path.to_path_buf(),
        };
        db.device_id = db.ensure_device()?;
        db.seed()?;
        Ok(db)
    }

    fn lock(&self) -> AppResult<std::sync::MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| AppError::msg("Database locked"))
    }

    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let conn = self.lock()?;
        f(&conn)
    }

    fn ensure_device(&self) -> AppResult<String> {
        let conn = self.lock()?;
        if let Some(id) = conn
            .query_row(
                "SELECT id FROM devices LIMIT 1",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            return Ok(id);
        }
        let id = new_id();
        let host = hostname::get()
            .ok()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".into());
        let os = std::env::consts::OS.to_string();
        conn.execute(
            "INSERT INTO devices (id, hostname, os, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, host, os, now()],
        )?;
        Ok(id)
    }

    fn seed(&self) -> AppResult<()> {
        let conn = self.lock()?;
        seed_settings(&conn)?;
        seed_user(&conn)?;
        seed_categories(&conn)?;
        seed_tags(&conn)?;
        Ok(())
    }
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn parse_dt(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
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
        "INSERT INTO app_settings(key, value) VALUES(?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn seed_settings(conn: &Connection) -> AppResult<()> {
    let defaults = [
        ("appearance.theme", "dark"),
        ("appearance.accent", "cyan"),
        ("appearance.reducedMotion", "false"),
        ("workspace.defaultCategoryId", SOFTWARE_CATEGORY_ID),
        ("updates.automatic", "true"),
        ("updates.channel", "stable"),
        (
            "updates.rawUrl",
            "https://raw.githubusercontent.com/bluephoenix-app/bluephoenix/main/version.json",
        ),
        ("developer.gitPath", ""),
        ("developer.vscodePath", ""),
        ("developer.terminal", "default"),
        ("developer.shell", ""),
        ("account.apiBase", "http://127.0.0.1:8787"),
        ("grading.includeFailed", "false"),
        ("onboarding.complete", "false"),
        ("ai.reserved", "{}"),
        ("ai.enabled", "true"),
        ("ai.models", "[\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\"]"),
        ("ai.commitFollowStyle", "true"),
        ("ai.setupDismissed", "false"),
    ];
    for (key, value) in defaults {
        conn.execute(
            "INSERT OR IGNORE INTO app_settings(key, value) VALUES(?1, ?2)",
            params![key, value],
        )?;
    }
    Ok(())
}

fn seed_user(conn: &Connection) -> AppResult<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM users WHERE deleted_at IS NULL", [], |r| r.get(0))?;
    if count == 0 {
        let id = new_id();
        let ts = now();
        conn.execute(
            "INSERT INTO users (id, display_name, created_at, updated_at, revision) VALUES (?1, ?2, ?3, ?3, 1)",
            params![id, "Local", ts],
        )?;
    }
    Ok(())
}

fn seed_categories(conn: &Connection) -> AppResult<()> {
    let rows: i64 = conn.query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))?;
    if rows > 0 {
        return Ok(());
    }
    let ts = now();
    let cats = [
        (SOFTWARE_CATEGORY_ID, "software", "software", "Software", "code", "#3dd6c6", 1, 0, 1),
        (UNIVERSITY_CATEGORY_ID, "university", "university", "University", "graduation-cap", "#5ec8e8", 1, 1, 1),
        (FITNESS_CATEGORY_ID, "fitness", "generic", "Fitness", "dumbbell", "#4ade80", 0, 2, 1),
        (PHOTOGRAPHY_CATEGORY_ID, "photography", "generic", "Photography", "camera", "#67e8f9", 0, 3, 1),
        (PERSONAL_CATEGORY_ID, "personal", "generic", "Personal", "sparkles", "#a5b4fc", 1, 4, 1),
    ];
    for (id, slug, kind, name, icon, accent, enabled, order, system) in cats {
        conn.execute(
            "INSERT INTO categories (id, slug, kind, name, icon, accent, enabled, sort_order, is_system, created_at, updated_at, revision)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10,1)",
            params![id, slug, kind, name, icon, accent, enabled, order, system, ts],
        )?;
        let kind_enum = CategoryKind::parse(kind).unwrap_or(CategoryKind::Generic);
        for (i, (slug_k, name_k)) in default_todo_kinds(kind_enum).into_iter().enumerate() {
            conn.execute(
                "INSERT INTO todo_kinds (id, category_id, slug, name, is_system, sort_order, created_at, updated_at, revision)
                 VALUES (?1,?2,?3,?4,1,?5,?6,?6,1)",
                params![new_id(), id, slug_k, name_k, i as i64, ts],
            )?;
        }
    }
    Ok(())
}

fn seed_tags(conn: &Connection) -> AppResult<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM tags", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }
    let ts = now();
    for name in bluephoenix_domain::terminology::LANGUAGES {
        conn.execute(
            "INSERT INTO tags (id, name, kind, created_at, updated_at, revision) VALUES (?1,?2,'language',?3,?3,1)",
            params![new_id(), name, ts],
        )?;
    }
    for name in bluephoenix_domain::terminology::FRAMEWORKS {
        conn.execute(
            "INSERT INTO tags (id, name, kind, created_at, updated_at, revision) VALUES (?1,?2,'framework',?3,?3,1)",
            params![new_id(), name, ts],
        )?;
    }
    Ok(())
}

fn queue_outbox(conn: &Connection, table: &str, row_id: &str, op: &str, payload: serde_json::Value) -> AppResult<()> {
    conn.execute(
        "INSERT INTO sync_outbox (id, table_name, row_id, op, payload, created_at) VALUES (?1,?2,?3,?4,?5,?6)",
        params![new_id(), table, row_id, op, payload.to_string(), now()],
    )?;
    Ok(())
}

fn index_search(conn: &Connection, entity_type: &str, entity_id: &str, title: &str, body: &str) -> AppResult<()> {
    conn.execute("DELETE FROM search_index WHERE entity_id = ?1", [entity_id])?;
    conn.execute(
        "INSERT INTO search_index (entity_type, entity_id, title, body) VALUES (?1,?2,?3,?4)",
        params![entity_type, entity_id, title, body],
    )?;
    Ok(())
}

fn activity(
    conn: &Connection,
    project_id: Option<&str>,
    category_id: Option<&str>,
    event_type: &str,
    payload: serde_json::Value,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO activity_events (id, project_id, category_id, event_type, payload, created_at, updated_at, revision)
         VALUES (?1,?2,?3,?4,?5,?6,?6,1)",
        params![new_id(), project_id, category_id, event_type, payload.to_string(), now()],
    )?;
    Ok(())
}

pub fn load_settings(conn: &Connection) -> AppSettingsDto {
    AppSettingsDto {
        theme: setting(conn, "appearance.theme", "dark"),
        accent: setting(conn, "appearance.accent", "cyan"),
        reduced_motion: setting(conn, "appearance.reducedMotion", "false") == "true",
        default_workspace: Some(setting(conn, "workspace.defaultCategoryId", SOFTWARE_CATEGORY_ID)),
        automatic_updates: setting(conn, "updates.automatic", "true") != "false",
        update_channel: setting(conn, "updates.channel", "stable"),
        raw_version_url: setting(
            conn,
            "updates.rawUrl",
            "https://raw.githubusercontent.com/bluephoenix-app/bluephoenix/main/version.json",
        ),
        git_path: setting(conn, "developer.gitPath", ""),
        vscode_path: setting(conn, "developer.vscodePath", ""),
        terminal: setting(conn, "developer.terminal", "default"),
        shell: setting(conn, "developer.shell", ""),
        api_base: setting(conn, "account.apiBase", "http://127.0.0.1:8787"),
        include_failed_grades: setting(conn, "grading.includeFailed", "false") == "true",
        onboarding_complete: setting(conn, "onboarding.complete", "false") == "true",
    }
}

pub fn save_settings(conn: &Connection, settings: &AppSettingsDto) -> AppResult<()> {
    set_setting(conn, "appearance.theme", &settings.theme)?;
    set_setting(conn, "appearance.accent", &settings.accent)?;
    set_setting(
        conn,
        "appearance.reducedMotion",
        if settings.reduced_motion { "true" } else { "false" },
    )?;
    if let Some(id) = &settings.default_workspace {
        set_setting(conn, "workspace.defaultCategoryId", id)?;
    }
    set_setting(
        conn,
        "updates.automatic",
        if settings.automatic_updates { "true" } else { "false" },
    )?;
    set_setting(conn, "updates.channel", &settings.update_channel)?;
    set_setting(conn, "updates.rawUrl", &settings.raw_version_url)?;
    set_setting(conn, "developer.gitPath", &settings.git_path)?;
    set_setting(conn, "developer.vscodePath", &settings.vscode_path)?;
    set_setting(conn, "developer.terminal", &settings.terminal)?;
    set_setting(conn, "developer.shell", &settings.shell)?;
    set_setting(conn, "account.apiBase", &settings.api_base)?;
    set_setting(
        conn,
        "grading.includeFailed",
        if settings.include_failed_grades { "true" } else { "false" },
    )?;
    Ok(())
}

fn kind_of(conn: &Connection, category_id: &str) -> AppResult<CategoryKind> {
    let kind: String = conn.query_row(
        "SELECT kind FROM categories WHERE id = ?1",
        [category_id],
        |r| r.get(0),
    )?;
    CategoryKind::parse(&kind).ok_or_else(|| AppError::msg("Unknown category kind"))
}

pub fn project_kind(conn: &Connection, project_id: &str) -> AppResult<CategoryKind> {
    let kind: String = conn.query_row(
        "SELECT c.kind FROM projects p JOIN categories c ON c.id = p.category_id WHERE p.id=?1",
        [project_id],
        |r| r.get(0),
    )?;
    CategoryKind::parse(&kind).ok_or_else(|| AppError::msg("Unknown category kind"))
}

fn caps_for(conn: &Connection, category_id: &str) -> AppResult<CapabilitySet> {
    let (kind, overrides, slug): (String, Option<String>, String) = conn.query_row(
        "SELECT kind, capability_overrides, slug FROM categories WHERE id = ?1",
        [category_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let kind = CategoryKind::parse(&kind).unwrap_or(CategoryKind::Generic);
    let parsed: CapabilityOverrides = overrides
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let _ = slug;
    Ok(CapabilitySet::from_kind(kind).apply_overrides(kind, &parsed))
}

fn category_dto(conn: &Connection, id: &str) -> AppResult<CategoryDto> {
    let row: (String, String, String, String, String, i64, i64, i64, Option<String>) = conn.query_row(
        "SELECT slug, kind, name, icon, accent, enabled, sort_order, is_system, capability_overrides FROM categories WHERE id=?1 AND deleted_at IS NULL",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?)),
    )?;
    let kind = CategoryKind::parse(&row.1).unwrap_or(CategoryKind::Generic);
    let overrides: CapabilityOverrides = row
        .8
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let caps = CapabilitySet::from_kind(kind).apply_overrides(kind, &overrides);
    let terms = terminology::Terminology::for_slug(kind, &row.0);
    let nav = terminology::nav_for_kind(kind);
    Ok(CategoryDto {
        id: id.to_string(),
        slug: row.0,
        kind: row.1,
        name: row.2,
        icon: row.3,
        accent: row.4,
        enabled: row.5 != 0,
        sort_order: row.6,
        is_system: row.7 != 0,
        capabilities: serde_json::to_value(caps).unwrap_or(json!({})),
        terminology: serde_json::to_value(terms).unwrap_or(json!({})),
        nav: serde_json::to_value(nav).unwrap_or(json!([])),
    })
}

pub fn list_categories(conn: &Connection, include_disabled: bool) -> AppResult<Vec<CategoryDto>> {
    let sql = if include_disabled {
        "SELECT id FROM categories WHERE deleted_at IS NULL ORDER BY sort_order"
    } else {
        "SELECT id FROM categories WHERE deleted_at IS NULL AND enabled = 1 ORDER BY sort_order"
    };
    let mut stmt = conn.prepare(sql)?;
    let ids: Vec<String> = stmt.query_map([], |r| r.get(0))?.filter_map(|r| r.ok()).collect();
    ids.into_iter().map(|id| category_dto(conn, &id)).collect()
}

pub fn set_category_enabled(conn: &Connection, id: &str, enabled: bool) -> AppResult<CategoryDto> {
    conn.execute(
        "UPDATE categories SET enabled=?1, updated_at=?2, revision=revision+1 WHERE id=?3",
        params![enabled as i64, now(), id],
    )?;
    if !enabled {
        let current = setting(conn, "workspace.defaultCategoryId", SOFTWARE_CATEGORY_ID);
        if current == id {
            let fallback: Option<String> = conn
                .query_row(
                    "SELECT id FROM categories WHERE enabled=1 AND deleted_at IS NULL ORDER BY sort_order LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(fb) = fallback {
                set_setting(conn, "workspace.defaultCategoryId", &fb)?;
            }
        }
    }
    category_dto(conn, id)
}

pub fn create_custom_category(
    conn: &Connection,
    name: &str,
    icon: &str,
    accent: &str,
) -> AppResult<CategoryDto> {
    let name = required_name(name, "name")?;
    let id = new_id();
    let ts = now();
    let slug = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>();
    let order: i64 = conn.query_row("SELECT COALESCE(MAX(sort_order),0)+1 FROM categories", [], |r| r.get(0))?;
    conn.execute(
        "INSERT INTO categories (id, slug, kind, name, icon, accent, enabled, sort_order, is_system, created_at, updated_at, revision)
         VALUES (?1,?2,'generic',?3,?4,?5,1,?6,0,?7,?7,1)",
        params![id, slug, name, icon, accent, order, ts],
    )?;
    for (i, (slug_k, name_k)) in default_todo_kinds(CategoryKind::Generic).into_iter().enumerate() {
        conn.execute(
            "INSERT INTO todo_kinds (id, category_id, slug, name, is_system, sort_order, created_at, updated_at, revision)
             VALUES (?1,?2,?3,?4,1,?5,?6,?6,1)",
            params![new_id(), id, slug_k, name_k, i as i64, ts],
        )?;
    }
    let tx_payload = json!({"id": id, "name": name, "kind": "generic"});
    conn.execute(
        "INSERT INTO sync_outbox (id, table_name, row_id, op, payload, created_at) VALUES (?1,'categories',?2,'upsert',?3,?4)",
        params![new_id(), id, tx_payload.to_string(), ts],
    )?;
    category_dto(conn, &id)
}

pub fn list_tags(conn: &Connection) -> AppResult<Vec<TagDto>> {
    let mut stmt = conn.prepare("SELECT id, name, kind FROM tags WHERE deleted_at IS NULL ORDER BY kind, name")?;
    let rows = stmt.query_map([], |r| {
        Ok(TagDto {
            id: r.get(0)?,
            name: r.get(1)?,
            kind: r.get(2)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn create_custom_tag(conn: &Connection, name: &str, kind: &str) -> AppResult<TagDto> {
    let name = required_name(name, "tag")?;
    let kind = TagKind::parse(kind).ok_or_else(|| AppError::msg("Tag kind must be language, framework, or custom"))?;
    let kind_str = kind.as_str();
    if let Some((id, existing_name, existing_kind, deleted_at)) = conn
        .query_row(
            "SELECT id, name, kind, deleted_at FROM tags WHERE name = ?1 COLLATE NOCASE AND kind = ?2",
            params![name, kind_str],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()?
    {
        if deleted_at.is_some() {
            conn.execute(
                "UPDATE tags SET deleted_at=NULL, name=?1, updated_at=?2, revision=revision+1 WHERE id=?3",
                params![name, now(), id],
            )?;
        }
        return Ok(TagDto {
            id,
            name: existing_name,
            kind: existing_kind,
        });
    }
    let id = new_id();
    let ts = now();
    conn.execute(
        "INSERT INTO tags (id, name, kind, created_at, updated_at, revision) VALUES (?1,?2,?3,?4,?4,1)",
        params![id, name, kind_str, ts],
    )?;
    Ok(TagDto {
        id,
        name,
        kind: kind_str.into(),
    })
}

pub fn project_binding(
    conn: &Connection,
    device_id: &str,
    project_id: &str,
) -> (Option<String>, Option<String>) {
    conn.query_row(
        "SELECT local_path, primary_file_path FROM device_project_bindings WHERE device_id=?1 AND project_id=?2",
        params![device_id, project_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()
    .ok()
    .flatten()
    .unwrap_or((None, None))
}

fn set_binding(
    conn: &Connection,
    device_id: &str,
    project_id: &str,
    local_path: Option<&str>,
    primary_file: Option<&str>,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO device_project_bindings (device_id, project_id, local_path, primary_file_path, updated_at)
         VALUES (?1,?2,?3,?4,?5)
         ON CONFLICT(device_id, project_id) DO UPDATE SET
           local_path=excluded.local_path,
           primary_file_path=excluded.primary_file_path,
           updated_at=excluded.updated_at",
        params![device_id, project_id, local_path, primary_file, now()],
    )?;
    Ok(())
}

fn tags_for(conn: &Connection, project_id: &str) -> Vec<TagDto> {
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.name, t.kind FROM tags t
             JOIN project_tags pt ON pt.tag_id = t.id
             WHERE pt.project_id=?1 AND pt.deleted_at IS NULL AND t.deleted_at IS NULL
             ORDER BY t.kind, t.name",
        )
        .ok();
    let Some(mut stmt) = stmt.take() else {
        return vec![];
    };
    stmt.query_map([project_id], |r| {
        Ok(TagDto {
            id: r.get(0)?,
            name: r.get(1)?,
            kind: r.get(2)?,
        })
    })
    .ok()
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

fn unlocks_for(conn: &Connection, project_id: Option<&str>) -> Vec<UnlockDto> {
    let sql = if project_id.is_some() {
        "SELECT id, achievement_id, earned_at, project_id FROM achievement_unlocks
         WHERE deleted_at IS NULL AND project_id = ?1 ORDER BY earned_at"
    } else {
        "SELECT id, achievement_id, earned_at, project_id FROM achievement_unlocks
         WHERE deleted_at IS NULL AND project_id IS NULL ORDER BY earned_at"
    };
    let Ok(mut stmt) = conn.prepare(sql) else {
        return vec![];
    };
    let mut rows: Vec<(String, String, String, Option<String>)> = Vec::new();
    if let Some(pid) = project_id {
        if let Ok(iter) = stmt.query_map([pid], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        }) {
            rows.extend(iter.flatten());
        }
    } else if let Ok(iter) = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, Option<String>>(3)?,
        ))
    }) {
        rows.extend(iter.flatten());
    }
    let mut out = Vec::new();
    for row in rows {
        if let Some(def) = achievements::definition(&row.1) {
            out.push(UnlockDto {
                id: row.0,
                achievement_id: row.1,
                name: def.name.to_string(),
                description: def.description.to_string(),
                icon: def.icon.to_string(),
                rarity: def.rarity.as_str().to_string(),
                earned_at: row.2,
                project_id: row.3,
            });
        }
    }
    out.sort_by_key(|u| {
        achievements::definition(&u.achievement_id)
            .map(|d| d.display_order)
            .unwrap_or(9999)
    });
    out
}

fn pinned_command(conn: &Connection, project_id: &str) -> Option<CommandDto> {
    conn.query_row(
        "SELECT id, name, command, description, working_directory, pinned, favorite, sort_order
         FROM commands WHERE project_id=?1 AND deleted_at IS NULL AND pinned=1 ORDER BY sort_order LIMIT 1",
        [project_id],
        |r| {
            let command: String = r.get(2)?;
            Ok(CommandDto {
                id: r.get(0)?,
                project_id: project_id.into(),
                name: r.get(1)?,
                command: command.clone(),
                description: r.get(3)?,
                working_directory: r.get(4)?,
                pinned: r.get::<_, i64>(5)? != 0,
                favorite: r.get::<_, i64>(6)? != 0,
                sort_order: r.get(7)?,
                dangerous: is_dangerous(&command),
            })
        },
    )
    .optional()
    .ok()
    .flatten()
}

fn attendance_for(conn: &Connection, project_id: &str) -> AttendanceDto {
    let mut stmt = conn
        .prepare("SELECT attended, duration_seconds FROM course_lessons WHERE project_id=?1 AND deleted_at IS NULL")
        .ok();
    let lessons: Vec<LessonView> = stmt
        .as_mut()
        .and_then(|s| {
            s.query_map([project_id], |r| {
                Ok(LessonView {
                    attended: r.get::<_, i64>(0)? != 0,
                    duration_seconds: r.get(1)?,
                })
            })
            .ok()
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();
    let stats = attendance::attendance_stats(&lessons);
    AttendanceDto {
        lessons: stats.lessons as i64,
        attended: stats.attended as i64,
        missed: stats.missed as i64,
        percent: stats.attendance_percent,
        seconds: stats.attended_seconds,
    }
}

fn exam_average(conn: &Connection, project_id: &str) -> Option<f64> {
    let mut stmt = conn
        .prepare("SELECT status, grade FROM exam_attempts WHERE project_id=?1 AND deleted_at IS NULL")
        .ok()?;
    let attempts: Vec<ExamAttemptView> = stmt
        .query_map([project_id], |r| {
            let status: String = r.get(0)?;
            Ok(ExamAttemptView {
                status: ExamStatus::parse(&status).unwrap_or(ExamStatus::Attempted),
                grade: r.get(1)?,
            })
        })
        .ok()?
        .filter_map(|r| r.ok())
        .collect();
    exams::summarize_exams(&attempts, None).average_grade
}

fn last_activity(conn: &Connection, project_id: &str) -> Option<String> {
    conn.query_row(
        "SELECT created_at FROM activity_events WHERE project_id=?1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT 1",
        [project_id],
        |r| r.get(0),
    )
    .optional()
    .ok()
    .flatten()
}

fn open_todo_count(conn: &Connection, project_id: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM todos WHERE project_id=?1 AND deleted_at IS NULL AND status != 'completed' AND status != 'cancelled'",
        [project_id],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

pub fn project_card(
    conn: &Connection,
    project_id: &str,
    device_id: &str,
    git_path: &str,
) -> AppResult<ProjectCardDto> {
    let (category_id, name, description, status, favorite, archived, icon, accent, tracked, xp, created_at): (
        String,
        String,
        String,
        String,
        i64,
        i64,
        Option<String>,
        Option<String>,
        i64,
        i64,
        String,
    ) = conn.query_row(
        "SELECT category_id, name, description, status, favorite, archived, icon, accent, total_tracked_seconds, xp, created_at
         FROM projects WHERE id=?1 AND deleted_at IS NULL",
        [project_id],
        |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
                r.get(8)?,
                r.get(9)?,
                r.get(10)?,
            ))
        },
    )?;
    let kind = kind_of(conn, &category_id)?;
    let level = DefaultXpFormula.level_for_xp(xp);
    let (github, website, version) = if kind == CategoryKind::Software {
        conn.query_row(
            "SELECT github_url, website_url, current_version FROM software_profiles WHERE project_id=?1",
            [project_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .unwrap_or((None, None, None))
    } else {
        (None, None, None)
    };
    let (cfu, final_grade, honors) = if kind == CategoryKind::University {
        conn.query_row(
            "SELECT cfu, final_grade, honors FROM university_profiles WHERE project_id=?1",
            [project_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0)),
        )
        .optional()?
        .unwrap_or((None, None, false))
    } else {
        (None, None, false)
    };
    let (local_path, primary_file_path) = project_binding(conn, device_id, project_id);
    let git = if kind == CategoryKind::Software {
        local_path
            .as_ref()
            .map(|p| crate::git::status(git_path, Path::new(p)))
    } else {
        None
    };
    let _ = created_at;
    Ok(ProjectCardDto {
        id: project_id.to_string(),
        category_id,
        kind: kind.as_str().to_string(),
        name,
        description,
        status,
        favorite: favorite != 0,
        archived: archived != 0,
        icon,
        accent,
        total_tracked_seconds: tracked,
        xp,
        level: level.level,
        level_ratio: level.ratio(),
        github_url: github,
        website_url: website,
        current_version: version,
        cfu,
        final_grade,
        average_grade: if kind == CategoryKind::University {
            exam_average(conn, project_id)
        } else {
            None
        },
        honors,
        open_todos: open_todo_count(conn, project_id),
        pinned_command: if kind == CategoryKind::Software {
            pinned_command(conn, project_id)
        } else {
            None
        },
        tags: tags_for(conn, project_id),
        achievements: unlocks_for(conn, Some(project_id)),
        local_path,
        primary_file_path,
        attendance: if kind == CategoryKind::University {
            Some(attendance_for(conn, project_id))
        } else {
            None
        },
        last_activity: last_activity(conn, project_id),
        git,
    })
}

pub fn list_projects(
    conn: &Connection,
    category_id: &str,
    device_id: &str,
    git_path: &str,
    include_archived: bool,
) -> AppResult<Vec<ProjectCardDto>> {
    let sql = if include_archived {
        "SELECT id FROM projects WHERE category_id=?1 AND deleted_at IS NULL ORDER BY favorite DESC, updated_at DESC"
    } else {
        "SELECT id FROM projects WHERE category_id=?1 AND deleted_at IS NULL AND archived=0 ORDER BY favorite DESC, updated_at DESC"
    };
    let mut stmt = conn.prepare(sql)?;
    let ids: Vec<String> = stmt
        .query_map([category_id], |r| r.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    ids.into_iter()
        .map(|id| project_card(conn, &id, device_id, git_path))
        .collect()
}

pub fn create_project(
    conn: &Connection,
    device_id: &str,
    input: CreateProjectInput,
) -> AppResult<ProjectCardDto> {
    let name = required_name(&input.name, "name")?;
    let kind = kind_of(conn, &input.category_id)?;
    let caps = caps_for(conn, &input.category_id)?;
    let id = new_id();
    let ts = now();
    conn.execute(
        "INSERT INTO projects (id, category_id, name, description, status, created_at, started_at, favorite, archived, total_tracked_seconds, xp, updated_at, revision)
         VALUES (?1,?2,?3,?4,'active',?5,?5,0,0,0,0,?5,1)",
        params![id, input.category_id, name, input.description.clone().unwrap_or_default(), ts],
    )?;
    match kind {
        CategoryKind::Software => {
            let github = optional_url(input.github_url.as_deref())?;
            let website = optional_url(input.website_url.as_deref())?;
            conn.execute(
                "INSERT INTO software_profiles (project_id, github_url, website_url, updated_at, revision) VALUES (?1,?2,?3,?4,1)",
                params![id, github, website, ts],
            )?;
        }
        CategoryKind::University => {
            conn.execute(
                "INSERT INTO university_profiles (project_id, university, professor, academic_year, semester, cfu, honors, grading_max, updated_at, revision)
                 VALUES (?1,?2,?3,?4,?5,?6,0,30,?7,1)",
                params![
                    id,
                    input.university,
                    input.professor,
                    input.academic_year,
                    input.semester,
                    validate_cfu(input.cfu)?,
                    ts
                ],
            )?;
        }
        CategoryKind::Generic => {}
    }
    if input.local_path.is_some() || input.primary_file_path.is_some() {
        set_binding(
            conn,
            device_id,
            &id,
            input.local_path.as_deref(),
            input.primary_file_path.as_deref(),
        )?;
    }
    if caps.languages || caps.frameworks {
        let mut tag_ids = Vec::new();
        if let Some(ids) = input.language_ids {
            tag_ids.extend(ids);
        }
        if let Some(ids) = input.framework_ids {
            tag_ids.extend(ids);
        }
        for tag_id in tag_ids {
            conn.execute(
                "INSERT OR IGNORE INTO project_tags (project_id, tag_id, created_at, updated_at, revision) VALUES (?1,?2,?3,?3,1)",
                params![id, tag_id, ts],
            )?;
        }
    }
    index_search(conn, "project", &id, &name, input.description.as_deref().unwrap_or(""))?;
    activity(
        conn,
        Some(&id),
        Some(&input.category_id),
        "project.created",
        json!({"name": name}),
    )?;
    conn.execute(
        "INSERT INTO sync_outbox (id, table_name, row_id, op, payload, created_at) VALUES (?1,'projects',?2,'upsert',?3,?4)",
        params![new_id(), id, json!({"id": id, "name": name}).to_string(), ts],
    )?;
    evaluate_project(conn, &id)?;
    evaluate_user(conn)?;
    let settings = load_settings(conn);
    project_card(conn, &id, device_id, &settings.git_path)
}

pub fn update_project_fields(
    conn: &Connection,
    id: &str,
    name: Option<String>,
    description: Option<String>,
    status: Option<String>,
    favorite: Option<bool>,
    archived: Option<bool>,
    github_url: Option<String>,
    website_url: Option<String>,
    current_version: Option<String>,
    university: Option<String>,
    professor: Option<String>,
    academic_year: Option<String>,
    semester: Option<String>,
    cfu: Option<f64>,
    final_grade: Option<f64>,
    honors: Option<bool>,
    completed: Option<bool>,
) -> AppResult<()> {
    let ts = now();
    if let Some(n) = name {
        let n = required_name(&n, "name")?;
        conn.execute(
            "UPDATE projects SET name=?1, updated_at=?2, revision=revision+1 WHERE id=?3",
            params![n, ts, id],
        )?;
        index_search(conn, "project", id, &n, "")?;
    }
    if let Some(d) = description {
        conn.execute(
            "UPDATE projects SET description=?1, updated_at=?2, revision=revision+1 WHERE id=?3",
            params![d, ts, id],
        )?;
    }
    if let Some(s) = status {
        conn.execute(
            "UPDATE projects SET status=?1, updated_at=?2, revision=revision+1 WHERE id=?3",
            params![s, ts, id],
        )?;
    }
    if let Some(f) = favorite {
        conn.execute(
            "UPDATE projects SET favorite=?1, updated_at=?2, revision=revision+1 WHERE id=?3",
            params![f as i64, ts, id],
        )?;
    }
    if let Some(a) = archived {
        conn.execute(
            "UPDATE projects SET archived=?1, updated_at=?2, revision=revision+1 WHERE id=?3",
            params![a as i64, ts, id],
        )?;
    }
    if completed == Some(true) {
        conn.execute(
            "UPDATE projects SET status='completed', completed_at=?1, updated_at=?1, revision=revision+1 WHERE id=?2",
            params![ts, id],
        )?;
    }
    if github_url.is_some() || website_url.is_some() || current_version.is_some() {
        let github = optional_url(github_url.as_deref())?;
        let website = optional_url(website_url.as_deref())?;
        conn.execute(
            "UPDATE software_profiles SET github_url=COALESCE(?1, github_url), website_url=COALESCE(?2, website_url),
             current_version=COALESCE(?3, current_version), updated_at=?4, revision=revision+1 WHERE project_id=?5",
            params![github, website, current_version, ts, id],
        )?;
    }
    if university.is_some()
        || professor.is_some()
        || academic_year.is_some()
        || semester.is_some()
        || cfu.is_some()
        || final_grade.is_some()
        || honors.is_some()
    {
        conn.execute(
            "UPDATE university_profiles SET
                university=COALESCE(?1, university),
                professor=COALESCE(?2, professor),
                academic_year=COALESCE(?3, academic_year),
                semester=COALESCE(?4, semester),
                cfu=COALESCE(?5, cfu),
                final_grade=COALESCE(?6, final_grade),
                honors=COALESCE(?7, honors),
                updated_at=?8, revision=revision+1
             WHERE project_id=?9",
            params![
                university,
                professor,
                academic_year,
                semester,
                cfu,
                final_grade,
                honors.map(|h| h as i64),
                ts,
                id
            ],
        )?;
    }
    evaluate_project(conn, id)?;
    evaluate_user(conn)?;
    Ok(())
}

pub fn update_binding(
    conn: &Connection,
    device_id: &str,
    project_id: &str,
    local_path: Option<String>,
    primary_file_path: Option<String>,
) -> AppResult<()> {
    set_binding(
        conn,
        device_id,
        project_id,
        local_path.as_deref(),
        primary_file_path.as_deref(),
    )
}

fn recompute_time(conn: &Connection, project_id: &str) -> AppResult<i64> {
    let mut stmt = conn.prepare(
        "SELECT started_at, ended_at FROM time_entries WHERE project_id=?1 AND deleted_at IS NULL",
    )?;
    let now_dt = Utc::now();
    let mut total = 0_i64;
    let rows = stmt.query_map([project_id], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
    })?;
    for row in rows.flatten() {
        let range = TimeRange {
            started_at: parse_dt(&row.0),
            ended_at: row.1.as_deref().map(parse_dt),
        };
        total += elapsed_seconds(&range, now_dt);
    }
    let xp = DefaultXpFormula.xp_for_duration(total);
    conn.execute(
        "UPDATE projects SET total_tracked_seconds=?1, xp=?2, updated_at=?3, revision=revision+1 WHERE id=?4",
        params![total, xp, now(), project_id],
    )?;
    Ok(total)
}

pub fn running_timer(conn: &Connection, device_id: &str) -> AppResult<Option<RunningTimerDto>> {
    let row = conn
        .query_row(
            "SELECT e.id, e.project_id, e.topic_id, e.device_id, e.started_at, e.ended_at, e.notes, p.name
             FROM time_entries e JOIN projects p ON p.id = e.project_id
             WHERE e.ended_at IS NULL AND e.deleted_at IS NULL LIMIT 1",
            [],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, Option<String>>(6)?,
                    r.get::<_, String>(7)?,
                ))
            },
        )
        .optional()?;
    let Some(row) = row else {
        return Ok(None);
    };
    let seconds = elapsed_seconds(
        &TimeRange {
            started_at: parse_dt(&row.4),
            ended_at: None,
        },
        Utc::now(),
    );
    let topic_title = row.2.as_ref().and_then(|tid| {
        conn.query_row("SELECT title FROM course_topics WHERE id=?1", [tid], |r| r.get(0))
            .optional()
            .ok()
            .flatten()
    });
    Ok(Some(RunningTimerDto {
        entry: TimeEntryDto {
            id: row.0,
            project_id: row.1,
            topic_id: row.2,
            topic_title,
            device_id: row.3.clone(),
            started_at: row.4,
            ended_at: row.5,
            seconds,
            notes: row.6,
        },
        project_name: row.7,
        device_id: row.3.clone(),
        is_local_device: row.3 == device_id,
    }))
}

pub fn start_timer(
    conn: &Connection,
    device_id: &str,
    project_id: &str,
    topic_id: Option<String>,
) -> AppResult<RunningTimerDto> {
    if let Some(existing) = running_timer(conn, device_id)? {
        if existing.is_local_device {
            return Err(AppError::msg("A timer is already running"));
        }
        return Err(AppError::msg(format!(
            "Running on another device ({})",
            existing.device_id
        )));
    }
    let id = new_id();
    let ts = now();
    conn.execute(
        "INSERT INTO time_entries (id, project_id, topic_id, device_id, started_at, created_at, updated_at, revision)
         VALUES (?1,?2,?3,?4,?5,?5,?5,1)",
        params![id, project_id, topic_id, device_id, ts],
    )?;
    queue_outbox(
        conn,
        "time_entries",
        &id,
        "upsert",
        json!({"id": id, "projectId": project_id, "startedAt": ts}),
    )?;
    running_timer(conn, device_id)?.ok_or_else(|| AppError::msg("Failed to start timer"))
}

pub fn stop_timer(conn: &Connection, device_id: &str) -> AppResult<i64> {
    let Some(running) = running_timer(conn, device_id)? else {
        return Err(AppError::msg("No running timer"));
    };
    if !running.is_local_device {
        return Err(AppError::msg("Timer is owned by another device"));
    }
    let ts = now();
    conn.execute(
        "UPDATE time_entries SET ended_at=?1, updated_at=?1, revision=revision+1 WHERE id=?2",
        params![ts, running.entry.id],
    )?;
    let total = recompute_time(conn, &running.entry.project_id)?;
    let category_id: String = conn.query_row(
        "SELECT category_id FROM projects WHERE id=?1",
        [&running.entry.project_id],
        |r| r.get(0),
    )?;
    activity(
        conn,
        Some(&running.entry.project_id),
        Some(&category_id),
        "time.stopped",
        json!({"seconds": running.entry.seconds}),
    )?;
    evaluate_project(conn, &running.entry.project_id)?;
    evaluate_user(conn)?;
    let _ = device_id;
    Ok(total)
}

pub fn pause_timer(conn: &Connection, device_id: &str) -> AppResult<i64> {
    stop_timer(conn, device_id)
}

pub fn add_manual_entry(
    conn: &Connection,
    device_id: &str,
    project_id: &str,
    started_at: String,
    ended_at: String,
    topic_id: Option<String>,
    notes: Option<String>,
) -> AppResult<()> {
    let start = parse_dt(&started_at);
    let end = parse_dt(&ended_at);
    if end <= start {
        return Err(AppError::msg("End must be after start"));
    }
    let id = new_id();
    let ts = now();
    conn.execute(
        "INSERT INTO time_entries (id, project_id, topic_id, device_id, started_at, ended_at, notes, created_at, updated_at, revision)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8,1)",
        params![id, project_id, topic_id, device_id, started_at, ended_at, notes, ts],
    )?;
    recompute_time(conn, project_id)?;
    evaluate_project(conn, project_id)?;
    queue_outbox(conn, "time_entries", &id, "upsert", json!({"id": id, "projectId": project_id}))?;
    Ok(())
}

pub fn update_time_entry(
    conn: &Connection,
    id: &str,
    started_at: Option<String>,
    ended_at: Option<String>,
    notes: Option<String>,
    topic_id: Option<String>,
) -> AppResult<()> {
    let project_id: String = conn.query_row(
        "SELECT project_id FROM time_entries WHERE id=?1",
        [id],
        |r| r.get(0),
    )?;
    let ts = now();
    if let Some(start) = started_at {
        conn.execute(
            "UPDATE time_entries SET started_at=?1, updated_at=?2, revision=revision+1 WHERE id=?3",
            params![start, ts, id],
        )?;
    }
    if let Some(end) = ended_at {
        conn.execute(
            "UPDATE time_entries SET ended_at=?1, updated_at=?2, revision=revision+1 WHERE id=?3",
            params![end, ts, id],
        )?;
    }
    if let Some(notes) = notes {
        conn.execute(
            "UPDATE time_entries SET notes=?1, updated_at=?2, revision=revision+1 WHERE id=?3",
            params![notes, ts, id],
        )?;
    }
    if let Some(topic) = topic_id {
        conn.execute(
            "UPDATE time_entries SET topic_id=?1, updated_at=?2, revision=revision+1 WHERE id=?3",
            params![topic, ts, id],
        )?;
    }
    recompute_time(conn, &project_id)?;
    queue_outbox(conn, "time_entries", id, "upsert", json!({"id": id}))?;
    Ok(())
}

pub fn delete_time_entry(conn: &Connection, id: &str) -> AppResult<()> {
    let project_id: String = conn.query_row(
        "SELECT project_id FROM time_entries WHERE id=?1",
        [id],
        |r| r.get(0),
    )?;
    conn.execute(
        "UPDATE time_entries SET deleted_at=?1, updated_at=?1, revision=revision+1 WHERE id=?2",
        params![now(), id],
    )?;
    recompute_time(conn, &project_id)?;
    Ok(())
}

pub fn list_time_entries(conn: &Connection, project_id: &str) -> AppResult<Vec<TimeEntryDto>> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.project_id, e.topic_id, t.title, e.device_id, e.started_at, e.ended_at, e.notes
         FROM time_entries e LEFT JOIN course_topics t ON t.id = e.topic_id
         WHERE e.project_id=?1 AND e.deleted_at IS NULL ORDER BY e.started_at DESC LIMIT 500",
    )?;
    let now_dt = Utc::now();
    let rows = stmt.query_map([project_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<String>>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
            r.get::<_, Option<String>>(6)?,
            r.get::<_, Option<String>>(7)?,
        ))
    })?;
    Ok(rows
        .flatten()
        .map(|r| {
            let seconds = elapsed_seconds(
                &TimeRange {
                    started_at: parse_dt(&r.5),
                    ended_at: r.6.as_deref().map(parse_dt),
                },
                now_dt,
            );
            TimeEntryDto {
                id: r.0,
                project_id: r.1,
                topic_id: r.2,
                topic_title: r.3,
                device_id: r.4,
                started_at: r.5,
                ended_at: r.6,
                seconds,
                notes: r.7,
            }
        })
        .collect())
}

pub fn list_todos(conn: &Connection, filter: TodoFilter) -> AppResult<Vec<TodoDto>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.project_id, p.name, p.category_id, t.kind_id, k.name, t.title, t.description, t.priority, t.status, t.due_date, t.created_at, t.completed_at
         FROM todos t
         JOIN projects p ON p.id = t.project_id
         LEFT JOIN todo_kinds k ON k.id = t.kind_id
         WHERE t.deleted_at IS NULL
         ORDER BY CASE t.priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 WHEN 'medium' THEN 2 ELSE 3 END, t.created_at DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(TodoDto {
            id: r.get(0)?,
            project_id: r.get(1)?,
            project_name: r.get(2)?,
            category_id: r.get(3)?,
            kind_id: r.get(4)?,
            kind_name: r.get(5)?,
            title: r.get(6)?,
            description: r.get(7)?,
            priority: r.get(8)?,
            status: r.get(9)?,
            due_date: r.get(10)?,
            created_at: r.get(11)?,
            completed_at: r.get(12)?,
        })
    })?;
    Ok(rows
        .flatten()
        .filter(|t| {
            todo_matches(
                &t.title,
                &t.description,
                &t.status,
                t.kind_id.as_deref().unwrap_or(""),
                &t.project_id,
                &t.category_id,
                &filter,
            )
        })
        .collect())
}

pub fn create_todo(
    conn: &Connection,
    project_id: &str,
    title: &str,
    description: Option<String>,
    kind_id: Option<String>,
    priority: Option<String>,
    due_date: Option<String>,
) -> AppResult<TodoDto> {
    let title = required_name(title, "title")?;
    let id = new_id();
    let ts = now();
    let priority = priority.unwrap_or_else(|| "medium".into());
    conn.execute(
        "INSERT INTO todos (id, project_id, kind_id, title, description, priority, status, due_date, created_at, updated_at, revision)
         VALUES (?1,?2,?3,?4,?5,?6,'open',?7,?8,?8,1)",
        params![id, project_id, kind_id, title, description.clone().unwrap_or_default(), priority, due_date, ts],
    )?;
    index_search(conn, "todo", &id, &title, description.as_deref().unwrap_or(""))?;
    activity(conn, Some(project_id), None, "todo.created", json!({"title": title}))?;
    queue_outbox(conn, "todos", &id, "upsert", json!({"id": id, "title": title, "projectId": project_id}))?;
    list_todos(
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
    .find(|t| t.id == id)
    .ok_or_else(|| AppError::msg("Todo not found after create"))
}

pub fn set_todo_status(conn: &Connection, id: &str, status: &str) -> AppResult<()> {
    let ts = now();
    let completed = if status == "completed" { Some(ts.clone()) } else { None };
    conn.execute(
        "UPDATE todos SET status=?1, completed_at=?2, updated_at=?3, revision=revision+1 WHERE id=?4",
        params![status, completed, ts, id],
    )?;
    let project_id: String = conn.query_row("SELECT project_id FROM todos WHERE id=?1", [id], |r| r.get(0))?;
    if status == "completed" {
        activity(conn, Some(&project_id), None, "todo.completed", json!({"id": id}))?;
        evaluate_project(conn, &project_id)?;
    }
    queue_outbox(conn, "todos", id, "upsert", json!({"id": id, "status": status}))?;
    Ok(())
}

pub fn delete_todo(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE todos SET deleted_at=?1, updated_at=?1, revision=revision+1 WHERE id=?2",
        params![now(), id],
    )?;
    Ok(())
}

pub fn todo_kinds(conn: &Connection, category_id: &str) -> AppResult<Vec<(String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, slug, name FROM todo_kinds WHERE category_id=?1 AND deleted_at IS NULL ORDER BY sort_order",
    )?;
    let rows = stmt.query_map([category_id], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn list_links(conn: &Connection, project_id: &str) -> AppResult<Vec<LinkDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, title, url, icon, description, pinned, sort_order FROM project_links
         WHERE project_id=?1 AND deleted_at IS NULL ORDER BY pinned DESC, sort_order, title",
    )?;
    let rows = stmt.query_map([project_id], |r| {
        Ok(LinkDto {
            id: r.get(0)?,
            project_id: r.get(1)?,
            title: r.get(2)?,
            url: r.get(3)?,
            icon: r.get(4)?,
            description: r.get(5)?,
            pinned: r.get::<_, i64>(6)? != 0,
            sort_order: r.get(7)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn upsert_link(conn: &Connection, link: LinkDto) -> AppResult<LinkDto> {
    optional_url(Some(&link.url))?;
    let title = required_name(&link.title, "title")?;
    let ts = now();
    let id = if link.id.is_empty() { new_id() } else { link.id.clone() };
    conn.execute(
        "INSERT INTO project_links (id, project_id, title, url, icon, description, pinned, sort_order, created_at, updated_at, revision)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9,1)
         ON CONFLICT(id) DO UPDATE SET title=excluded.title, url=excluded.url, icon=excluded.icon,
           description=excluded.description, pinned=excluded.pinned, sort_order=excluded.sort_order,
           updated_at=excluded.updated_at, revision=revision+1",
        params![id, link.project_id, title, link.url, link.icon, link.description, link.pinned as i64, link.sort_order, ts],
    )?;
    index_search(conn, "link", &id, &title, &link.url)?;
    Ok(LinkDto { id, title, ..link })
}

pub fn delete_link(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE project_links SET deleted_at=?1, updated_at=?1, revision=revision+1 WHERE id=?2",
        params![now(), id],
    )?;
    Ok(())
}

pub fn list_project_files(conn: &Connection, project_id: &str, device_id: &str) -> AppResult<Vec<ProjectFileDto>> {
    let mut stmt = conn.prepare(
        "SELECT f.id, f.project_id, f.display_name, f.relative_path, f.mime, f.size_bytes, f.file_kind, b.absolute_path,
                d.status, d.stage, d.limitation
         FROM project_files f
         LEFT JOIN device_file_bindings b ON b.file_id = f.id AND b.device_id = ?2
         LEFT JOIN document_records d ON d.file_id = f.id
         WHERE f.project_id=?1 AND f.deleted_at IS NULL ORDER BY f.display_name",
    )?;
    let rows = stmt.query_map(params![project_id, device_id], |r| {
        Ok(ProjectFileDto {
            id: r.get(0)?,
            project_id: r.get(1)?,
            display_name: r.get(2)?,
            relative_path: r.get(3)?,
            mime: r.get(4)?,
            size_bytes: r.get(5)?,
            file_kind: r.get(6)?,
            absolute_path: r.get(7)?,
            index_status: r.get(8)?,
            index_stage: r.get(9)?,
            index_limitation: r.get(10)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn add_project_file(
    conn: &Connection,
    device_id: &str,
    project_id: &str,
    display_name: &str,
    absolute_path: &str,
    file_kind: Option<String>,
) -> AppResult<ProjectFileDto> {
    let id = new_id();
    let ts = now();
    let size = std::fs::metadata(absolute_path).ok().map(|m| m.len() as i64);
    conn.execute(
        "INSERT INTO project_files (id, project_id, display_name, relative_path, size_bytes, file_kind, created_at, updated_at, revision)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?7,1)",
        params![id, project_id, display_name, display_name, size, file_kind, ts],
    )?;
    conn.execute(
        "INSERT INTO device_file_bindings (device_id, file_id, absolute_path, updated_at) VALUES (?1,?2,?3,?4)
         ON CONFLICT(device_id, file_id) DO UPDATE SET absolute_path=excluded.absolute_path, updated_at=excluded.updated_at",
        params![device_id, id, absolute_path, ts],
    )?;
    Ok(ProjectFileDto {
        id,
        project_id: project_id.into(),
        display_name: display_name.into(),
        relative_path: Some(display_name.into()),
        mime: None,
        size_bytes: size,
        file_kind,
        absolute_path: Some(absolute_path.into()),
        index_status: None,
        index_stage: None,
        index_limitation: None,
    })
}

pub fn remove_project_file(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE project_files SET deleted_at=?1, updated_at=?1, revision=revision+1 WHERE id=?2",
        params![now(), id],
    )?;
    Ok(())
}

pub fn list_commands(conn: &Connection, project_id: &str) -> AppResult<Vec<CommandDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, command, description, working_directory, pinned, favorite, sort_order
         FROM commands WHERE project_id=?1 AND deleted_at IS NULL ORDER BY pinned DESC, sort_order, name",
    )?;
    let rows = stmt.query_map([project_id], |r| {
        let command: String = r.get(3)?;
        Ok(CommandDto {
            id: r.get(0)?,
            project_id: r.get(1)?,
            name: r.get(2)?,
            command: command.clone(),
            description: r.get(4)?,
            working_directory: r.get(5)?,
            pinned: r.get::<_, i64>(6)? != 0,
            favorite: r.get::<_, i64>(7)? != 0,
            sort_order: r.get(8)?,
            dangerous: is_dangerous(&command),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn upsert_command(conn: &Connection, cmd: CommandDto) -> AppResult<CommandDto> {
    let name = required_name(&cmd.name, "command name")?;
    let command = cmd.command.trim().to_string();
    if command.is_empty() {
        return Err(AppError::msg("command is required"));
    }
    let ts = now();
    let id = if cmd.id.is_empty() { new_id() } else { cmd.id.clone() };
    conn.execute(
        "INSERT INTO commands (id, project_id, name, command, description, working_directory, pinned, favorite, sort_order, created_at, updated_at, revision)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10,1)
         ON CONFLICT(id) DO UPDATE SET name=excluded.name, command=excluded.command, description=excluded.description,
           working_directory=excluded.working_directory, pinned=excluded.pinned, favorite=excluded.favorite,
           sort_order=excluded.sort_order, updated_at=excluded.updated_at, revision=revision+1",
        params![id, cmd.project_id, name, command, cmd.description, cmd.working_directory, cmd.pinned as i64, cmd.favorite as i64, cmd.sort_order, ts],
    )?;
    index_search(conn, "command", &id, &name, &command)?;
    list_commands(conn, &cmd.project_id)?
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(|| AppError::msg("Command missing"))
}

pub fn delete_command(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE commands SET deleted_at=?1, updated_at=?1, revision=revision+1 WHERE id=?2",
        params![now(), id],
    )?;
    Ok(())
}

pub fn list_versions(conn: &Connection, project_id: &str) -> AppResult<Vec<VersionDto>> {
    let mut stmt = conn.prepare(
        "SELECT v.id, v.project_id, v.version, v.title, v.released_at,
                COALESCE((SELECT group_concat(body, char(10)) FROM changelog_entries c WHERE c.version_id=v.id AND c.deleted_at IS NULL), '')
         FROM project_versions v WHERE v.project_id=?1 AND v.deleted_at IS NULL ORDER BY v.created_at DESC",
    )?;
    let rows = stmt.query_map([project_id], |r| {
        Ok(VersionDto {
            id: r.get(0)?,
            project_id: r.get(1)?,
            version: r.get(2)?,
            title: r.get(3)?,
            released_at: r.get(4)?,
            changelog: r.get(5)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn add_version(
    conn: &Connection,
    project_id: &str,
    version: &str,
    title: Option<String>,
    changelog: Option<String>,
) -> AppResult<VersionDto> {
    bluephoenix_domain::semver_check::parse_version(version).map_err(|_| AppError::InvalidVersion)?;
    let id = new_id();
    let ts = now();
    conn.execute(
        "INSERT INTO project_versions (id, project_id, version, title, released_at, created_at, updated_at, revision)
         VALUES (?1,?2,?3,?4,?5,?5,?5,1)",
        params![id, project_id, version, title, ts],
    )?;
    if let Some(body) = changelog.filter(|s| !s.trim().is_empty()) {
        conn.execute(
            "INSERT INTO changelog_entries (id, version_id, body, created_at, updated_at, revision) VALUES (?1,?2,?3,?4,?4,1)",
            params![new_id(), id, body, ts],
        )?;
    }
    conn.execute(
        "UPDATE software_profiles SET current_version=?1, updated_at=?2, revision=revision+1 WHERE project_id=?3",
        params![version, ts, project_id],
    )?;
    conn.execute(
        "UPDATE projects SET released_at=?1, updated_at=?1, revision=revision+1 WHERE id=?2 AND released_at IS NULL",
        params![ts, project_id],
    )?;
    activity(conn, Some(project_id), None, "version.released", json!({"version": version}))?;
    evaluate_project(conn, project_id)?;
    list_versions(conn, project_id)?
        .into_iter()
        .find(|v| v.id == id)
        .ok_or_else(|| AppError::msg("Version missing"))
}

pub fn list_topics(conn: &Connection, project_id: &str) -> AppResult<Vec<TopicDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, title, description, sort_order, status, estimated_seconds, completion_percent
         FROM course_topics WHERE project_id=?1 AND deleted_at IS NULL ORDER BY sort_order, title",
    )?;
    let rows = stmt.query_map([project_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, i64>(4)?,
            r.get::<_, String>(5)?,
            r.get::<_, Option<i64>>(6)?,
            r.get::<_, f64>(7)?,
        ))
    })?;
    let mut out = Vec::new();
    for r in rows.flatten() {
        let tracked = topic_seconds(conn, &r.0);
        out.push(TopicDto {
            id: r.0,
            project_id: r.1,
            title: r.2,
            description: r.3,
            sort_order: r.4,
            status: r.5,
            estimated_seconds: r.6,
            tracked_seconds: tracked,
            completion_percent: r.7,
        });
    }
    Ok(out)
}

fn topic_seconds(conn: &Connection, topic_id: &str) -> i64 {
    let mut stmt = conn
        .prepare("SELECT started_at, ended_at FROM time_entries WHERE topic_id=?1 AND deleted_at IS NULL")
        .ok();
    let Some(mut stmt) = stmt.take() else {
        return 0;
    };
    let now_dt = Utc::now();
    stmt.query_map([topic_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)))
        .ok()
        .map(|rows| {
            rows.flatten()
                .map(|row| {
                    elapsed_seconds(
                        &TimeRange {
                            started_at: parse_dt(&row.0),
                            ended_at: row.1.as_deref().map(parse_dt),
                        },
                        now_dt,
                    )
                })
                .sum()
        })
        .unwrap_or(0)
}

pub fn upsert_topic(conn: &Connection, topic: TopicDto) -> AppResult<TopicDto> {
    let title = required_name(&topic.title, "title")?;
    let ts = now();
    let id = if topic.id.is_empty() { new_id() } else { topic.id.clone() };
    conn.execute(
        "INSERT INTO course_topics (id, project_id, title, description, sort_order, status, estimated_seconds, completion_percent, created_at, updated_at, revision)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9,1)
         ON CONFLICT(id) DO UPDATE SET title=excluded.title, description=excluded.description, sort_order=excluded.sort_order,
           status=excluded.status, estimated_seconds=excluded.estimated_seconds, completion_percent=excluded.completion_percent,
           updated_at=excluded.updated_at, revision=revision+1",
        params![id, topic.project_id, title, topic.description, topic.sort_order, topic.status, topic.estimated_seconds, topic.completion_percent, ts],
    )?;
    if topic.status == "completed" {
        activity(conn, Some(&topic.project_id), None, "topic.completed", json!({"title": title}))?;
        evaluate_project(conn, &topic.project_id)?;
    }
    list_topics(conn, &topic.project_id)?
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| AppError::msg("Topic missing"))
}

pub fn delete_topic(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE course_topics SET deleted_at=?1, updated_at=?1, revision=revision+1 WHERE id=?2",
        params![now(), id],
    )?;
    Ok(())
}

pub fn list_lessons(conn: &Connection, project_id: &str) -> AppResult<Vec<LessonDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, topic_id, date, start_time, end_time, duration_seconds, attended, notes
         FROM course_lessons WHERE project_id=?1 AND deleted_at IS NULL ORDER BY date DESC",
    )?;
    let rows = stmt.query_map([project_id], |r| {
        Ok(LessonDto {
            id: r.get(0)?,
            project_id: r.get(1)?,
            topic_id: r.get(2)?,
            date: r.get(3)?,
            start_time: r.get(4)?,
            end_time: r.get(5)?,
            duration_seconds: r.get(6)?,
            attended: r.get::<_, i64>(7)? != 0,
            notes: r.get(8)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn upsert_lesson(conn: &Connection, lesson: LessonDto) -> AppResult<LessonDto> {
    let ts = now();
    let id = if lesson.id.is_empty() { new_id() } else { lesson.id.clone() };
    conn.execute(
        "INSERT INTO course_lessons (id, project_id, topic_id, date, start_time, end_time, duration_seconds, attended, notes, created_at, updated_at, revision)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10,1)
         ON CONFLICT(id) DO UPDATE SET topic_id=excluded.topic_id, date=excluded.date, start_time=excluded.start_time,
           end_time=excluded.end_time, duration_seconds=excluded.duration_seconds, attended=excluded.attended,
           notes=excluded.notes, updated_at=excluded.updated_at, revision=revision+1",
        params![id, lesson.project_id, lesson.topic_id, lesson.date, lesson.start_time, lesson.end_time, lesson.duration_seconds, lesson.attended as i64, lesson.notes, ts],
    )?;
    if lesson.attended {
        activity(conn, Some(&lesson.project_id), None, "lesson.attended", json!({"date": lesson.date}))?;
    }
    evaluate_project(conn, &lesson.project_id)?;
    Ok(LessonDto { id, ..lesson })
}

pub fn delete_lesson(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE course_lessons SET deleted_at=?1, updated_at=?1, revision=revision+1 WHERE id=?2",
        params![now(), id],
    )?;
    Ok(())
}

pub fn list_exams(conn: &Connection, project_id: Option<&str>, category_id: Option<&str>) -> AppResult<Vec<ExamDto>> {
    let mut sql = String::from(
        "SELECT e.id, e.project_id, p.name, e.date, e.grade, e.status, e.notes
         FROM exam_attempts e JOIN projects p ON p.id = e.project_id
         WHERE e.deleted_at IS NULL",
    );
    if project_id.is_some() {
        sql.push_str(" AND e.project_id = ?1");
    } else if category_id.is_some() {
        sql.push_str(" AND p.category_id = ?1");
    }
    sql.push_str(" ORDER BY e.date DESC, e.created_at DESC");
    let mut stmt = conn.prepare(&sql)?;
    let bind = project_id.or(category_id);
    let rows = if let Some(id) = bind {
        stmt.query_map([id], map_exam)?.collect::<Result<Vec<_>, _>>()?
    } else {
        stmt.query_map([], map_exam)?.collect::<Result<Vec<_>, _>>()?
    };
    Ok(rows)
}

fn map_exam(r: &rusqlite::Row<'_>) -> rusqlite::Result<ExamDto> {
    Ok(ExamDto {
        id: r.get(0)?,
        project_id: r.get(1)?,
        project_name: r.get(2)?,
        date: r.get(3)?,
        grade: r.get(4)?,
        status: r.get(5)?,
        notes: r.get(6)?,
    })
}

pub fn upsert_exam(conn: &Connection, exam: ExamDto) -> AppResult<ExamDto> {
    if let Some(grade) = exam.grade {
        exams::validate_grade(grade, 30.0).map_err(|_| AppError::InvalidGrade)?;
    }
    let ts = now();
    let id = if exam.id.is_empty() { new_id() } else { exam.id.clone() };
    conn.execute(
        "INSERT INTO exam_attempts (id, project_id, date, grade, status, notes, created_at, updated_at, revision)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?7,1)
         ON CONFLICT(id) DO UPDATE SET date=excluded.date, grade=excluded.grade, status=excluded.status,
           notes=excluded.notes, updated_at=excluded.updated_at, revision=revision+1",
        params![id, exam.project_id, exam.date, exam.grade, exam.status, exam.notes, ts],
    )?;
    let event = match exam.status.as_str() {
        "passed" => "exam.passed",
        "failed" => "exam.failed",
        _ => "exam.attempted",
    };
    activity(conn, Some(&exam.project_id), None, event, json!({"grade": exam.grade, "status": exam.status}))?;
    evaluate_project(conn, &exam.project_id)?;
    evaluate_user(conn)?;
    queue_outbox(
        conn,
        "exam_attempts",
        &id,
        "upsert",
        json!({"id": id, "projectId": exam.project_id, "status": exam.status, "grade": exam.grade}),
    )?;
    Ok(ExamDto { id, ..exam })
}

pub fn delete_exam(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE exam_attempts SET deleted_at=?1, updated_at=?1, revision=revision+1 WHERE id=?2",
        params![now(), id],
    )?;
    Ok(())
}

pub fn list_activity(conn: &Connection, category_id: Option<&str>, project_id: Option<&str>) -> AppResult<Vec<ActivityDto>> {
    let mut sql = String::from(
        "SELECT a.id, a.project_id, p.name, a.category_id, a.event_type, a.payload, a.created_at
         FROM activity_events a LEFT JOIN projects p ON p.id = a.project_id
         WHERE a.deleted_at IS NULL",
    );
    if project_id.is_some() {
        sql.push_str(" AND a.project_id = ?1");
    } else if category_id.is_some() {
        sql.push_str(" AND a.category_id = ?1");
    }
    sql.push_str(" ORDER BY a.created_at DESC LIMIT 200");
    let mut stmt = conn.prepare(&sql)?;
    let mapper = |r: &rusqlite::Row<'_>| {
        let payload: String = r.get(5)?;
        Ok(ActivityDto {
            id: r.get(0)?,
            project_id: r.get(1)?,
            project_name: r.get(2)?,
            category_id: r.get(3)?,
            event_type: r.get(4)?,
            payload: serde_json::from_str(&payload).unwrap_or(json!({})),
            created_at: r.get(6)?,
        })
    };
    let rows = if let Some(id) = project_id.or(category_id) {
        stmt.query_map([id], mapper)?.filter_map(|r| r.ok()).collect()
    } else {
        stmt.query_map([], mapper)?.filter_map(|r| r.ok()).collect()
    };
    Ok(rows)
}

pub fn university_dashboard(conn: &Connection) -> AppResult<GpaDashboardDto> {
    let settings = load_settings(conn);
    let mut stmt = conn.prepare(
        "SELECT p.name, u.cfu, u.final_grade, u.honors FROM projects p
         JOIN university_profiles u ON u.project_id = p.id
         WHERE p.deleted_at IS NULL AND p.category_id = ?1",
    )?;
    let courses: Vec<CourseGradeInput> = stmt
        .query_map([UNIVERSITY_CATEGORY_ID], |r| {
            Ok(CourseGradeInput {
                name: r.get(0)?,
                cfu: r.get(1)?,
                final_grade: r.get(2)?,
                honors: r.get::<_, i64>(3)? != 0,
                included_explicitly: true,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    let config = GpaConfig {
        include_failed: settings.include_failed_grades,
        ..GpaConfig::default()
    };
    let gpa = gpa::compute_gpa(&courses, &config)?;
    let failed_attempts: i64 = conn.query_row(
        "SELECT COUNT(*) FROM exam_attempts e JOIN projects p ON p.id=e.project_id
         WHERE e.deleted_at IS NULL AND e.status='failed' AND p.category_id=?1",
        [UNIVERSITY_CATEGORY_ID],
        |r| r.get(0),
    )?;
    let total_attempts: i64 = conn.query_row(
        "SELECT COUNT(*) FROM exam_attempts e JOIN projects p ON p.id=e.project_id
         WHERE e.deleted_at IS NULL AND p.category_id=?1",
        [UNIVERSITY_CATEGORY_ID],
        |r| r.get(0),
    )?;
    let study_seconds: i64 = conn.query_row(
        "SELECT COALESCE(SUM(total_tracked_seconds),0) FROM projects WHERE category_id=?1 AND deleted_at IS NULL",
        [UNIVERSITY_CATEGORY_ID],
        |r| r.get(0),
    )?;
    let mut att_lessons = 0_i64;
    let mut att_ok = 0_i64;
    {
        let mut stmt = conn.prepare(
            "SELECT l.attended FROM course_lessons l JOIN projects p ON p.id=l.project_id
             WHERE l.deleted_at IS NULL AND p.category_id=?1",
        )?;
        let rows = stmt.query_map([UNIVERSITY_CATEGORY_ID], |r| r.get::<_, i64>(0))?;
        for v in rows.flatten() {
            att_lessons += 1;
            if v != 0 {
                att_ok += 1;
            }
        }
    }
    let attendance_percent = if att_lessons == 0 {
        0.0
    } else {
        (att_ok as f64 / att_lessons as f64) * 100.0
    };
    Ok(GpaDashboardDto {
        simple_average: gpa.simple_average,
        weighted_average: gpa.weighted_average,
        used_weighted: gpa.used_weighted,
        included: gpa.included.iter().map(|c| c.name.clone()).collect(),
        excluded_missing_final: gpa.excluded_missing_final,
        excluded_failed: gpa.excluded_failed,
        total_cfu: gpa.total_cfu,
        completed_cfu: gpa.completed_cfu,
        passed_courses: gpa.passed_courses as i64,
        failed_attempts,
        total_attempts,
        study_seconds,
        attendance_percent,
        max_grade: 30.0,
    })
}

pub fn search(conn: &Connection, query: &str) -> AppResult<Vec<SearchHitDto>> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let like = format!("%{q}%");
    let mut hits = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT entity_type, entity_id, title, body FROM search_index WHERE search_index MATCH ?1 LIMIT 40",
    )?;
    if let Ok(rows) = stmt.query_map([q], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    }) {
        for row in rows.flatten() {
            let mut hit = SearchHitDto {
                entity_type: row.0,
                entity_id: row.1,
                title: row.2,
                subtitle: row.3,
                category_id: None,
                project_id: None,
            };
            hydrate_search_hit(conn, &mut hit);
            hits.push(hit);
        }
    }
    if hits.is_empty() {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, category_id FROM projects WHERE deleted_at IS NULL AND (name LIKE ?1 OR description LIKE ?1) LIMIT 20",
        )?;
        for row in stmt
            .query_map([&like], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })?
            .flatten()
        {
            hits.push(SearchHitDto {
                entity_type: "project".into(),
                entity_id: row.0.clone(),
                title: row.1,
                subtitle: row.2,
                category_id: Some(row.3),
                project_id: Some(row.0),
            });
        }
    }
    Ok(hits)
}

pub fn global_progress(conn: &Connection) -> AppResult<GlobalProgressDto> {
    let xp: i64 = conn.query_row(
        "SELECT COALESCE(SUM(xp),0) FROM projects WHERE deleted_at IS NULL",
        [],
        |r| r.get(0),
    )?;
    let tracked: i64 = conn.query_row(
        "SELECT COALESCE(SUM(total_tracked_seconds),0) FROM projects WHERE deleted_at IS NULL",
        [],
        |r| r.get(0),
    )?;
    let level = DefaultXpFormula.level_for_xp(xp);
    Ok(GlobalProgressDto {
        xp,
        level: level.level,
        ratio: level.ratio(),
        tracked_seconds: tracked,
    })
}

pub fn list_unlocks(conn: &Connection, project_id: Option<&str>) -> Vec<UnlockDto> {
    unlocks_for(conn, project_id)
}

pub fn all_definitions() -> serde_json::Value {
    serde_json::to_value(
        achievements::ACHIEVEMENTS
            .iter()
            .map(|d| {
                json!({
                    "id": d.id,
                    "name": d.name,
                    "description": d.description,
                    "icon": d.icon,
                    "rarity": d.rarity.as_str(),
                    "scope": match d.scope {
                        achievements::AchievementScope::Project => "project",
                        achievements::AchievementScope::User => "user",
                    },
                    "displayOrder": d.display_order,
                })
            })
            .collect::<Vec<_>>(),
    )
    .unwrap_or(json!([]))
}

fn already_ids(conn: &Connection, project_id: Option<&str>) -> Vec<String> {
    unlocks_for(conn, project_id)
        .into_iter()
        .map(|u| u.achievement_id)
        .collect()
}

fn award(conn: &Connection, achievement_id: &str, project_id: Option<&str>) -> AppResult<()> {
    let exists: i64 = if let Some(pid) = project_id {
        conn.query_row(
            "SELECT COUNT(*) FROM achievement_unlocks WHERE achievement_id=?1 AND project_id=?2 AND deleted_at IS NULL",
            params![achievement_id, pid],
            |r| r.get(0),
        )?
    } else {
        conn.query_row(
            "SELECT COUNT(*) FROM achievement_unlocks WHERE achievement_id=?1 AND project_id IS NULL AND deleted_at IS NULL",
            [achievement_id],
            |r| r.get(0),
        )?
    };
    if exists > 0 {
        return Ok(());
    }
    let ts = now();
    conn.execute(
        "INSERT INTO achievement_unlocks (id, achievement_id, project_id, earned_at, created_at, updated_at, revision)
         VALUES (?1,?2,?3,?4,?4,?4,1)",
        params![new_id(), achievement_id, project_id, ts],
    )?;
    activity(
        conn,
        project_id,
        None,
        "achievement.unlocked",
        json!({"id": achievement_id}),
    )?;
    Ok(())
}

pub fn evaluate_project(conn: &Connection, project_id: &str) -> AppResult<Vec<String>> {
    let category_id: String = conn.query_row(
        "SELECT category_id FROM projects WHERE id=?1",
        [project_id],
        |r| r.get(0),
    )?;
    let kind = kind_of(conn, &category_id)?;
    let (tracked, status, completed_at): (i64, String, Option<String>) = conn.query_row(
        "SELECT total_tracked_seconds, status, completed_at FROM projects WHERE id=?1",
        [project_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let completed_todos: i64 = conn.query_row(
        "SELECT COUNT(*) FROM todos WHERE project_id=?1 AND status='completed' AND deleted_at IS NULL",
        [project_id],
        |r| r.get(0),
    )?;
    let mut snap = AchievementSnapshot {
        kind: Some(kind),
        tracked_seconds: tracked,
        study_seconds: if kind == CategoryKind::University { tracked } else { 0 },
        completed_todos,
        ..AchievementSnapshot::default()
    };
    if kind == CategoryKind::Software {
        let (github, website, version): (Option<String>, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT github_url, website_url, current_version FROM software_profiles WHERE project_id=?1",
                [project_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?
            .unwrap_or((None, None, None));
        snap.has_github = github.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
        snap.has_website = website.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
        snap.has_release = conn
            .query_row(
                "SELECT COUNT(*) FROM project_versions WHERE project_id=?1 AND deleted_at IS NULL",
                [project_id],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;
        snap.has_version_one = version
            .as_deref()
            .and_then(|v| is_at_least_one(v).ok())
            .unwrap_or(false);
        let settings = load_settings(conn);
        if let (Some(path), _) = project_binding(conn, &current_device(conn), project_id) {
            snap.commit_count = crate::git::commit_count(&settings.git_path, Path::new(&path));
        }
    }
    if kind == CategoryKind::University {
        snap.passed_exams = conn.query_row(
            "SELECT COUNT(*) FROM exam_attempts WHERE project_id=?1 AND status='passed' AND deleted_at IS NULL",
            [project_id],
            |r| r.get(0),
        )?;
        let max: f64 = conn
            .query_row(
                "SELECT grading_max FROM university_profiles WHERE project_id=?1",
                [project_id],
                |r| r.get(0),
            )
            .unwrap_or(30.0);
        snap.perfect_exam = conn
            .query_row(
                "SELECT COUNT(*) FROM exam_attempts WHERE project_id=?1 AND status='passed' AND grade >= ?2 AND deleted_at IS NULL",
                params![project_id, max],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;
        snap.attended_lessons = conn.query_row(
            "SELECT COUNT(*) FROM course_lessons WHERE project_id=?1 AND attended=1 AND deleted_at IS NULL",
            [project_id],
            |r| r.get(0),
        )?;
        snap.completed_courses = if status == "completed" || completed_at.is_some() {
            1
        } else {
            0
        };
    }
    let already = already_ids(conn, Some(project_id));
    let newly = achievements::evaluate(&snap, &already);
    let mut ids = Vec::new();
    for item in newly {
        award(conn, &item.id, Some(project_id))?;
        ids.push(item.id);
    }
    Ok(ids)
}

fn current_device(conn: &Connection) -> String {
    conn.query_row("SELECT id FROM devices LIMIT 1", [], |r| r.get(0))
        .unwrap_or_default()
}

pub fn evaluate_user(conn: &Connection) -> AppResult<Vec<String>> {
    let project_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM projects WHERE deleted_at IS NULL",
        [],
        |r| r.get(0),
    )?;
    let course_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM projects WHERE category_id=?1 AND deleted_at IS NULL",
        [UNIVERSITY_CATEGORY_ID],
        |r| r.get(0),
    )?;
    let completed_courses: i64 = conn.query_row(
        "SELECT COUNT(*) FROM projects WHERE category_id=?1 AND (status='completed' OR completed_at IS NOT NULL) AND deleted_at IS NULL",
        [UNIVERSITY_CATEGORY_ID],
        |r| r.get(0),
    )?;
    let dashboard = university_dashboard(conn).ok();
    let snap = AchievementSnapshot {
        kind: None,
        project_count: project_count.max(course_count),
        completed_courses,
        completed_cfu: dashboard.map(|d| d.completed_cfu).unwrap_or(0.0),
        ..AchievementSnapshot::default()
    };
    let already = already_ids(conn, None);
    let newly = achievements::evaluate(&snap, &already);
    let mut ids = Vec::new();
    for item in newly {
        award(conn, &item.id, None)?;
        ids.push(item.id);
    }
    Ok(ids)
}

pub fn sync_status(conn: &Connection) -> SyncStatusDto {
    let pending: i64 = conn
        .query_row("SELECT COUNT(*) FROM sync_outbox", [], |r| r.get(0))
        .unwrap_or(0);
    let last_error: Option<String> = conn
        .query_row(
            "SELECT last_error FROM sync_state WHERE collection='metadata'",
            [],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten()
        .flatten();
    let last_pulled_at: Option<String> = conn
        .query_row(
            "SELECT last_pulled_at FROM sync_state WHERE collection='metadata'",
            [],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten()
        .flatten();
    let (status, label) = if last_error.is_some() {
        ("sync_error", "Sync error")
    } else if pending > 0 {
        ("changes_pending", "Changes pending")
    } else {
        ("synced", "Synced")
    };
    SyncStatusDto {
        status: status.into(),
        label: label.into(),
        pending,
        last_error,
        last_pulled_at,
    }
}

pub fn pending_outbox(conn: &Connection) -> AppResult<Vec<(String, String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, table_name, row_id, payload FROM sync_outbox ORDER BY created_at LIMIT 100",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn ack_outbox(conn: &Connection, ids: &[String]) -> AppResult<()> {
    for id in ids {
        conn.execute("DELETE FROM sync_outbox WHERE id=?1", [id])?;
    }
    Ok(())
}

pub fn enqueue_job(conn: &Connection, kind: &str, payload: serde_json::Value) -> AppResult<String> {
    let id = new_id();
    let ts = now();
    conn.execute(
        "INSERT INTO background_jobs (id, kind, payload, status, created_at, updated_at) VALUES (?1,?2,?3,'queued',?4,?4)",
        params![id, kind, payload.to_string(), ts],
    )?;
    Ok(id)
}

#[allow(dead_code)]
pub fn take_job(conn: &Connection) -> AppResult<Option<(String, String, String)>> {
    let row = conn
        .query_row(
            "SELECT id, kind, payload FROM background_jobs WHERE status='queued' ORDER BY created_at LIMIT 1",
            [],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)),
        )
        .optional()?;
    if let Some((id, kind, payload)) = row {
        conn.execute(
            "UPDATE background_jobs SET status='running', updated_at=?1 WHERE id=?2",
            params![now(), id],
        )?;
        Ok(Some((id, kind, payload)))
    } else {
        Ok(None)
    }
}

#[allow(dead_code)]
pub fn finish_job(conn: &Connection, id: &str, error: Option<String>) -> AppResult<()> {
    if let Some(err) = error {
        conn.execute(
            "UPDATE background_jobs SET status='error', error=?1, updated_at=?2 WHERE id=?3",
            params![err, now(), id],
        )?;
    } else {
        conn.execute(
            "UPDATE background_jobs SET status='done', updated_at=?1 WHERE id=?2",
            params![now(), id],
        )?;
    }
    Ok(())
}

pub fn cache_files(conn: &Connection, project_id: &str, entries: &[FileEntryDto]) -> AppResult<()> {
    conn.execute("DELETE FROM file_index_cache WHERE project_id=?1", [project_id])?;
    let ts = now();
    for entry in entries {
        conn.execute(
            "INSERT INTO file_index_cache (id, project_id, path, name, is_dir, size_bytes, mtime, content_hash, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,NULL,?7)",
            params![new_id(), project_id, entry.path, entry.name, entry.is_dir as i64, entry.size_bytes, ts],
        )?;
    }
    Ok(())
}

pub fn cached_files(conn: &Connection, project_id: &str) -> AppResult<Vec<FileEntryDto>> {
    let mut stmt = conn.prepare(
        "SELECT name, path, is_dir, size_bytes FROM file_index_cache WHERE project_id=?1 ORDER BY is_dir DESC, name",
    )?;
    let rows = stmt.query_map([project_id], |r| {
        Ok(FileEntryDto {
            name: r.get(0)?,
            path: r.get(1)?,
            is_dir: r.get::<_, i64>(2)? != 0,
            size_bytes: r.get(3)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn export_all(conn: &Connection) -> AppResult<serde_json::Value> {
    let dump_table = |name: &str| -> AppResult<Vec<serde_json::Value>> {
        let mut stmt = conn.prepare(&format!("SELECT * FROM {name}"))?;
        let col_count = stmt.column_count();
        let names: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();
        let mut rows_iter = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows_iter.next()? {
            let mut obj = serde_json::Map::new();
            for i in 0..col_count {
                let key = names[i].clone();
                let value = row
                    .get::<_, Option<String>>(i)
                    .ok()
                    .flatten()
                    .map(serde_json::Value::String)
                    .or_else(|| row.get::<_, Option<i64>>(i).ok().flatten().map(|n| json!(n)))
                    .or_else(|| row.get::<_, Option<f64>>(i).ok().flatten().map(|n| json!(n)))
                    .unwrap_or(serde_json::Value::Null);
                obj.insert(key, value);
            }
            out.push(serde_json::Value::Object(obj));
        }
        Ok(out)
    };
    Ok(json!({
        "exportedAt": now(),
        "version": env!("CARGO_PKG_VERSION"),
        "categories": dump_table("categories")?,
        "projects": dump_table("projects")?,
        "software_profiles": dump_table("software_profiles")?,
        "university_profiles": dump_table("university_profiles")?,
        "todos": dump_table("todos")?,
        "time_entries": dump_table("time_entries")?,
        "commands": dump_table("commands")?,
        "project_links": dump_table("project_links")?,
        "exam_attempts": dump_table("exam_attempts")?,
        "course_topics": dump_table("course_topics")?,
        "course_lessons": dump_table("course_lessons")?,
        "achievement_unlocks": dump_table("achievement_unlocks")?,
        "activity_events": dump_table("activity_events")?,
        "tags": dump_table("tags")?,
        "project_tags": dump_table("project_tags")?,
        "project_versions": dump_table("project_versions")?,
    }))
}

pub fn local_user(conn: &Connection) -> Option<UserDto> {
    conn.query_row(
        "SELECT id, email, display_name FROM users WHERE deleted_at IS NULL LIMIT 1",
        [],
        |r| {
            Ok(UserDto {
                id: r.get(0)?,
                email: r.get(1)?,
                display_name: r.get(2)?,
            })
        },
    )
    .optional()
    .ok()
    .flatten()
}

pub fn complete_onboarding(conn: &Connection) -> AppResult<()> {
    set_setting(conn, "onboarding.complete", "true")
}

pub fn set_default_workspace(conn: &Connection, id: &str) -> AppResult<()> {
    set_setting(conn, "workspace.defaultCategoryId", id)
}

pub fn replace_project_tags(conn: &Connection, project_id: &str, tag_ids: &[String]) -> AppResult<()> {
    conn.execute("DELETE FROM project_tags WHERE project_id=?1", [project_id])?;
    let ts = now();
    for tag_id in tag_ids {
        conn.execute(
            "INSERT INTO project_tags (project_id, tag_id, created_at, updated_at, revision) VALUES (?1,?2,?3,?3,1)",
            params![project_id, tag_id, ts],
        )?;
    }
    evaluate_project(conn, project_id)?;
    Ok(())
}

pub fn project_context(conn: &Connection, project_id: &str) -> AppResult<ProjectContext> {
    let card = project_card(conn, project_id, &current_device(conn), "")?;
    let caps = caps_for(conn, &card.category_id)?;
    Ok(ProjectContext {
        project_id: card.id,
        category_id: card.category_id,
        kind: CategoryKind::parse(&card.kind).unwrap_or(CategoryKind::Generic),
        capabilities: caps,
        name: card.name,
        description: card.description,
        status: bluephoenix_domain::ids::ProjectStatus::parse(&card.status)
            .unwrap_or(bluephoenix_domain::ids::ProjectStatus::Active),
        tracked_seconds: card.total_tracked_seconds,
        xp: card.xp,
        open_todos: card.open_todos,
        github_url: card.github_url,
        website_url: card.website_url,
        current_version: card.current_version,
        final_grade: card.final_grade,
        cfu: card.cfu,
    })
}

pub fn store_secret_envelope(
    conn: &Connection,
    kind: &str,
    ciphertext: &str,
    nonce: &str,
    wrap_params: serde_json::Value,
) -> AppResult<String> {
    let ts = now();
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM encrypted_secrets WHERE kind=?1 AND deleted_at IS NULL",
            [kind],
            |r| r.get(0),
        )
        .optional()?;
    let id = existing.unwrap_or_else(new_id);
    conn.execute(
        "INSERT INTO encrypted_secrets (id, kind, ciphertext, nonce, wrap_params, revision, updated_at)
         VALUES (?1,?2,?3,?4,?5,1,?6)
         ON CONFLICT(id) DO UPDATE SET ciphertext=excluded.ciphertext, nonce=excluded.nonce,
           wrap_params=excluded.wrap_params, revision=revision+1, updated_at=excluded.updated_at, deleted_at=NULL",
        params![id, kind, ciphertext, nonce, wrap_params.to_string(), ts],
    )?;
    conn.execute(
        "INSERT INTO secret_sync_outbox (id, secret_id, payload, created_at) VALUES (?1,?2,?3,?4)",
        params![
            new_id(),
            id,
            json!({"id": id, "kind": kind, "ciphertext": ciphertext, "nonce": nonce, "wrapParams": wrap_params}).to_string(),
            ts
        ],
    )?;
    Ok(id)
}

pub fn bootstrap(conn: &Connection, device_id: &str) -> AppResult<BootstrapDto> {
    Ok(BootstrapDto {
        device_id: device_id.to_string(),
        settings: load_settings(conn),
        categories: list_categories(conn, true)?,
        tags: list_tags(conn)?,
        sync: sync_status(conn),
        onboarding_complete: setting(conn, "onboarding.complete", "false") == "true",
        user: local_user(conn),
    })
}

fn hydrate_search_hit(conn: &Connection, hit: &mut SearchHitDto) {
    match hit.entity_type.as_str() {
        "project" => {
            if let Ok(cat) = conn.query_row(
                "SELECT category_id FROM projects WHERE id=?1",
                [&hit.entity_id],
                |r| r.get::<_, String>(0),
            ) {
                hit.category_id = Some(cat);
                hit.project_id = Some(hit.entity_id.clone());
            }
        }
        "todo" | "command" | "link" => {
            let sql = match hit.entity_type.as_str() {
                "todo" => "SELECT t.project_id, p.category_id FROM todos t JOIN projects p ON p.id=t.project_id WHERE t.id=?1",
                "command" => "SELECT c.project_id, p.category_id FROM commands c JOIN projects p ON p.id=c.project_id WHERE c.id=?1",
                _ => "SELECT l.project_id, p.category_id FROM project_links l JOIN projects p ON p.id=l.project_id WHERE l.id=?1",
            };
            if let Ok((pid, cid)) = conn.query_row(sql, [&hit.entity_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            }) {
                hit.project_id = Some(pid);
                hit.category_id = Some(cid);
            }
        }
        _ => {}
    }
}

pub fn import_all(conn: &Connection, data: serde_json::Value) -> AppResult<()> {
    for table in bluephoenix_sync_protocol::METADATA_TABLES {
        let Some(rows) = data.get(*table).and_then(|v| v.as_array()) else {
            continue;
        };
        for row in rows {
            insert_portable_row(conn, table, row)?;
        }
    }
    Ok(())
}

fn insert_portable_row(conn: &Connection, table: &str, row: &serde_json::Value) -> AppResult<()> {
    let Some(obj) = row.as_object() else {
        return Ok(());
    };
    let keys: Vec<String> = obj
        .keys()
        .filter(|k| k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .cloned()
        .collect();
    if keys.is_empty() {
        return Ok(());
    }
    let cols = keys.join(",");
    let placeholders = keys.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("INSERT OR REPLACE INTO {table} ({cols}) VALUES ({placeholders})");
    let values: Vec<Option<String>> = keys
        .iter()
        .map(|k| match &obj[k] {
            serde_json::Value::Null => None,
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            serde_json::Value::Bool(b) => Some(if *b { "1".into() } else { "0".into() }),
            other => Some(other.to_string()),
        })
        .collect();
    conn.execute(&sql, rusqlite::params_from_iter(values.iter().map(|v| v.as_deref())))?;
    Ok(())
}

pub fn apply_pull_changes(conn: &Connection, changes: &[serde_json::Value]) -> AppResult<()> {
    for change in changes {
        let Some(table) = change.get("table").and_then(|v| v.as_str()) else {
            continue;
        };
        if !bluephoenix_sync_protocol::METADATA_TABLES.contains(&table) {
            continue;
        }
        let mut payload = change.get("payload").cloned().unwrap_or_else(|| json!({}));
        if payload.get("id").is_none() {
            if let Some(row_id) = change.get("rowId").or(change.get("row_id")).and_then(|v| v.as_str()) {
                if let Some(obj) = payload.as_object_mut() {
                    obj.insert("id".into(), json!(row_id));
                }
            }
        }
        let remote_rev = change.get("revision").and_then(|v| v.as_i64()).unwrap_or(1);
        let row_id = payload
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !row_id.is_empty() {
            let local_rev: Option<i64> = conn
                .query_row(
                    &format!("SELECT revision FROM {table} WHERE id=?1"),
                    [&row_id],
                    |r| r.get(0),
                )
                .optional()
                .ok()
                .flatten();
            if let Some(local) = local_rev {
                if local > remote_rev {
                    continue;
                }
            }
        }
        insert_portable_row(conn, table, &payload)?;
    }
    Ok(())
}

pub fn set_sync_cursor(
    conn: &Connection,
    collection: &str,
    cursor: Option<&str>,
    error: Option<&str>,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO sync_state (collection, cursor, last_pulled_at, last_error)
         VALUES (?1,?2,?3,?4)
         ON CONFLICT(collection) DO UPDATE SET cursor=excluded.cursor, last_pulled_at=excluded.last_pulled_at, last_error=excluded.last_error",
        params![collection, cursor, now(), error],
    )?;
    Ok(())
}

pub fn sync_cursor(conn: &Connection, collection: &str) -> Option<String> {
    conn.query_row(
        "SELECT cursor FROM sync_state WHERE collection=?1",
        [collection],
        |r| r.get(0),
    )
    .optional()
    .ok()
    .flatten()
}

pub fn apply_secret_envelopes(conn: &Connection, secrets: &[serde_json::Value]) -> AppResult<()> {
    for secret in secrets {
        let Some(id) = secret.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let kind = secret.get("kind").and_then(|v| v.as_str()).unwrap_or("unknown");
        let ciphertext = secret.get("ciphertext").and_then(|v| v.as_str()).unwrap_or("");
        let nonce = secret.get("nonce").and_then(|v| v.as_str()).unwrap_or("");
        let wrap = secret.get("wrapParams").cloned().unwrap_or(json!({}));
        let rev = secret.get("revision").and_then(|v| v.as_i64()).unwrap_or(1);
        conn.execute(
            "INSERT INTO encrypted_secrets (id, kind, ciphertext, nonce, wrap_params, revision, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(id) DO UPDATE SET ciphertext=excluded.ciphertext, nonce=excluded.nonce,
               wrap_params=excluded.wrap_params, revision=excluded.revision, updated_at=excluded.updated_at",
            params![id, kind, ciphertext, nonce, wrap.to_string(), rev, now()],
        )?;
    }
    Ok(())
}

impl Db {
    #[allow(dead_code)]
    pub fn conn_op<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        self.with(f)
    }
}

