use crate::error::{AppError, AppResult};
use crate::models::GitStatusDto;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

fn git_bin(configured: &str) -> String {
    if !configured.trim().is_empty() {
        return configured.trim().to_string();
    }
    which::which("git")
        .ok()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "git".into())
}

pub fn status(configured_git: &str, repo: &Path) -> GitStatusDto {
    if !repo.exists() {
        return GitStatusDto {
            available: git_exists(configured_git),
            is_repo: false,
            branch: None,
            ahead: 0,
            behind: 0,
            dirty: false,
            last_commit: None,
            last_commit_at: None,
            error: Some("Folder does not exist".into()),
        };
    }
    let git = git_bin(configured_git);
    if Command::new(&git).arg("--version").output().is_err() {
        return GitStatusDto {
            available: false,
            is_repo: false,
            branch: None,
            ahead: 0,
            behind: 0,
            dirty: false,
            last_commit: None,
            last_commit_at: None,
            error: Some("Git is unavailable".into()),
        };
    }
    let git_dir = repo.join(".git");
    if !git_dir.exists() {
        return GitStatusDto {
            available: true,
            is_repo: false,
            branch: None,
            ahead: 0,
            behind: 0,
            dirty: false,
            last_commit: None,
            last_commit_at: None,
            error: Some("Git repository not found".into()),
        };
    }

    let branch = stdout(&git, repo, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let dirty = stdout(&git, repo, &["status", "--porcelain"])
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    let last = stdout(&git, repo, &["log", "-1", "--pretty=%s"]);
    let last_at = stdout(&git, repo, &["log", "-1", "--pretty=%cI"]);
    let (ahead, behind) = ahead_behind(&git, repo);

    GitStatusDto {
        available: true,
        is_repo: true,
        branch,
        ahead,
        behind,
        dirty,
        last_commit: last,
        last_commit_at: last_at,
        error: None,
    }
}

fn git_exists(configured: &str) -> bool {
    Command::new(git_bin(configured))
        .arg("--version")
        .output()
        .is_ok()
}

fn stdout(git: &str, repo: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new(git)
        .args(args)
        .current_dir(repo)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string()).filter(|s| !s.is_empty())
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

fn ahead_behind(git: &str, repo: &Path) -> (i64, i64) {
    let Some(raw) = stdout(
        git,
        repo,
        &["rev-list", "--left-right", "--count", "@{u}...HEAD"],
    ) else {
        return (0, 0);
    };
    let parts: Vec<_> = raw.split_whitespace().collect();
    if parts.len() == 2 {
        let behind = parts[0].parse().unwrap_or(0);
        let ahead = parts[1].parse().unwrap_or(0);
        (ahead, behind)
    } else {
        (0, 0)
    }
}

pub fn run_git(configured: &str, repo: &Path, args: &[&str]) -> AppResult<String> {
    if !repo.exists() {
        return Err(AppError::FolderMissing);
    }
    let git = git_bin(configured);
    let mut cmd = Command::new(&git);
    cmd.args(args).current_dir(repo);
    let output = cmd.output().map_err(|_| AppError::GitUnavailable)?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if output.status.success() {
        Ok(if stdout.trim().is_empty() {
            stderr
        } else {
            stdout
        })
    } else {
        let combined = format!("{}{}", stdout, stderr).to_lowercase();
        if combined.contains("conflict") {
            return Err(AppError::msg(
                "Merge conflicts — resolve them before continuing",
            ));
        }
        if combined.contains("authentication") || combined.contains("permission denied") {
            return Err(AppError::msg("Git authentication failed"));
        }
        if combined.contains("could not resolve host") || combined.contains("unable to access") {
            return Err(AppError::Network);
        }
        if combined.contains("not a git repository") {
            return Err(AppError::GitRepoNotFound);
        }
        Err(AppError::msg(
            stderr.trim().lines().next().unwrap_or("Git command failed"),
        ))
    }
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

#[allow(dead_code)]
pub fn timeout() -> Duration {
    Duration::from_secs(30)
}

#[cfg(test)]
mod tests {
    use super::*;

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
