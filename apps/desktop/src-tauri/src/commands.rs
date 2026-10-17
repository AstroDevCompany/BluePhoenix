use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::state::AppState;
use bluephoenix_domain::commands_safety::{is_dangerous, split_command_line};
use bluephoenix_domain::context::TodoFilter;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub fn bootstrap(state: State<AppState>) -> AppResult<BootstrapDto> {
    state.db.with(|c| db::bootstrap(c, &state.db.device_id))
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> AppResult<AppSettingsDto> {
    state.db.with(|c| Ok(db::load_settings(c)))
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<AppState>,
    settings: AppSettingsDto,
) -> AppResult<AppSettingsDto> {
    state.db.with(|c| db::save_settings(c, &settings))?;
    apply_launch_at_startup(&app, settings.launch_at_startup);
    state.db.with(|c| Ok(db::load_settings(c)))
}

pub fn sync_launch_at_startup(app: &AppHandle) {
    let enabled = app
        .try_state::<AppState>()
        .and_then(|state| {
            state
                .db
                .with(|c| Ok(db::load_settings(c).launch_at_startup))
                .ok()
        })
        .unwrap_or(false);
    apply_launch_at_startup(app, enabled);
}

fn apply_launch_at_startup(app: &AppHandle, enabled: bool) {
    #[cfg(desktop)]
    {
        use tauri_plugin_autostart::ManagerExt;
        let launcher = app.autolaunch();
        let _ = if enabled {
            launcher.enable()
        } else {
            launcher.disable()
        };
    }
    let _ = (app, enabled);
}

#[tauri::command]
pub fn list_categories(
    state: State<AppState>,
    include_disabled: Option<bool>,
) -> AppResult<Vec<CategoryDto>> {
    state
        .db
        .with(|c| db::list_categories(c, include_disabled.unwrap_or(false)))
}

#[tauri::command]
pub fn set_category_enabled(
    state: State<AppState>,
    id: String,
    enabled: bool,
) -> AppResult<CategoryDto> {
    state.db.with(|c| db::set_category_enabled(c, &id, enabled))
}

#[tauri::command]
pub fn create_custom_category(
    state: State<AppState>,
    name: String,
    icon: String,
    accent: String,
) -> AppResult<CategoryDto> {
    state
        .db
        .with(|c| db::create_custom_category(c, &name, &icon, &accent))
}

#[tauri::command]
pub fn list_tags(state: State<AppState>) -> AppResult<Vec<TagDto>> {
    state.db.with(db::list_tags)
}

#[tauri::command]
pub fn create_custom_tag(
    state: State<AppState>,
    name: String,
    kind: Option<String>,
) -> AppResult<TagDto> {
    state
        .db
        .with(|c| db::create_custom_tag(c, &name, kind.as_deref().unwrap_or("custom")))
}

#[tauri::command]
pub fn list_projects(
    state: State<AppState>,
    category_id: String,
    include_archived: Option<bool>,
) -> AppResult<Vec<ProjectCardDto>> {
    state.db.with(|c| {
        db::list_projects(
            c,
            &category_id,
            &state.db.device_id,
            include_archived.unwrap_or(false),
        )
    })
}

#[tauri::command]
pub fn get_project(state: State<AppState>, id: String) -> AppResult<ProjectCardDto> {
    state
        .db
        .with(|c| db::project_card(c, &id, &state.db.device_id))
}

#[tauri::command]
pub fn create_project(
    state: State<AppState>,
    input: CreateProjectInput,
) -> AppResult<ProjectCardDto> {
    state
        .db
        .with(|c| db::create_project(c, &state.db.device_id, input))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectInput {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub favorite: Option<bool>,
    pub archived: Option<bool>,
    pub github_url: Option<String>,
    pub website_url: Option<String>,
    pub current_version: Option<String>,
    pub university: Option<String>,
    pub professor: Option<String>,
    pub academic_year: Option<String>,
    pub semester: Option<String>,
    pub cfu: Option<f64>,
    pub final_grade: Option<f64>,
    pub honors: Option<bool>,
    pub completed: Option<bool>,
    pub local_path: Option<String>,
    pub primary_file_path: Option<String>,
    pub tag_ids: Option<Vec<String>>,
}

#[tauri::command]
pub fn update_project(
    state: State<AppState>,
    input: UpdateProjectInput,
) -> AppResult<ProjectCardDto> {
    state.db.with(|c| {
        db::update_project_fields(
            c,
            &input.id,
            input.name,
            input.description,
            input.status,
            input.favorite,
            input.archived,
            input.github_url,
            input.website_url,
            input.current_version,
            input.university,
            input.professor,
            input.academic_year,
            input.semester,
            input.cfu,
            input.final_grade,
            input.honors,
            input.completed,
        )?;
        if input.local_path.is_some() || input.primary_file_path.is_some() {
            db::update_binding(
                c,
                &state.db.device_id,
                &input.id,
                input.local_path,
                input.primary_file_path,
            )?;
        }
        if let Some(tags) = input.tag_ids {
            db::replace_project_tags(c, &input.id, &tags)?;
        }
        db::project_card(c, &input.id, &state.db.device_id)
    })
}

#[tauri::command]
pub fn delete_project(state: State<AppState>, id: String) -> AppResult<()> {
    state
        .db
        .with(|c| db::delete_project(c, &id, &state.db.device_id))
}

#[tauri::command]
pub fn running_timer(state: State<AppState>) -> AppResult<Option<RunningTimerDto>> {
    state.db.with(|c| db::running_timer(c, &state.db.device_id))
}

#[tauri::command]
pub fn start_timer(
    state: State<AppState>,
    project_id: String,
    topic_id: Option<String>,
) -> AppResult<RunningTimerDto> {
    state
        .db
        .with(|c| db::start_timer(c, &state.db.device_id, &project_id, topic_id))
}

#[tauri::command]
pub fn stop_timer(state: State<AppState>) -> AppResult<i64> {
    state.db.with(|c| db::stop_timer(c, &state.db.device_id))
}

#[tauri::command]
pub fn pause_timer(state: State<AppState>) -> AppResult<i64> {
    state.db.with(|c| db::pause_timer(c, &state.db.device_id))
}

#[tauri::command]
pub fn add_manual_entry(
    state: State<AppState>,
    project_id: String,
    started_at: String,
    ended_at: String,
    topic_id: Option<String>,
    notes: Option<String>,
) -> AppResult<()> {
    state.db.with(|c| {
        db::add_manual_entry(
            c,
            &state.db.device_id,
            &project_id,
            started_at,
            ended_at,
            topic_id,
            notes,
        )
    })
}

#[tauri::command]
pub fn update_time_entry(
    state: State<AppState>,
    id: String,
    started_at: Option<String>,
    ended_at: Option<String>,
    notes: Option<String>,
    topic_id: Option<String>,
) -> AppResult<()> {
    state
        .db
        .with(|c| db::update_time_entry(c, &id, started_at, ended_at, notes, topic_id))
}

#[tauri::command]
pub fn delete_time_entry(state: State<AppState>, id: String) -> AppResult<()> {
    state.db.with(|c| db::delete_time_entry(c, &id))
}

#[tauri::command]
pub fn list_time_entries(
    state: State<AppState>,
    project_id: String,
) -> AppResult<Vec<TimeEntryDto>> {
    state.db.with(|c| db::list_time_entries(c, &project_id))
}

#[tauri::command]
pub fn list_todos(
    state: State<AppState>,
    project_id: Option<String>,
    category_id: Option<String>,
    status: Option<String>,
    kind_id: Option<String>,
    query: Option<String>,
) -> AppResult<Vec<TodoDto>> {
    state.db.with(|c| {
        db::list_todos(
            c,
            TodoFilter {
                project_id,
                category_id,
                status,
                kind_id,
                query,
            },
        )
    })
}

#[tauri::command]
pub fn create_todo(
    state: State<AppState>,
    project_id: String,
    title: String,
    description: Option<String>,
    kind_id: Option<String>,
    priority: Option<String>,
    due_date: Option<String>,
) -> AppResult<TodoDto> {
    state.db.with(|c| {
        db::create_todo(
            c,
            &project_id,
            &title,
            description,
            kind_id,
            priority,
            due_date,
        )
    })
}

#[tauri::command]
pub fn set_todo_status(state: State<AppState>, id: String, status: String) -> AppResult<()> {
    state.db.with(|c| db::set_todo_status(c, &id, &status))
}

#[tauri::command]
pub fn delete_todo(state: State<AppState>, id: String) -> AppResult<()> {
    state.db.with(|c| db::delete_todo(c, &id))
}

#[tauri::command]
pub fn todo_kinds(
    state: State<AppState>,
    category_id: String,
) -> AppResult<Vec<(String, String, String)>> {
    state.db.with(|c| db::todo_kinds(c, &category_id))
}

#[tauri::command]
pub fn list_links(state: State<AppState>, project_id: String) -> AppResult<Vec<LinkDto>> {
    state.db.with(|c| db::list_links(c, &project_id))
}

#[tauri::command]
pub fn upsert_link(state: State<AppState>, link: LinkDto) -> AppResult<LinkDto> {
    state.db.with(|c| db::upsert_link(c, link))
}

#[tauri::command]
pub fn delete_link(state: State<AppState>, id: String) -> AppResult<()> {
    state.db.with(|c| db::delete_link(c, &id))
}

#[tauri::command]
pub fn list_project_files(
    state: State<AppState>,
    project_id: String,
) -> AppResult<Vec<ProjectFileDto>> {
    state
        .db
        .with(|c| db::list_project_files(c, &project_id, &state.db.device_id))
}

#[tauri::command]
pub fn add_project_file(
    state: State<AppState>,
    project_id: String,
    display_name: String,
    absolute_path: String,
    file_kind: Option<String>,
) -> AppResult<ProjectFileDto> {
    state.db.with(|c| {
        let file = db::add_project_file(
            c,
            &state.db.device_id,
            &project_id,
            &display_name,
            &absolute_path,
            file_kind,
        )?;
        if db::project_kind(c, &project_id)? != bluephoenix_domain::ids::CategoryKind::Software {
            crate::ai_store::upsert_document_record(
                c,
                &file.id,
                &project_id,
                "pending",
                "Queued",
                None,
                None,
            )?;
            let _ = db::enqueue_job(
                c,
                "index_documents",
                serde_json::json!({"fileId": file.id, "projectId": project_id}),
            );
        }
        Ok(file)
    })
}

#[tauri::command]
pub fn remove_project_file(state: State<AppState>, id: String) -> AppResult<()> {
    state.db.with(|c| db::remove_project_file(c, &id))
}

#[tauri::command]
pub fn list_folder(path: String) -> AppResult<Vec<FileEntryDto>> {
    crate::native::list_dir_shallow(std::path::Path::new(&path))
}

#[tauri::command]
pub fn scan_project_folder(
    state: State<AppState>,
    project_id: String,
) -> AppResult<Vec<FileEntryDto>> {
    let path = state.db.with(|c| {
        let (local, _) = db::project_binding(c, &state.db.device_id, &project_id);
        Ok(local)
    })?;
    let Some(path) = path else {
        return Ok(vec![]);
    };
    let entries = crate::native::list_dir_shallow(std::path::Path::new(&path))?;
    let cloned = entries.clone();
    state.db.with(|c| {
        db::cache_files(c, &project_id, &cloned)?;
        let _ = db::enqueue_job(
            c,
            "index_files",
            serde_json::json!({"projectId": project_id}),
        );
        Ok(())
    })?;
    Ok(entries)
}

#[tauri::command]
pub fn cached_files(state: State<AppState>, project_id: String) -> AppResult<Vec<FileEntryDto>> {
    state.db.with(|c| db::cached_files(c, &project_id))
}

#[tauri::command]
pub fn open_path(path: String) -> AppResult<()> {
    crate::native::open_path(std::path::Path::new(&path))
}

#[tauri::command]
pub fn reveal_path(path: String) -> AppResult<()> {
    crate::native::reveal_path(std::path::Path::new(&path))
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> AppResult<()> {
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| AppError::msg(e.to_string()))
}

#[tauri::command]
pub fn list_editors() -> Vec<crate::native::LaunchOption> {
    crate::native::list_editors()
}

#[tauri::command]
pub fn list_terminals() -> Vec<crate::native::LaunchOption> {
    crate::native::list_terminals()
}

/// Opens a folder in an editor: the given id, else the saved preference, else the first one installed.
#[tauri::command]
pub fn open_in_editor(
    state: State<AppState>,
    folder: String,
    editor: Option<String>,
) -> AppResult<()> {
    let preferred = state
        .db
        .with(|c| Ok(db::load_settings(c).preferred_editor))?;
    let editor = editor.or(Some(preferred));
    crate::native::open_in_editor(editor.as_deref(), std::path::Path::new(&folder))
}

#[tauri::command]
pub fn open_vscode(state: State<AppState>, folder: String) -> AppResult<()> {
    open_in_editor(state, folder, Some("vscode".into()))
}

/// Opens the system terminal in a folder, optionally running a command (dangerous ones need confirmation).
#[tauri::command]
pub fn open_terminal(
    state: State<AppState>,
    folder: String,
    command: Option<String>,
    terminal: Option<String>,
    confirmed: Option<bool>,
) -> AppResult<()> {
    if let Some(cmd) = &command {
        if is_dangerous(cmd) && confirmed != Some(true) {
            return Err(AppError::NeedsConfirm);
        }
    }
    let preferred = match terminal {
        Some(t) => t,
        None => state.db.with(|c| Ok(db::load_settings(c).terminal))?,
    };
    crate::native::open_terminal(
        &preferred,
        std::path::Path::new(&folder),
        command.as_deref(),
    )
}

#[tauri::command]
pub fn list_commands(state: State<AppState>, project_id: String) -> AppResult<Vec<CommandDto>> {
    state.db.with(|c| db::list_commands(c, &project_id))
}

#[tauri::command]
pub fn upsert_command(state: State<AppState>, command: CommandDto) -> AppResult<CommandDto> {
    state.db.with(|c| db::upsert_command(c, command))
}

#[tauri::command]
pub fn delete_command(state: State<AppState>, id: String) -> AppResult<()> {
    state.db.with(|c| db::delete_command(c, &id))
}

#[tauri::command]
pub fn run_command(
    state: State<AppState>,
    project_id: String,
    command: String,
    working_directory: Option<String>,
    confirmed: Option<bool>,
) -> AppResult<CommandOutputDto> {
    if is_dangerous(&command) && confirmed != Some(true) {
        return Err(AppError::NeedsConfirm);
    }
    let args = split_command_line(&command).map_err(AppError::msg)?;
    let program = args[0].clone();
    let rest = &args[1..];
    let cwd = if let Some(dir) = working_directory {
        PathBuf::from(dir)
    } else {
        state.db.with(|c| {
            let (local, _) = db::project_binding(c, &state.db.device_id, &project_id);
            Ok(local
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(".")))
        })?
    };
    let output = Command::new(&program)
        .args(rest)
        .current_dir(&cwd)
        .output()
        .map_err(|e| AppError::msg(format!("Unable to run command: {e}")))?;
    Ok(CommandOutputDto {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn term_spawn(
    app: AppHandle,
    state: State<AppState>,
    project_id: Option<String>,
    command_id: Option<String>,
    command: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
    confirmed: Option<bool>,
) -> AppResult<crate::terminal::TerminalSessionDto> {
    // Resolve everything from the DB first so the lock is released before spawning.
    let (project_id, title, command, working_dir, env, root) = state.db.with(|c| {
        let mut project_id = project_id;
        let mut title = None;
        let mut command = command.filter(|c| !c.trim().is_empty());
        let mut working_dir = None;
        let mut env = Vec::new();
        if let Some(cid) = command_id {
            let row = c
                .query_row(
                    "SELECT project_id, name, command, working_directory, env_json FROM commands WHERE id=?1 AND deleted_at IS NULL",
                    [&cid],
                    |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                            r.get::<_, Option<String>>(3)?,
                            r.get::<_, Option<String>>(4)?,
                        ))
                    },
                )
                .map_err(|_| AppError::msg("Command not found"))?;
            project_id = Some(row.0);
            title = Some(row.1);
            command = Some(row.2);
            working_dir = row.3;
            if let Some(raw) = row.4 {
                if let Ok(map) = serde_json::from_str::<std::collections::HashMap<String, String>>(&raw) {
                    env = map.into_iter().collect();
                }
            }
        }
        let root = match &project_id {
            Some(pid) => db::project_binding(c, &state.db.device_id, pid)
                .0
                .filter(|p| !p.trim().is_empty())
                .map(PathBuf::from),
            None => None,
        };
        Ok((project_id, title, command, working_dir, env, root))
    })?;
    if let Some(cmd) = &command {
        if is_dangerous(cmd) && confirmed != Some(true) {
            return Err(AppError::NeedsConfirm);
        }
    }
    let cwd = match root {
        Some(root) => crate::terminal::resolve_cwd(&root, working_dir.as_deref()),
        None if project_id.is_some() => return Err(AppError::FolderMissing),
        None => dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
    };
    let title = title.unwrap_or_else(|| command.clone().unwrap_or_else(|| "Terminal".to_string()));
    state.terminals.spawn(
        crate::terminal::app_sink(&app),
        crate::terminal::SpawnRequest {
            project_id,
            title,
            command,
            cwd,
            env,
            cols: cols.unwrap_or(100),
            rows: rows.unwrap_or(28),
        },
    )
}

#[tauri::command]
pub fn term_write(state: State<AppState>, id: String, data: String) -> AppResult<()> {
    state.terminals.write(&id, &data)
}

#[tauri::command]
pub fn term_resize(state: State<AppState>, id: String, cols: u16, rows: u16) -> AppResult<()> {
    state.terminals.resize(&id, cols, rows)
}

#[tauri::command]
pub fn term_kill(state: State<AppState>, id: String) -> AppResult<()> {
    state.terminals.kill(&id)
}

#[tauri::command]
pub fn term_close(state: State<AppState>, id: String) -> AppResult<()> {
    state.terminals.close(&id)
}

#[tauri::command]
pub fn term_list(state: State<AppState>) -> Vec<crate::terminal::TerminalSessionDto> {
    state.terminals.list()
}

#[tauri::command]
pub fn term_backlog(state: State<AppState>, id: String) -> AppResult<String> {
    state.terminals.backlog(&id)
}

/// Looks up the repo folder (and git binary setting) without holding the DB lock afterwards.
fn repo_target(state: &AppState, project_id: &str) -> AppResult<(String, std::path::PathBuf)> {
    state.db.with(|c| {
        let git = db::load_settings(c).git_path;
        let (local, _) = db::project_binding(c, &state.db.device_id, project_id);
        let path = local
            .filter(|p| !p.trim().is_empty())
            .ok_or(AppError::FolderMissing)?;
        Ok((git, std::path::PathBuf::from(path)))
    })
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::msg(e.to_string()))?
}

#[tauri::command]
pub async fn git_status(state: State<'_, AppState>, project_id: String) -> AppResult<GitStatusDto> {
    let (git, path) = match repo_target(&state, &project_id) {
        Ok(v) => v,
        Err(_) => {
            return Ok(GitStatusDto {
                available: true,
                error: Some("No local folder configured".into()),
                ..GitStatusDto::default()
            })
        }
    };
    blocking(move || Ok(crate::git_cache::refresh_now(&git, &path))).await
}

/// Refreshes the cached status for the given projects (or all tracked repos when empty).
#[tauri::command]
pub async fn git_refresh(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    project_ids: Option<Vec<String>>,
) -> AppResult<Vec<GitStatusUpdate>> {
    let only = project_ids.filter(|ids| !ids.is_empty());
    let (git, targets) = state.db.with(|c| {
        Ok((
            db::load_settings(c).git_path,
            crate::git_cache::tracked_targets(c, &state.db.device_id, only.as_deref()),
        ))
    })?;
    blocking(move || Ok(crate::git_cache::refresh_many(&app, &git, targets))).await
}

#[tauri::command]
pub async fn git_run(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    action: String,
    arg: Option<String>,
) -> AppResult<String> {
    let (git, path) = repo_target(&state, &project_id)?;
    let git2 = git.clone();
    let path2 = path.clone();
    let out = blocking(move || run_git_action(&git2, &path2, &action, arg.as_deref())).await;
    // Refresh even on failure: a conflicted pull changes the repo state.
    let id = project_id.clone();
    let status = blocking(move || Ok(crate::git_cache::refresh_now(&git, &path))).await?;
    let _ = tauri::Emitter::emit(
        &app,
        crate::git_cache::STATUS_EVENT,
        GitStatusUpdate {
            project_id: id,
            git: status,
        },
    );
    let out = out?;
    state.db.with(|c| db::evaluate_project(c, &project_id))?;
    Ok(out)
}

fn run_git_action(
    git: &str,
    path: &std::path::Path,
    action: &str,
    arg: Option<&str>,
) -> AppResult<String> {
    use crate::git::{has_remote, run_git};
    match action {
        "fetch" => run_git(git, path, &["fetch", "--prune"]),
        "pull" => run_git(git, path, &["pull"]),
        "push" => {
            let status = crate::git::status(git, path);
            if status.upstream.is_none()
                && status.branch.is_some()
                && has_remote(git, path, "origin")
            {
                run_git(git, path, &["push", "--set-upstream", "origin", "HEAD"])
            } else {
                run_git(git, path, &["push"])
            }
        }
        "checkout" => {
            let branch = arg
                .map(str::trim)
                .filter(|b| !b.is_empty() && !b.starts_with('-'));
            let branch = branch.ok_or_else(|| AppError::msg("Choose a branch"))?;
            run_git(git, path, &["checkout", branch])
        }
        "stash" => run_git(
            git,
            path,
            &[
                "stash",
                "push",
                "--include-untracked",
                "-m",
                "BluePhoenix stash",
            ],
        ),
        "stash_pop" => run_git(git, path, &["stash", "pop"]),
        _ => Err(AppError::msg("Unknown git action")),
    }
}

/// Fetches every tracked repo (or the given projects), reporting a result for each.
#[tauri::command]
pub async fn git_fetch_all(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    project_ids: Option<Vec<String>>,
) -> AppResult<Vec<GitOpResult>> {
    let only = project_ids.filter(|ids| !ids.is_empty());
    let (git, targets) = state.db.with(|c| {
        Ok((
            db::load_settings(c).git_path,
            crate::git_cache::tracked_targets(c, &state.db.device_id, only.as_deref()),
        ))
    })?;
    blocking(move || {
        let queue = std::sync::Mutex::new(targets.into_iter());
        let results = std::sync::Mutex::new(Vec::new());
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(|| loop {
                    let next = queue.lock().ok().and_then(|mut q| q.next());
                    let Some((project_id, path)) = next else {
                        break;
                    };
                    let outcome = run_git_action(&git, &path, "fetch", None);
                    let status = crate::git_cache::refresh_now(&git, &path);
                    let _ = tauri::Emitter::emit(
                        &app,
                        crate::git_cache::STATUS_EVENT,
                        GitStatusUpdate {
                            project_id: project_id.clone(),
                            git: status,
                        },
                    );
                    let (ok, message) = match outcome {
                        Ok(_) => (true, "Fetched".to_string()),
                        Err(e) => (false, e.to_string()),
                    };
                    if let Ok(mut r) = results.lock() {
                        r.push(GitOpResult {
                            project_id,
                            ok,
                            message,
                        });
                    }
                });
            }
        });
        Ok(results.into_inner().unwrap_or_default())
    })
    .await
}

#[tauri::command]
pub async fn git_branches(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<Vec<GitBranchDto>> {
    let (git, path) = repo_target(&state, &project_id)?;
    blocking(move || crate::git::branches(&git, &path)).await
}

#[tauri::command]
pub async fn git_changes(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<Vec<GitChangeDto>> {
    let (git, path) = repo_target(&state, &project_id)?;
    blocking(move || crate::git::changes(&git, &path)).await
}

#[tauri::command]
pub async fn git_log(
    state: State<'_, AppState>,
    project_id: String,
    limit: Option<usize>,
) -> AppResult<Vec<GitCommitDto>> {
    let (git, path) = repo_target(&state, &project_id)?;
    blocking(move || crate::git::log(&git, &path, limit.unwrap_or(30))).await
}

#[tauri::command]
pub fn list_versions(state: State<AppState>, project_id: String) -> AppResult<Vec<VersionDto>> {
    state.db.with(|c| db::list_versions(c, &project_id))
}

#[tauri::command]
pub fn add_version(
    state: State<AppState>,
    project_id: String,
    version: String,
    title: Option<String>,
    changelog: Option<String>,
) -> AppResult<VersionDto> {
    state
        .db
        .with(|c| db::add_version(c, &project_id, &version, title, changelog))
}

#[tauri::command]
pub fn list_topics(state: State<AppState>, project_id: String) -> AppResult<Vec<TopicDto>> {
    state.db.with(|c| db::list_topics(c, &project_id))
}

#[tauri::command]
pub fn upsert_topic(state: State<AppState>, topic: TopicDto) -> AppResult<TopicDto> {
    state.db.with(|c| db::upsert_topic(c, topic))
}

#[tauri::command]
pub fn delete_topic(state: State<AppState>, id: String) -> AppResult<()> {
    state.db.with(|c| db::delete_topic(c, &id))
}

#[tauri::command]
pub fn list_lessons(state: State<AppState>, project_id: String) -> AppResult<Vec<LessonDto>> {
    state.db.with(|c| db::list_lessons(c, &project_id))
}

#[tauri::command]
pub fn upsert_lesson(state: State<AppState>, lesson: LessonDto) -> AppResult<LessonDto> {
    state.db.with(|c| db::upsert_lesson(c, lesson))
}

#[tauri::command]
pub fn delete_lesson(state: State<AppState>, id: String) -> AppResult<()> {
    state.db.with(|c| db::delete_lesson(c, &id))
}

#[tauri::command]
pub fn list_exams(
    state: State<AppState>,
    project_id: Option<String>,
    category_id: Option<String>,
) -> AppResult<Vec<ExamDto>> {
    state
        .db
        .with(|c| db::list_exams(c, project_id.as_deref(), category_id.as_deref()))
}

#[tauri::command]
pub fn upsert_exam(state: State<AppState>, exam: ExamDto) -> AppResult<ExamDto> {
    state.db.with(|c| db::upsert_exam(c, exam))
}

#[tauri::command]
pub fn delete_exam(state: State<AppState>, id: String) -> AppResult<()> {
    state.db.with(|c| db::delete_exam(c, &id))
}

#[tauri::command]
pub fn university_dashboard(state: State<AppState>) -> AppResult<GpaDashboardDto> {
    state.db.with(db::university_dashboard)
}

#[tauri::command]
pub fn list_activity(
    state: State<AppState>,
    category_id: Option<String>,
    project_id: Option<String>,
) -> AppResult<Vec<ActivityDto>> {
    state
        .db
        .with(|c| db::list_activity(c, category_id.as_deref(), project_id.as_deref()))
}

#[tauri::command]
pub fn list_achievements(
    state: State<AppState>,
    project_id: Option<String>,
) -> AppResult<Vec<UnlockDto>> {
    state
        .db
        .with(|c| Ok(db::list_unlocks(c, project_id.as_deref())))
}

#[tauri::command]
pub fn achievement_catalog() -> serde_json::Value {
    db::all_definitions()
}

#[tauri::command]
pub fn global_progress(state: State<AppState>) -> AppResult<GlobalProgressDto> {
    state.db.with(db::global_progress)
}

#[tauri::command]
pub fn search(state: State<AppState>, query: String) -> AppResult<Vec<SearchHitDto>> {
    state.db.with(|c| db::search(c, &query))
}

#[tauri::command]
pub fn project_context(
    state: State<AppState>,
    project_id: String,
) -> AppResult<bluephoenix_domain::context::ProjectContext> {
    state.db.with(|c| db::project_context(c, &project_id))
}

#[tauri::command]
pub fn export_data(state: State<AppState>) -> AppResult<serde_json::Value> {
    state.db.with(db::export_all)
}

#[tauri::command]
pub fn import_data(state: State<AppState>, data: serde_json::Value) -> AppResult<()> {
    state.db.with(|c| db::import_all(c, data))
}

#[tauri::command]
pub fn complete_onboarding(
    state: State<AppState>,
    default_workspace: Option<String>,
) -> AppResult<()> {
    state.db.with(|c| {
        if let Some(id) = default_workspace {
            db::set_default_workspace(c, &id)?;
        }
        db::complete_onboarding(c)
    })
}

#[tauri::command]
pub fn sync_status(state: State<AppState>) -> AppResult<SyncStatusDto> {
    state.db.with(|c| Ok(db::sync_status(c)))
}

#[tauri::command]
pub async fn check_for_updates() -> AppResult<UpdateCheckDto> {
    let local = crate::updater::local_version();
    Ok(crate::updater::check_raw(crate::updater::VERSION_CHECK_URL, &local).await)
}

#[tauri::command]
pub fn local_version() -> String {
    crate::updater::local_version()
}

fn file_path_to_string(path: tauri_plugin_dialog::FilePath) -> Option<String> {
    path.into_path()
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> AppResult<Option<String>> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Choose a folder")
        .pick_folder(move |folder| {
            let _ = tx.send(folder);
        });
    Ok(rx.await.ok().flatten().and_then(file_path_to_string))
}

#[tauri::command]
pub async fn pick_file(app: AppHandle) -> AppResult<Option<String>> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Choose a file")
        .pick_file(move |file| {
            let _ = tx.send(file);
        });
    Ok(rx.await.ok().flatten().and_then(file_path_to_string))
}

#[tauri::command]
pub async fn pick_gguf_file(app: AppHandle) -> AppResult<Option<String>> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Choose a GGUF model")
        .add_filter("GGUF model", &["gguf"])
        .pick_file(move |file| {
            let _ = tx.send(file);
        });
    Ok(rx.await.ok().flatten().and_then(file_path_to_string))
}

#[tauri::command]
pub fn enqueue_index_job(state: State<AppState>, project_id: String) -> AppResult<String> {
    state.db.with(|c| {
        db::enqueue_job(
            c,
            "index_files",
            serde_json::json!({"projectId": project_id}),
        )
    })
}

#[tauri::command]
pub fn poll_jobs(state: State<AppState>) -> AppResult<serde_json::Value> {
    state.db.with(|c| {
        let pending: i64 = c.query_row(
            "SELECT COUNT(*) FROM background_jobs WHERE status IN ('queued','running')",
            [],
            |r| r.get(0),
        )?;
        Ok(serde_json::json!({ "pending": pending }))
    })
}

#[tauri::command]
pub fn store_encrypted_secret(
    state: State<AppState>,
    kind: String,
    ciphertext: String,
    nonce: String,
    wrap_params: serde_json::Value,
) -> AppResult<String> {
    state
        .db
        .with(|c| db::store_secret_envelope(c, &kind, &ciphertext, &nonce, wrap_params))
}

#[tauri::command]
pub async fn auth_register(
    state: State<'_, AppState>,
    email: String,
    password: String,
    display_name: Option<String>,
) -> AppResult<serde_json::Value> {
    crate::sync::register(&state, email, password, display_name).await
}

#[tauri::command]
pub async fn auth_login(
    state: State<'_, AppState>,
    email: String,
    password: String,
) -> AppResult<serde_json::Value> {
    crate::sync::login(&state, email, password).await
}

#[tauri::command]
pub fn auth_logout(state: State<AppState>) -> AppResult<()> {
    crate::sync::logout(&state)
}

#[tauri::command]
pub async fn auth_forgot(state: State<'_, AppState>, email: String) -> AppResult<()> {
    crate::sync::forgot(&state, email).await
}

#[tauri::command]
pub async fn push_sync(state: State<'_, AppState>) -> AppResult<SyncStatusDto> {
    crate::sync::push_and_pull(&state).await
}
