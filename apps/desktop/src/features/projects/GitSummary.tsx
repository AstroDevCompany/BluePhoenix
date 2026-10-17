import { ArrowDown, ArrowUp, Archive, GitBranch } from "lucide-react";
import type { GitStatus } from "../../lib/types";
import { formatRelative } from "../../lib/format";

export function gitChangeCount(git: GitStatus) {
  return git.staged + git.changed + git.untracked;
}

/** Whether a repo has something the developer probably wants to act on. */
export function needsAttention(git?: GitStatus | null) {
  if (!git || git.pending || !git.isRepo) return false;
  return git.dirty || git.ahead > 0 || git.behind > 0;
}

/** One-line live repository state: branch, sync counts, local changes and the last commit. */
export function GitSummary({
  git,
  showCommit = true,
  showBranch = true,
}: {
  git?: GitStatus | null;
  showCommit?: boolean;
  showBranch?: boolean;
}) {
  if (!git) return null;
  if (git.pending) return <div className="git-summary muted">Checking repository…</div>;
  if (!git.isRepo) {
    return <div className="git-summary muted">{git.error ?? "Not a git repository"}</div>;
  }
  if (git.error) return <div className="git-summary git-error">{git.error}</div>;
  const changes = gitChangeCount(git);
  return (
    <div className="git-summary">
      {showBranch ? (
        <span className="git-chip branch" title={git.upstream ? `Tracking ${git.upstream}` : "No upstream branch"}>
          <GitBranch size={12} aria-hidden />
          {git.detached ? "detached HEAD" : git.branch ?? "unknown"}
        </span>
      ) : null}
      {git.ahead > 0 ? (
        <span className="git-chip ahead" title={`${git.ahead} commit(s) to push`}>
          <ArrowUp size={11} aria-hidden />
          {git.ahead}
        </span>
      ) : null}
      {git.behind > 0 ? (
        <span className="git-chip behind" title={`${git.behind} commit(s) to pull`}>
          <ArrowDown size={11} aria-hidden />
          {git.behind}
        </span>
      ) : null}
      {changes > 0 ? (
        <span
          className="git-chip dirty"
          title={`${git.staged} staged · ${git.changed} modified · ${git.untracked} untracked`}
        >
          {changes} change{changes === 1 ? "" : "s"}
        </span>
      ) : (
        <span className="git-chip clean">clean</span>
      )}
      {git.stash > 0 ? (
        <span className="git-chip" title={`${git.stash} stash entr${git.stash === 1 ? "y" : "ies"}`}>
          <Archive size={11} aria-hidden />
          {git.stash}
        </span>
      ) : null}
      {showCommit && git.lastCommit ? (
        <span className="git-commit-line" title={git.lastCommit}>
          {git.lastCommit}
          {git.lastCommitAt ? <span className="muted"> · {formatRelative(git.lastCommitAt)}</span> : null}
        </span>
      ) : null}
    </div>
  );
}
