//! Embedded terminal sessions backed by a pseudo-terminal.
//!
//! Each session runs the user's login shell (so PATH and version managers behave
//! like in their own terminal). Output is streamed to the frontend as `term-data`
//! events and kept in a bounded backlog so a reloaded UI can re-attach.

use crate::error::{AppError, AppResult};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Sends a named event with a JSON payload to the UI (abstracted so tests can capture output).
pub type EventSink = Arc<dyn Fn(&'static str, serde_json::Value) + Send + Sync>;

pub fn app_sink(app: &AppHandle) -> EventSink {
    let app = app.clone();
    Arc::new(move |event, payload| {
        let _ = app.emit(event, payload);
    })
}

pub const DATA_EVENT: &str = "term-data";
pub const EXIT_EVENT: &str = "term-exit";
const MAX_SESSIONS: usize = 16;
const BACKLOG_BYTES: usize = 256 * 1024;
const NO_EXIT_CODE: i64 = i64::MIN;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSessionDto {
    pub id: String,
    pub project_id: Option<String>,
    pub command_id: Option<String>,
    pub title: String,
    pub command: Option<String>,
    pub cwd: String,
    pub running: bool,
    pub exit_code: Option<i64>,
    pub started_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DataEvent<'a> {
    id: &'a str,
    data: &'a str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExitEvent<'a> {
    id: &'a str,
    code: Option<i64>,
}

struct Session {
    info: TerminalSessionDto,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    pid: Option<u32>,
    running: Arc<AtomicBool>,
    exit_code: Arc<AtomicI64>,
    backlog: Arc<Mutex<Vec<u8>>>,
}

#[derive(Default)]
pub struct TerminalRegistry {
    sessions: Mutex<HashMap<String, Session>>,
}

pub struct SpawnRequest {
    pub project_id: Option<String>,
    pub command_id: Option<String>,
    pub title: String,
    pub command: Option<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
    pub cols: u16,
    pub rows: u16,
}

fn default_shell() -> (String, Vec<String>) {
    #[cfg(windows)]
    {
        let shell = std::env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".into());
        (shell, vec![])
    }
    #[cfg(not(windows))]
    {
        let shell = std::env::var("SHELL")
            .ok()
            .filter(|s| Path::new(s).exists())
            .unwrap_or_else(|| {
                if cfg!(target_os = "macos") {
                    "/bin/zsh".into()
                } else {
                    "/bin/sh".into()
                }
            });
        (shell, vec!["-l".into(), "-i".into()])
    }
}

/// Builds the program and arguments: an interactive login shell, optionally running one command.
fn shell_invocation(command: Option<&str>) -> (String, Vec<String>) {
    let (shell, mut args) = default_shell();
    if let Some(cmd) = command {
        #[cfg(windows)]
        {
            if shell.to_lowercase().contains("powershell") {
                args.extend(["-NoLogo".into(), "-Command".into(), cmd.to_string()]);
            } else {
                args.extend(["/C".into(), cmd.to_string()]);
            }
        }
        #[cfg(not(windows))]
        {
            args.extend(["-c".into(), cmd.to_string()]);
        }
    }
    (shell, args)
}

/// Resolves a command's working directory under the project root; escapes fall back to the root.
pub fn resolve_cwd(root: &Path, requested: Option<&str>) -> PathBuf {
    let Some(dir) = requested.map(str::trim).filter(|d| !d.is_empty()) else {
        return root.to_path_buf();
    };
    let candidate = {
        let p = Path::new(dir);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            root.join(p)
        }
    };
    let (Ok(root_c), Ok(cand_c)) = (root.canonicalize(), candidate.canonicalize()) else {
        return root.to_path_buf();
    };
    if cand_c.starts_with(&root_c) && cand_c.is_dir() {
        cand_c
    } else {
        root.to_path_buf()
    }
}

/// Splits a byte stream into valid UTF-8 text, holding back an incomplete trailing sequence.
fn drain_utf8(pending: &mut Vec<u8>) -> String {
    match std::str::from_utf8(pending) {
        Ok(s) => {
            let out = s.to_string();
            pending.clear();
            out
        }
        Err(e) => {
            let valid = e.valid_up_to();
            match e.error_len() {
                // Incomplete sequence at the end: keep it for the next read.
                None => {
                    let out = String::from_utf8_lossy(&pending[..valid]).to_string();
                    pending.drain(..valid);
                    out
                }
                // Genuinely invalid bytes: replace and move on.
                Some(_) => {
                    let out = String::from_utf8_lossy(pending).to_string();
                    pending.clear();
                    out
                }
            }
        }
    }
}

#[cfg(unix)]
fn signal_group(pid: u32, signal: i32) {
    // The pty child is a session leader, so its pid is also its process group id.
    unsafe {
        libc::kill(-(pid as i32), signal);
    }
}

impl TerminalRegistry {
    pub fn spawn(&self, sink: EventSink, req: SpawnRequest) -> AppResult<TerminalSessionDto> {
        {
            let sessions = self
                .sessions
                .lock()
                .map_err(|_| AppError::msg("Terminal busy"))?;
            let live = sessions
                .values()
                .filter(|s| s.running.load(Ordering::Relaxed))
                .count();
            if live >= MAX_SESSIONS {
                return Err(AppError::msg(
                    "Too many running terminals — close one before starting another",
                ));
            }
        }
        if !req.cwd.is_dir() {
            return Err(AppError::FolderMissing);
        }
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: req.rows.max(2),
                cols: req.cols.max(2),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| AppError::msg(format!("Unable to open terminal: {e}")))?;
        let (program, args) = shell_invocation(req.command.as_deref());
        let mut cmd = CommandBuilder::new(program);
        cmd.args(args);
        cmd.cwd(&req.cwd);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        for (k, v) in &req.env {
            cmd.env(k, v);
        }
        let mut child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| AppError::msg(format!("Unable to start shell: {e}")))?;
        drop(pair.slave);
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| AppError::msg(e.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| AppError::msg(e.to_string()))?;
        let killer = child.clone_killer();
        let pid = child.process_id();

        let id = bluephoenix_domain::ids::new_id();
        let running = Arc::new(AtomicBool::new(true));
        let exit_code = Arc::new(AtomicI64::new(NO_EXIT_CODE));
        let backlog = Arc::new(Mutex::new(Vec::<u8>::new()));
        let info = TerminalSessionDto {
            id: id.clone(),
            project_id: req.project_id,
            command_id: req.command_id,
            title: req.title,
            command: req.command,
            cwd: req.cwd.to_string_lossy().to_string(),
            running: true,
            exit_code: None,
            started_at: chrono::Utc::now().to_rfc3339(),
        };

        {
            let (sink, id, running, exit_code, backlog) = (
                sink.clone(),
                id.clone(),
                running.clone(),
                exit_code.clone(),
                backlog.clone(),
            );
            std::thread::spawn(move || {
                let mut buf = [0u8; 8192];
                let mut pending: Vec<u8> = Vec::new();
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if let Ok(mut b) = backlog.lock() {
                                b.extend_from_slice(&buf[..n]);
                                if b.len() > BACKLOG_BYTES {
                                    let excess = b.len() - BACKLOG_BYTES;
                                    b.drain(..excess);
                                }
                            }
                            pending.extend_from_slice(&buf[..n]);
                            let text = drain_utf8(&mut pending);
                            if !text.is_empty() {
                                sink(
                                    DATA_EVENT,
                                    serde_json::json!(DataEvent {
                                        id: &id,
                                        data: &text
                                    }),
                                );
                            }
                        }
                    }
                }
                let code = child.wait().ok().map(|s| s.exit_code() as i64);
                exit_code.store(code.unwrap_or(NO_EXIT_CODE), Ordering::Relaxed);
                running.store(false, Ordering::Relaxed);
                sink(EXIT_EVENT, serde_json::json!(ExitEvent { id: &id, code }));
            });
        }

        let dto = info.clone();
        self.sessions
            .lock()
            .map_err(|_| AppError::msg("Terminal busy"))?
            .insert(
                id,
                Session {
                    info,
                    master: pair.master,
                    writer,
                    killer,
                    pid,
                    running,
                    exit_code,
                    backlog,
                },
            );
        Ok(dto)
    }

    fn with_session<T>(
        &self,
        id: &str,
        f: impl FnOnce(&mut Session) -> AppResult<T>,
    ) -> AppResult<T> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| AppError::msg("Terminal busy"))?;
        let session = sessions
            .get_mut(id)
            .ok_or_else(|| AppError::msg("Terminal session not found"))?;
        f(session)
    }

    pub fn write(&self, id: &str, data: &str) -> AppResult<()> {
        self.with_session(id, |s| {
            if !s.running.load(Ordering::Relaxed) {
                return Ok(());
            }
            s.writer.write_all(data.as_bytes())?;
            s.writer.flush()?;
            Ok(())
        })
    }

    pub fn resize(&self, id: &str, cols: u16, rows: u16) -> AppResult<()> {
        self.with_session(id, |s| {
            s.master
                .resize(PtySize {
                    rows: rows.max(2),
                    cols: cols.max(2),
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| AppError::msg(e.to_string()))
        })
    }

    pub fn backlog(&self, id: &str) -> AppResult<String> {
        self.with_session(id, |s| {
            let bytes = s.backlog.lock().map(|b| b.clone()).unwrap_or_default();
            Ok(String::from_utf8_lossy(&bytes).to_string())
        })
    }

    pub fn list(&self) -> Vec<TerminalSessionDto> {
        let Ok(sessions) = self.sessions.lock() else {
            return Vec::new();
        };
        let mut list: Vec<_> = sessions.values().map(snapshot).collect();
        list.sort_by(|a, b| a.started_at.cmp(&b.started_at));
        list
    }

    /// Stops the process tree: a polite signal first, then a forced kill if it lingers.
    pub fn kill(&self, id: &str) -> AppResult<()> {
        self.with_session(id, |s| {
            if !s.running.load(Ordering::Relaxed) {
                return Ok(());
            }
            terminate(s);
            Ok(())
        })
    }

    /// Kills the session if needed and forgets it.
    pub fn close(&self, id: &str) -> AppResult<()> {
        let removed = self
            .sessions
            .lock()
            .map_err(|_| AppError::msg("Terminal busy"))?
            .remove(id);
        if let Some(mut session) = removed {
            if session.running.load(Ordering::Relaxed) {
                terminate(&mut session);
            }
        }
        Ok(())
    }

    pub fn kill_all(&self) {
        if let Ok(mut sessions) = self.sessions.lock() {
            for (_, mut session) in sessions.drain() {
                if session.running.load(Ordering::Relaxed) {
                    #[cfg(unix)]
                    if let Some(pid) = session.pid {
                        signal_group(pid, libc::SIGKILL);
                    }
                    let _ = session.killer.kill();
                }
            }
        }
    }
}

fn snapshot(s: &Session) -> TerminalSessionDto {
    let running = s.running.load(Ordering::Relaxed);
    let code = s.exit_code.load(Ordering::Relaxed);
    TerminalSessionDto {
        running,
        exit_code: (!running && code != NO_EXIT_CODE).then_some(code),
        ..s.info.clone()
    }
}

fn terminate(s: &mut Session) {
    #[cfg(unix)]
    if let Some(pid) = s.pid {
        signal_group(pid, libc::SIGTERM);
        let running = s.running.clone();
        std::thread::spawn(move || {
            for _ in 0..20 {
                std::thread::sleep(Duration::from_millis(100));
                if !running.load(Ordering::Relaxed) {
                    return;
                }
            }
            signal_group(pid, libc::SIGKILL);
        });
        return;
    }
    let _ = s.killer.kill();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_split_across_reads_is_reassembled() {
        let bytes = "héllo ✓".as_bytes().to_vec();
        let mut pending = bytes[..2].to_vec(); // "h" + first byte of "é"
        let first = drain_utf8(&mut pending);
        assert_eq!(first, "h");
        pending.extend_from_slice(&bytes[2..]);
        assert_eq!(drain_utf8(&mut pending), "éllo ✓");
        assert!(pending.is_empty());
    }

    #[test]
    fn invalid_bytes_do_not_stall_the_stream() {
        let mut pending = vec![b'a', 0xFF, b'b'];
        let out = drain_utf8(&mut pending);
        assert!(out.starts_with('a') && out.ends_with('b'));
        assert!(pending.is_empty());
    }

    #[test]
    fn cwd_cannot_escape_the_project() {
        let root = std::env::temp_dir().join(format!("bp-term-{}", std::process::id()));
        let sub = root.join("packages").join("web");
        std::fs::create_dir_all(&sub).unwrap();
        assert_eq!(resolve_cwd(&root, None), root);
        assert_eq!(
            resolve_cwd(&root, Some("packages/web")),
            sub.canonicalize().unwrap()
        );
        assert_eq!(resolve_cwd(&root, Some("../../etc")), root);
        assert_eq!(resolve_cwd(&root, Some("/")), root);
        assert_eq!(resolve_cwd(&root, Some("missing")), root);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn runs_a_command_and_reports_output_and_exit_code() {
        let events: Arc<Mutex<Vec<(String, serde_json::Value)>>> = Arc::default();
        let captured = events.clone();
        let sink: EventSink = Arc::new(move |name, payload| {
            captured.lock().unwrap().push((name.to_string(), payload));
        });
        let registry = TerminalRegistry::default();
        let session = registry
            .spawn(
                sink,
                SpawnRequest {
                    project_id: None,
                    command_id: None,
                    title: "t".into(),
                    command: Some("echo hello-$BP_TEST && exit 3".into()),
                    cwd: std::env::temp_dir(),
                    env: vec![("BP_TEST".into(), "pty".into())],
                    cols: 80,
                    rows: 24,
                },
            )
            .unwrap();
        for _ in 0..100 {
            if !registry.list().iter().any(|s| s.running) {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let info = registry
            .list()
            .into_iter()
            .find(|s| s.id == session.id)
            .unwrap();
        assert!(!info.running);
        assert_eq!(info.exit_code, Some(3));
        assert!(registry.backlog(&session.id).unwrap().contains("hello-pty"));
        let seen = events.lock().unwrap();
        assert!(seen.iter().any(
            |(n, p)| n == DATA_EVENT && p["data"].as_str().unwrap_or("").contains("hello-pty")
        ));
        assert!(seen.iter().any(|(n, p)| n == EXIT_EVENT && p["code"] == 3));
        drop(seen);
        registry.close(&session.id).unwrap();
        assert!(registry.list().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn kill_stops_a_long_running_process() {
        let sink: EventSink = Arc::new(|_, _| {});
        let registry = TerminalRegistry::default();
        let session = registry
            .spawn(
                sink,
                SpawnRequest {
                    project_id: None,
                    command_id: None,
                    title: "sleep".into(),
                    command: Some("sleep 30".into()),
                    cwd: std::env::temp_dir(),
                    env: vec![],
                    cols: 80,
                    rows: 24,
                },
            )
            .unwrap();
        std::thread::sleep(Duration::from_millis(300));
        assert!(registry.list()[0].running);
        registry.kill(&session.id).unwrap();
        for _ in 0..60 {
            if !registry.list()[0].running {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(!registry.list()[0].running);
    }

    #[test]
    fn command_runs_through_the_login_shell() {
        let (_, args) = shell_invocation(Some("echo hi"));
        assert!(args.iter().any(|a| a == "echo hi"));
        let (_, plain) = shell_invocation(None);
        assert!(!plain.iter().any(|a| a == "-c"));
    }
}
