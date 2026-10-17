use crate::error::{AppError, AppResult};
use crate::models::{GitBranchDto, GitChangeDto, GitCommitDto, GitStatusDto};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const STATUS_TIMEOUT: Duration = Duration::from_secs(20);
const NETWORK_TIMEOUT: Duration = Duration::from_secs(120);
const LOCAL_TIMEOUT: Duration = Duration::from_secs(30);

/// Builds a git command that can never block on a prompt and reports messages in English,
/// so error classification stays reliable on localized systems.
fn git_command(git: &str, repo: &Path) -> Command {
    let mut cmd = Command::new(git);
    cmd.current_dir(repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("LC_MESSAGES", "C")
        .stdin(Stdio::null());
    cmd
}

struct Captured {
    ok: bool,
    stdout: String,
    stderr: String,
}

/// Runs a prepared command, killing it if it outlives `limit`.
fn capture(mut cmd: Command, limit: Duration) -> std::io::Result<Captured> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    let mut out = child.stdout.take();
    let mut err = child.stderr.take();
    let out_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(o) = out.as_mut() {
            let _ = o.read_to_end(&mut buf);
        }
        buf
    });
    let err_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(e) = err.as_mut() {
            let _ = e.read_to_end(&mut buf);
        }
        buf
    });
    let deadline = Instant::now() + limit;
    let (ok, timed_out) = loop {
        match child.try_wait()? {
            Some(status) => break (status.success(), false),
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                break (false, true);
            }
            None => std::thread::sleep(Duration::from_millis(25)),
        }
    };
    let stdout = String::from_utf8_lossy(&out_reader.join().unwrap_or_default()).to_string();
    let mut stderr = String::from_utf8_lossy(&err_reader.join().unwrap_or_default()).to_string();
    if timed_out {
        stderr = format!("timeout: git did not finish within {}s", limit.as_secs());
    }
    Ok(Captured { ok, stdout, stderr })
}

fn git_bin(_configured: &str) -> String {
    if let Ok(path) = which::which("git") {
        return path.to_string_lossy().to_string();
    }
    for candidate in git_install_candidates() {
        if Path::new(&candidate).is_file() {
            return candidate;
        }
    }
    "git".into()
}

fn git_install_candidates() -> Vec<String> {
    let mut candidates = Vec::new();
    #[cfg(target_os = "windows")]
    {
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            candidates.push(format!("{program_files}\\Git\\cmd\\git.exe"));
            candidates.push(format!("{program_files}\\Git\\bin\\git.exe"));
        }
        candidates.push("C:\\Program Files\\Git\\cmd\\git.exe".into());
        candidates.push("C:\\Program Files\\Git\\bin\\git.exe".into());
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            candidates.push(format!("{local}\\Programs\\Git\\cmd\\git.exe"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        candidates.push("/opt/homebrew/bin/git".into());
        candidates.push("/usr/local/bin/git".into());
        candidates.push("/usr/bin/git".into());
    }
    candidates
}

pub fn status(configured_git: &str, repo: &Path) -> GitStatusDto {
    let unavailable = |available: bool, is_repo: bool, error: &str| GitStatusDto {
        available,
        is_repo,
        error: Some(error.into()),
        ..GitStatusDto::default()
    };
    if !repo.exists() {
        return unavailable(git_exists(configured_git), false, "Folder does not exist");
    }
    let git = git_bin(configured_git);
    if !repo.join(".git").exists() {
        return unavailable(
            git_exists(configured_git),
            false,
            "Git repository not found",
        );
    }
    let mut cmd = git_command(&git, repo);
    cmd.args(["status", "--porcelain=v2", "--branch", "--show-stash"]);
    let captured = match capture(cmd, STATUS_TIMEOUT) {
        Ok(c) => c,
        Err(_) => return unavailable(false, false, "Git is unavailable"),
    };
    if !captured.ok {
        let msg = captured
            .stderr
            .trim()
            .lines()
            .next()
            .unwrap_or("Git status failed");
        return unavailable(true, true, msg);
    }
    let mut dto = parse_status_v2(&captured.stdout);
    dto.available = true;
    dto.is_repo = true;
    let mut log = git_command(&git, repo);
    log.args(["log", "-1", "--pretty=%s%x1f%cI"]);
    if let Ok(c) = capture(log, STATUS_TIMEOUT) {
        if c.ok {
            let mut parts = c.stdout.trim().splitn(2, '\u{1f}');
            dto.last_commit = parts.next().map(str::to_string).filter(|s| !s.is_empty());
            dto.last_commit_at = parts.next().map(str::to_string).filter(|s| !s.is_empty());
        }
    }
    dto
}

/// Parses `git status --porcelain=v2 --branch --show-stash` output.
pub fn parse_status_v2(raw: &str) -> GitStatusDto {
    let mut dto = GitStatusDto::default();
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            dto.branch = (rest != "(detached)").then(|| rest.to_string());
            if rest == "(detached)" {
                dto.detached = true;
            }
        } else if let Some(rest) = line.strip_prefix("# branch.upstream ") {
            dto.upstream = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            let mut it = rest.split_whitespace();
            dto.ahead = it
                .next()
                .and_then(|v| v.trim_start_matches('+').parse().ok())
                .unwrap_or(0);
            dto.behind = it
                .next()
                .and_then(|v| v.trim_start_matches('-').parse().ok())
                .unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("# stash ") {
            dto.stash = rest.trim().parse().unwrap_or(0);
        } else if line.starts_with("? ") {
            dto.untracked += 1;
        } else if line.starts_with("1 ") || line.starts_with("2 ") || line.starts_with("u ") {
            let xy = line.split_whitespace().nth(1).unwrap_or("..");
            let mut chars = xy.chars();
            let (x, y) = (chars.next().unwrap_or('.'), chars.next().unwrap_or('.'));
            if x != '.' {
                dto.staged += 1;
            }
            if y != '.' || line.starts_with("u ") {
                dto.changed += 1;
            }
        }
    }
    dto.dirty = dto.staged + dto.changed + dto.untracked > 0;
    dto
}

fn git_exists(configured: &str) -> bool {
    Command::new(git_bin(configured))
        .arg("--version")
        .output()
        .is_ok()
}

fn stdout(git: &str, repo: &Path, args: &[&str]) -> Option<String> {
    let mut cmd = git_command(git, repo);
    cmd.args(args);
    let out = capture(cmd, LOCAL_TIMEOUT).ok()?;
    if out.ok {
        Some(out.stdout.trim().to_string()).filter(|s| !s.is_empty())
    } else {
        None
    }
}

pub fn origin_url(configured: &str, repo: &Path) -> Option<String> {
    if !repo.exists() {
        return None;
    }
    let git = git_bin(configured);
    if let Some(url) = stdout(&git, repo, &["config", "--get", "remote.origin.url"]) {
        return Some(url);
    }
    let remotes = stdout(&git, repo, &["remote", "-v"])?;
    remotes.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let name = parts.next()?;
        let url = parts.next()?;
        (name == "origin").then(|| url.to_string())
    })
}

pub fn github_https_url(remote: &str) -> Option<String> {
    let remote = remote.trim();
    if remote.is_empty() {
        return None;
    }
    let trimmed = remote.trim_end_matches('/').trim_end_matches(".git");
    let path = if let Some(rest) = trimmed.strip_prefix("git@github.com:") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("ssh://git@github.com/") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("https://github.com/") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("http://github.com/") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("https://www.github.com/") {
        rest
    } else {
        return None;
    };
    let path = path.trim_start_matches('/');
    if path.is_empty() || !path.contains('/') {
        return None;
    }
    Some(format!("https://github.com/{path}"))
}

pub fn run_git(configured: &str, repo: &Path, args: &[&str]) -> AppResult<String> {
    run_git_with(configured, repo, args, NETWORK_TIMEOUT)
}

pub fn run_git_with(
    configured: &str,
    repo: &Path,
    args: &[&str],
    limit: Duration,
) -> AppResult<String> {
    if !repo.exists() {
        return Err(AppError::FolderMissing);
    }
    let git = git_bin(configured);
    let mut cmd = git_command(&git, repo);
    cmd.args(args);
    let out = capture(cmd, limit).map_err(|_| AppError::GitUnavailable)?;
    if out.ok {
        return Ok(if out.stdout.trim().is_empty() {
            out.stderr
        } else {
            out.stdout
        });
    }
    Err(classify_failure(&out.stdout, &out.stderr))
}

/// Maps git failure output to a typed error. The prefix (`conflict:`, `auth:`,
/// `network:`, `timeout:`, `dirty:`) lets the UI render a proper failure state.
fn classify_failure(stdout: &str, stderr: &str) -> AppError {
    let combined = format!("{stdout}{stderr}").to_lowercase();
    if combined.starts_with("timeout:") {
        return AppError::msg("timeout: Git took too long and was stopped");
    }
    if combined.contains("conflict") {
        return AppError::msg("conflict: Merge conflicts — resolve them before continuing");
    }
    if combined.contains("authentication")
        || combined.contains("permission denied")
        || combined.contains("could not read username")
    {
        return AppError::msg("auth: Git authentication failed");
    }
    if combined.contains("could not resolve host")
        || combined.contains("unable to access")
        || combined.contains("could not read from remote")
    {
        return AppError::msg("network: Can't reach the remote");
    }
    if combined.contains("would be overwritten") || combined.contains("local changes") {
        return AppError::msg(
            "dirty: Local changes would be overwritten — commit or stash them first",
        );
    }
    if combined.contains("not a git repository") {
        return AppError::GitRepoNotFound;
    }
    AppError::msg(
        stderr
            .trim()
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("Git command failed"),
    )
}

pub fn has_remote(configured: &str, repo: &Path, name: &str) -> bool {
    stdout(&git_bin(configured), repo, &["remote"])
        .map(|r| r.lines().any(|l| l.trim() == name))
        .unwrap_or(false)
}

pub fn branches(configured: &str, repo: &Path) -> AppResult<Vec<GitBranchDto>> {
    let raw = run_git_with(
        configured,
        repo,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            "--format=%(HEAD)%1f%(refname:short)%1f%(upstream:short)%1f%(committerdate:iso-strict)%1f%(subject)",
            "refs/heads",
        ],
        LOCAL_TIMEOUT,
    )?;
    Ok(parse_branches(&raw))
}

pub fn parse_branches(raw: &str) -> Vec<GitBranchDto> {
    raw.lines()
        .filter_map(|line| {
            let mut p = line.split('\u{1f}');
            let head = p.next()?;
            let name = p.next()?.to_string();
            if name.is_empty() {
                return None;
            }
            let upstream = p.next().map(str::to_string).filter(|s| !s.is_empty());
            let last_commit_at = p.next().map(str::to_string).filter(|s| !s.is_empty());
            let subject = p.next().map(str::to_string).filter(|s| !s.is_empty());
            Some(GitBranchDto {
                name,
                current: head.trim() == "*",
                upstream,
                last_commit_at,
                subject,
            })
        })
        .collect()
}

pub fn changes(configured: &str, repo: &Path) -> AppResult<Vec<GitChangeDto>> {
    let raw = run_git_with(
        configured,
        repo,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        STATUS_TIMEOUT,
    )?;
    Ok(parse_changes(&raw))
}

/// Parses NUL-separated porcelain v1 output. Renames carry a second path entry.
pub fn parse_changes(raw: &str) -> Vec<GitChangeDto> {
    let mut out = Vec::new();
    let mut entries = raw.split('\0').filter(|e| !e.is_empty());
    while let Some(entry) = entries.next() {
        if entry.len() < 4 {
            continue;
        }
        let mut chars = entry.chars();
        let index = chars.next().unwrap_or(' ');
        let worktree = chars.next().unwrap_or(' ');
        let path = entry[3..].to_string();
        let mut original = None;
        if index == 'R' || index == 'C' {
            original = entries.next().map(str::to_string);
        }
        out.push(GitChangeDto {
            path,
            original_path: original,
            index: index.to_string(),
            worktree: worktree.to_string(),
            staged: index != ' ' && index != '?' && index != '!',
            untracked: index == '?',
            conflicted: index == 'U'
                || worktree == 'U'
                || (index == 'A' && worktree == 'A')
                || (index == 'D' && worktree == 'D'),
        });
    }
    out
}

pub fn log(configured: &str, repo: &Path, limit: usize) -> AppResult<Vec<GitCommitDto>> {
    let n = format!("-{}", limit.clamp(1, 200));
    let raw = run_git_with(
        configured,
        repo,
        &["log", &n, "--pretty=%H%x1f%h%x1f%an%x1f%cI%x1f%s%x1e"],
        LOCAL_TIMEOUT,
    )
    .unwrap_or_default();
    Ok(parse_log(&raw))
}

pub fn parse_log(raw: &str) -> Vec<GitCommitDto> {
    raw.split('\u{1e}')
        .filter_map(|rec| {
            let rec = rec.trim_matches(|c| c == '\n' || c == '\r');
            if rec.is_empty() {
                return None;
            }
            let mut p = rec.split('\u{1f}');
            Some(GitCommitDto {
                hash: p.next()?.to_string(),
                short_hash: p.next()?.to_string(),
                author: p.next()?.to_string(),
                date: p.next()?.to_string(),
                subject: p.next().unwrap_or("").to_string(),
            })
        })
        .collect()
}

pub fn commit_count(configured: &str, repo: &Path) -> i64 {
    stdout(&git_bin(configured), repo, &["rev-list", "--count", "HEAD"])
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

pub const DIFF_CHAR_LIMIT: usize = 24_000;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitDiffContext {
    pub status: String,
    pub diff: String,
    pub truncated: bool,
    pub has_history: bool,
    pub staged: bool,
    pub dirty: bool,
    pub subjects: Vec<String>,
}

pub fn truncate_text(value: &str, max: usize) -> (String, bool) {
    if value.len() <= max {
        (value.to_string(), false)
    } else {
        (format!("{}\n…[truncated]", &value[..max]), true)
    }
}

pub fn classify_porcelain(status: &str) -> (bool, bool) {
    let dirty = !status.trim().is_empty();
    let staged = status.lines().any(|line| {
        let bytes = line.as_bytes();
        !bytes.is_empty() && bytes[0] != b' ' && bytes[0] != b'?'
    });
    (dirty, staged)
}

pub fn recent_subjects(configured: &str, repo: &Path, n: usize) -> Vec<String> {
    let git = git_bin(configured);
    let pretty = format!("-{n}");
    stdout(&git, repo, &["log", &pretty, "--pretty=%s"])
        .map(|s| {
            s.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

pub fn log_since(configured: &str, repo: &Path, last_version: Option<&str>) -> String {
    let git = git_bin(configured);
    if let Some(v) = last_version {
        let range = format!("{v}..HEAD");
        if let Some(s) = stdout(&git, repo, &["log", &range, "--pretty=%s"]) {
            if !s.trim().is_empty() {
                return s;
            }
        }
    }
    if let Some(tag) = stdout(&git, repo, &["describe", "--tags", "--abbrev=0"]) {
        let range = format!("{tag}..HEAD");
        if let Some(s) = stdout(&git, repo, &["log", &range, "--pretty=%s"]) {
            if !s.trim().is_empty() {
                return s;
            }
        }
    }
    stdout(&git, repo, &["log", "-n", "40", "--pretty=%s"]).unwrap_or_default()
}

pub fn diff_context(configured: &str, repo: &Path) -> GitDiffContext {
    let git = git_bin(configured);
    let status = stdout(&git, repo, &["status", "--porcelain"]).unwrap_or_default();
    let (dirty, staged) = classify_porcelain(&status);
    let unstaged = stdout(&git, repo, &["diff", "HEAD"]).unwrap_or_default();
    let staged_diff = stdout(&git, repo, &["diff", "--cached"]).unwrap_or_default();
    let combined = if staged_diff.trim().is_empty() {
        unstaged
    } else {
        format!("# staged\n{staged_diff}\n# unstaged/HEAD\n{unstaged}")
    };
    let (diff, truncated) = truncate_text(&combined, DIFF_CHAR_LIMIT);
    let subjects = recent_subjects(configured, repo, 20);
    GitDiffContext {
        status,
        diff,
        truncated,
        has_history: !subjects.is_empty(),
        staged,
        dirty,
        subjects,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status_v2_branch_and_counts() {
        let raw = "# branch.oid abc\n# branch.head main\n# branch.upstream origin/main\n# branch.ab +2 -1\n# stash 3\n1 M. N... 100644 100644 100644 a b src/a.rs\n1 .M N... 100644 100644 100644 a b src/b.rs\n1 MM N... 100644 100644 100644 a b src/c.rs\n? new.txt\n? other.txt\n";
        let s = parse_status_v2(raw);
        assert_eq!(s.branch.as_deref(), Some("main"));
        assert_eq!(s.upstream.as_deref(), Some("origin/main"));
        assert_eq!((s.ahead, s.behind), (2, 1));
        assert_eq!(s.stash, 3);
        assert_eq!(s.staged, 2);
        assert_eq!(s.changed, 2);
        assert_eq!(s.untracked, 2);
        assert!(s.dirty);
    }

    #[test]
    fn parses_clean_and_detached_status() {
        let s = parse_status_v2("# branch.oid abc\n# branch.head (detached)\n");
        assert!(s.detached && s.branch.is_none() && !s.dirty);
        let s = parse_status_v2("# branch.head main\n");
        assert_eq!((s.ahead, s.behind, s.upstream.clone()), (0, 0, None));
    }

    #[test]
    fn parses_porcelain_changes_with_rename() {
        let raw =
            "M  staged.rs\0 M dirty.rs\0?? new.txt\0R  new_name.rs\0old_name.rs\0UU conflict.rs\0";
        let c = parse_changes(raw);
        assert_eq!(c.len(), 5);
        assert!(c[0].staged && !c[0].untracked);
        assert!(!c[1].staged);
        assert!(c[2].untracked);
        assert_eq!(c[3].original_path.as_deref(), Some("old_name.rs"));
        assert!(c[4].conflicted);
    }

    #[test]
    fn parses_branches_and_log() {
        let b = parse_branches("*\u{1f}main\u{1f}origin/main\u{1f}2026-01-01T00:00:00+00:00\u{1f}fix\n \u{1f}dev\u{1f}\u{1f}\u{1f}wip\n");
        assert_eq!(b.len(), 2);
        assert!(b[0].current && b[0].upstream.is_some());
        assert!(!b[1].current && b[1].upstream.is_none());
        let l = parse_log("abc123\u{1f}abc\u{1f}Ann\u{1f}2026-01-01T00:00:00Z\u{1f}feat: x\u{1e}\ndef456\u{1f}def\u{1f}Bob\u{1f}2026-01-02T00:00:00Z\u{1f}fix: y\u{1e}\n");
        assert_eq!(l.len(), 2);
        assert_eq!(l[1].author, "Bob");
        assert_eq!(l[0].short_hash, "abc");
    }

    #[test]
    fn classifies_git_failures() {
        let m = |o: &str, e: &str| classify_failure(o, e).to_string();
        assert!(m("", "CONFLICT (content): Merge conflict in a").starts_with("conflict:"));
        assert!(m("", "fatal: Authentication failed").starts_with("auth:"));
        assert!(m("", "fatal: Could not resolve host: github.com").starts_with("network:"));
        assert!(m(
            "",
            "error: Your local changes would be overwritten by merge"
        )
        .starts_with("dirty:"));
        assert!(m("", "timeout: git did not finish within 5s").starts_with("timeout:"));
    }

    #[test]
    fn status_branches_and_log_against_a_real_repo() {
        let dir = std::env::temp_dir().join(format!("bp-git-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let git = git_bin("");
        let run = |args: &[&str]| {
            let out = Command::new(&git)
                .args(["-c", "user.name=T", "-c", "user.email=t@example.com"])
                .args(args)
                .current_dir(&dir)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "git {:?}: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        };
        run(&["init", "-q", "-b", "main"]);
        std::fs::write(dir.join("a.txt"), "one").unwrap();
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "feat: first"]);
        std::fs::write(dir.join("a.txt"), "two").unwrap();
        std::fs::write(dir.join("b.txt"), "new").unwrap();

        let s = status("", &dir);
        assert!(s.is_repo && s.available && s.dirty);
        assert_eq!(s.branch.as_deref(), Some("main"));
        assert_eq!((s.changed, s.untracked, s.staged), (1, 1, 0));
        assert_eq!(s.last_commit.as_deref(), Some("feat: first"));

        let c = changes("", &dir).unwrap();
        assert_eq!(c.len(), 2);
        let b = branches("", &dir).unwrap();
        assert_eq!(b.len(), 1);
        assert!(b[0].current);
        let l = log("", &dir, 10).unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].subject, "feat: first");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn capture_kills_slow_commands() {
        let mut cmd = Command::new("sleep");
        cmd.arg("5");
        let started = Instant::now();
        let out = capture(cmd, Duration::from_millis(200)).unwrap();
        assert!(!out.ok);
        assert!(out.stderr.starts_with("timeout:"));
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn git_bin_ignores_configured_override() {
        let resolved = git_bin(r"C:\definitely-not-git.exe");
        assert_ne!(resolved, r"C:\definitely-not-git.exe");
    }

    #[test]
    fn porcelain_clean_dirty_staged() {
        assert_eq!(classify_porcelain(""), (false, false));
        assert_eq!(classify_porcelain("?? new.txt\n"), (true, false));
        assert_eq!(classify_porcelain("M  staged.rs\n"), (true, true));
        assert_eq!(classify_porcelain(" M unstaged.rs\n"), (true, false));
    }

    #[test]
    fn truncates_huge_diff() {
        let big = "a".repeat(DIFF_CHAR_LIMIT + 50);
        let (out, truncated) = truncate_text(&big, DIFF_CHAR_LIMIT);
        assert!(truncated);
        assert!(out.contains("truncated"));
        assert!(out.len() < big.len());
    }

    #[test]
    fn style_on_uses_history_off_ignores_it() {
        let dirty = GitDiffContext {
            status: "M  a.rs".into(),
            diff: "diff --git a/a.rs".into(),
            truncated: false,
            has_history: true,
            staged: true,
            dirty: true,
            subjects: vec!["feat: add rail".into()],
        };
        assert!(dirty.staged && dirty.dirty && dirty.has_history);
        let clean = GitDiffContext {
            status: String::new(),
            diff: String::new(),
            truncated: false,
            has_history: false,
            staged: false,
            dirty: false,
            subjects: vec![],
        };
        assert!(!clean.dirty && !clean.has_history);
    }

    #[test]
    fn no_history_context() {
        let ctx = GitDiffContext {
            status: String::new(),
            diff: String::new(),
            truncated: false,
            has_history: false,
            staged: false,
            dirty: false,
            subjects: vec![],
        };
        assert!(!ctx.has_history);
        assert!(!ctx.dirty);
    }

    #[test]
    fn github_https_from_ssh_and_https() {
        assert_eq!(
            github_https_url("git@github.com:org/repo.git"),
            Some("https://github.com/org/repo".into())
        );
        assert_eq!(
            github_https_url("https://github.com/org/repo.git"),
            Some("https://github.com/org/repo".into())
        );
        assert_eq!(github_https_url("https://gitlab.com/org/repo.git"), None);
    }
}
