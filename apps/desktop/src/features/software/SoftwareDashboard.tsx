import { useState } from "react";
import { CheckCircle2, Loader2, RefreshCw, Square } from "lucide-react";
import type { Category, ProjectCard, Todo } from "../../lib/types";
import { formatDuration } from "../../lib/format";
import { useRunningTimer, useTimeSummary, useTodos, useToggleTodo } from "../../lib/queries";
import { fetchAll } from "../../lib/projectActions";
import { GitButton } from "../projects/ProjectActions";
import { GitSummary, needsAttention } from "../projects/GitSummary";
import { attentionScore } from "../projects/projectView";
import { useTerminals } from "../../stores/terminals";
import { closeSession, killSession } from "../../lib/terminalActions";

const PRIORITY_RANK: Record<string, number> = { high: 0, urgent: 0, medium: 1, low: 2 };

function Panel({ title, action, children }: { title: string; action?: React.ReactNode; children: React.ReactNode }) {
  return (
    <section className="dash-panel glass-panel">
      <header className="dash-panel-head">
        <h3 className="dash-panel-title">{title}</h3>
        {action}
      </header>
      {children}
    </section>
  );
}

function WeekBars({ days }: { days: { date: string; seconds: number }[] }) {
  const max = Math.max(1, ...days.map((d) => d.seconds));
  return (
    <div className="week-bars" role="img" aria-label={`Tracked time over the last 7 days: ${days.map((d) => formatDuration(d.seconds)).join(", ")}`}>
      {days.map((d, i) => {
        const label = new Date(`${d.date}T12:00:00`).toLocaleDateString(undefined, { weekday: "narrow" });
        return (
          <div key={d.date} className={`week-bar ${i === days.length - 1 ? "today" : ""}`} title={`${d.date} · ${formatDuration(d.seconds)}`}>
            <span className="week-bar-fill" style={{ height: `${Math.max(d.seconds > 0 ? 6 : 2, (d.seconds / max) * 100)}%` }} />
            <span className="week-bar-label">{label}</span>
          </div>
        );
      })}
    </div>
  );
}

export function SoftwareDashboard({
  category,
  projects,
  onOpenProject,
}: {
  category: Category;
  projects: ProjectCard[];
  onOpenProject?: (id: string) => void;
}) {
  const summary = useTimeSummary(category.id);
  const { data: timer } = useRunningTimer();
  const todosQuery = useTodos(category.id);
  const toggle = useToggleTodo(category.id);
  const sessions = useTerminals((s) => s.sessions);
  const [fetching, setFetching] = useState(false);

  const live = projects.filter((p) => !p.archived);
  const attention = live.filter((p) => needsAttention(p.git)).sort((a, b) => attentionScore(b) - attentionScore(a));
  const trackedRepos = live.filter((p) => p.git?.isRepo);
  const nextUp: Todo[] = (todosQuery.data ?? [])
    .filter((t) => t.status !== "completed")
    .sort(
      (a, b) =>
        (PRIORITY_RANK[a.priority] ?? 1) - (PRIORITY_RANK[b.priority] ?? 1) ||
        (a.dueDate ?? "9999").localeCompare(b.dueDate ?? "9999"),
    )
    .slice(0, 6);
  const running = timer && timer.isLocalDevice && !timer.entry.endedAt ? timer : null;
  const runningSessions = sessions.filter((s) => s.running);

  const doFetchAll = async () => {
    setFetching(true);
    try {
      await fetchAll(trackedRepos.map((p) => p.id));
    } finally {
      setFetching(false);
    }
  };

  return (
    <div className="dash">
      <div className="stat-grid">
        <div className="stat">
          Today
          <b>{formatDuration(summary.data?.todaySeconds ?? 0)}</b>
          {running ? <small className="stat-note">● {running.projectName} running</small> : null}
        </div>
        <div className="stat">
          This week
          <b>{formatDuration(summary.data?.weekSeconds ?? 0)}</b>
        </div>
        <div className="stat">
          Open TODOs
          <b>{live.reduce((a, p) => a + p.openTodos, 0)}</b>
        </div>
        <div className="stat">
          Needs attention
          <b>{attention.length}</b>
          <small className="stat-note">of {trackedRepos.length} repos</small>
        </div>
      </div>

      <div className="dash-grid">
        <Panel
          title="Needs attention"
          action={
            <button className="btn" type="button" disabled={fetching || trackedRepos.length === 0} onClick={() => void doFetchAll()}>
              {fetching ? <Loader2 size={14} className="spin" /> : <RefreshCw size={14} />} Fetch all
            </button>
          }
        >
          {attention.length === 0 ? (
            <p className="dash-empty muted">
              <CheckCircle2 size={15} aria-hidden /> {trackedRepos.length === 0 ? "No git repositories tracked yet." : "Every repository is clean and in sync."}
            </p>
          ) : (
            attention.map((p) => (
              <div key={p.id} className="dash-row">
                <button type="button" className="dash-row-main" onClick={() => onOpenProject?.(p.id)}>
                  <span className="dash-row-title">{p.name}</span>
                  <GitSummary git={p.git} showCommit={false} />
                </button>
                <GitButton project={p} />
              </div>
            ))
          )}
        </Panel>

        <Panel title="Next up">
          {todosQuery.isPending ? (
            <p className="dash-empty muted">Loading…</p>
          ) : nextUp.length === 0 ? (
            <p className="dash-empty muted">No open TODOs. Nice.</p>
          ) : (
            nextUp.map((t) => (
              <label key={t.id} className="dash-todo">
                <input type="checkbox" checked={false} onChange={() => toggle.mutate(t)} />
                <span className="dash-todo-title">{t.title}</span>
                <button
                  type="button"
                  className="dash-todo-project"
                  onClick={(e) => {
                    e.preventDefault();
                    onOpenProject?.(t.projectId);
                  }}
                >
                  {t.projectName}
                </button>
              </label>
            ))
          )}
        </Panel>

        <Panel title="Last 7 days">
          <WeekBars days={summary.data?.days ?? []} />
        </Panel>

        <Panel title="Running in terminals">
          {runningSessions.length === 0 ? (
            <p className="dash-empty muted">Nothing running. Start a dev server from a project card.</p>
          ) : (
            runningSessions.map((s) => (
              <div key={s.id} className="dash-row">
                <button
                  type="button"
                  className="dash-row-main"
                  onClick={() => {
                    useTerminals.getState().setActive(s.id);
                    useTerminals.getState().setDockOpen(true);
                  }}
                >
                  <span className="dash-row-title">{s.title}</span>
                  <span className="muted dash-row-sub">{s.cwd}</span>
                </button>
                <button type="button" className="btn icon ghost" aria-label={`Stop ${s.title}`} onClick={() => void killSession(s.id)}>
                  <Square size={13} />
                </button>
                <button type="button" className="btn icon ghost" aria-label={`Close ${s.title}`} onClick={() => void closeSession(s.id)}>
                  ×
                </button>
              </div>
            ))
          )}
        </Panel>
      </div>
    </div>
  );
}
