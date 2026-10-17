import { GitBranch } from "lucide-react";
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

/** One quiet line of live repository state: branch, sync counts, local changes, optional last commit. */
export function GitSummary({
  git,
  showCommit = false,
  showBranch = true,
}: {
  git?: GitStatus | null;
  showCommit?: boolean;
  showBranch?: boolean;
}) {
  if (!git) return null;
  if (git.pending) return <div className="git-line faint">Checking repository…</div>;
  if (!git.isRepo) return <div className="git-line faint">{git.error ?? "Not a git repository"}</div>;
  if (git.error) return <div className="git-line git-error">{git.error}</div>;
  const changes = gitChangeCount(git);
  const synced = git.ahead === 0 && git.behind === 0 && changes === 0;
  return (
    <div className="git-line">
      {showBranch ? (
        <span className="git-branch" title={git.upstream ? `Tracking ${git.upstream}` : "No upstream branch"}>
          <GitBranch size={12} aria-hidden />
          {git.detached ? "detached HEAD" : git.branch ?? "unknown"}
        </span>
      ) : null}
      {git.ahead > 0 ? <span className="git-up" title={`${git.ahead} commit(s) to push`}>↑{git.ahead}</span> : null}
      {git.behind > 0 ? <span className="git-down" title={`${git.behind} commit(s) to pull`}>↓{git.behind}</span> : null}
      {changes > 0 ? (
        <span className="git-dirty" title={`${git.staged} staged · ${git.changed} modified · ${git.untracked} untracked`}>
          {changes} change{changes === 1 ? "" : "s"}
        </span>
      ) : null}
      {synced ? <span className="faint">up to date</span> : null}
      {git.stash > 0 ? <span className="faint">{git.stash} stashed</span> : null}
      {showCommit && git.lastCommit ? (
        <span className="git-commit-line" title={git.lastCommit}>
          {git.lastCommit}
          {git.lastCommitAt ? <span className="faint"> · {formatRelative(git.lastCommitAt)}</span> : null}
        </span>
      ) : null}
    </div>
  );
}
