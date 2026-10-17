import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Archive, Check, ChevronDown, Download, GitBranch, Loader2, RefreshCw, Upload } from "lucide-react";
import type { GitAction, ProjectCard } from "../../../lib/types";
import { api } from "../../../lib/ipc";
import { formatRelative } from "../../../lib/format";
import { runGit } from "../../../lib/projectActions";
import { Menu, MenuContent, MenuItem, MenuTrigger } from "../../../components/ui/Menu";
import { GitSummary } from "../GitSummary";
import { EmptyState } from "../../../components/ui/EmptyState";

const gitKeys = {
  branches: (id: string) => ["git", "branches", id] as const,
  changes: (id: string) => ["git", "changes", id] as const,
  log: (id: string) => ["git", "log", id] as const,
};

const STATUS_LABEL: Record<string, string> = { M: "Modified", A: "Added", D: "Deleted", R: "Renamed", C: "Copied", "?": "Untracked", U: "Conflict" };

function changeLetter(index: string, worktree: string, untracked: boolean, conflicted: boolean) {
  if (conflicted) return "U";
  if (untracked) return "?";
  return (index.trim() || worktree.trim() || "M").charAt(0);
}

export function GitTab({ project }: { project: ProjectCard }) {
  const client = useQueryClient();
  const git = project.git;
  const [busy, setBusy] = useState<GitAction | null>(null);
  const ready = Boolean(git && git.isRepo && !git.pending);

  const branches = useQuery({ queryKey: gitKeys.branches(project.id), queryFn: () => api.gitBranches(project.id), enabled: ready });
  const changes = useQuery({ queryKey: gitKeys.changes(project.id), queryFn: () => api.gitChanges(project.id), enabled: ready });
  const log = useQuery({ queryKey: gitKeys.log(project.id), queryFn: () => api.gitLog(project.id, 30), enabled: ready });

  // Whenever the backend reports a fresh status, the lists below are probably stale too.
  useEffect(() => {
    void client.invalidateQueries({ queryKey: ["git"] });
  }, [git?.checkedAt, client]);

  const run = async (action: GitAction, arg?: string) => {
    if (busy) return;
    setBusy(action);
    try {
      await runGit(project, action, arg);
    } finally {
      setBusy(null);
    }
  };

  if (!git || git.pending) return <p className="muted">Checking repository…</p>;
  if (!git.isRepo) {
    return (
      <EmptyState
        compact
        title="Not a git repository"
        body={git.error ?? "Link a project folder that contains a .git directory to see branches and history."}
      />
    );
  }

  const spin = (a: GitAction, icon: React.ReactNode) => (busy === a ? <Loader2 size={14} className="spin" /> : icon);

  return (
    <div className="git-tab">
      <section className="detail-section">
        <div className="git-tab-head">
          <div className="git-tab-status">
            <Menu>
              <MenuTrigger asChild>
                <button type="button" className="btn branch-switch" disabled={busy !== null}>
                  <GitBranch size={14} /> {git.detached ? "detached HEAD" : git.branch ?? "unknown"} <ChevronDown size={13} />
                </button>
              </MenuTrigger>
              <MenuContent>
                {(branches.data ?? []).map((b) => (
                  <MenuItem
                    key={b.name}
                    icon={b.current ? <Check size={14} /> : undefined}
                    hint={b.lastCommitAt ? formatRelative(b.lastCommitAt) : undefined}
                    onSelect={() => !b.current && void run("checkout", b.name)}
                  >
                    {b.name}
                  </MenuItem>
                ))}
                {branches.isPending ? <MenuItem disabled>Loading…</MenuItem> : null}
              </MenuContent>
            </Menu>
            <GitSummary git={git} showCommit={false} showBranch={false} />
          </div>
          <div className="row">
            <button className="btn" type="button" disabled={busy !== null} onClick={() => void run("fetch")}>{spin("fetch", <RefreshCw size={14} />)} Fetch</button>
            <button className="btn" type="button" disabled={busy !== null} onClick={() => void run("pull")}>{spin("pull", <Download size={14} />)} Pull</button>
            <button className="btn" type="button" disabled={busy !== null} onClick={() => void run("push")}>{spin("push", <Upload size={14} />)} Push</button>
            <button className="btn ghost" type="button" disabled={busy !== null || !git.dirty} onClick={() => void run("stash")}>{spin("stash", <Archive size={14} />)} Stash</button>
            <button className="btn ghost" type="button" disabled={busy !== null || git.stash === 0} onClick={() => void run("stash_pop")}>Pop stash{git.stash ? ` (${git.stash})` : ""}</button>
          </div>
        </div>
        {git.upstream ? <p className="muted git-upstream">Tracking {git.upstream}{git.checkedAt ? ` · checked ${formatRelative(git.checkedAt)}` : ""}</p> : <p className="muted git-upstream">No upstream branch. The first push will set origin/{git.branch} when an origin remote exists.</p>}
      </section>

      <section className="detail-section">
        <h3 className="detail-section-title">Changes {changes.data ? `(${changes.data.length})` : ""}</h3>
        {changes.data && changes.data.length === 0 ? <p className="muted">Working tree clean.</p> : null}
        <div className="git-files">
          {(changes.data ?? []).slice(0, 200).map((c) => {
            const letter = changeLetter(c.index, c.worktree, c.untracked, c.conflicted);
            return (
              <div key={c.path} className="git-file" title={STATUS_LABEL[letter] ?? ""}>
                <span className={`git-file-status s-${letter === "?" ? "u" : letter.toLowerCase()}`}>{letter}</span>
                <span className="git-file-path">{c.originalPath ? `${c.originalPath} → ${c.path}` : c.path}</span>
                {c.staged ? <span className="badge">staged</span> : null}
              </div>
            );
          })}
        </div>
      </section>

      <section className="detail-section">
        <h3 className="detail-section-title">Recent commits</h3>
        {(log.data ?? []).length === 0 && !log.isPending ? <p className="muted">No commits yet.</p> : null}
        {(log.data ?? []).map((c) => (
          <div key={c.hash} className="git-commit-row">
            <code className="git-hash">{c.shortHash}</code>
            <span className="git-commit-subject" title={c.subject}>{c.subject}</span>
            <span className="muted git-commit-meta">{c.author} · {formatRelative(c.date)}</span>
          </div>
        ))}
      </section>
    </div>
  );
}
