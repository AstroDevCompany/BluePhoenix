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
import { StatusDot, projectState } from "../../components/SidebarProjects";

const PRIORITY_RANK: Record<string, number> = { high: 0, urgent: 0, medium: 1, low: 2 };

function Section({ title, action, children }: { title: string; action?: React.ReactNode; children: React.ReactNode }) {
  return (
    <section className="dash-section">
      <header className="dash-section-head">
        <h2 className="dash-section-title">{title}</h2>
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
        const label = new Date(`${d.date}T12:00:00`).toLocaleDateString(undefined, { weekday: "short" });
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
  const openTodos = live.reduce((a, p) => a + p.openTodos, 0);

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
      <div className="dash-metrics">
        <div className="metric-item">
          <span className="metric-label">Today</span>
          <b>{formatDuration(summary.data?.todaySeconds ?? 0)}</b>
          {running ? <span className="metric-note">{running.projectName} running</span> : null}
        </div>
        <div className="metric-item">
          <span className="metric-label">This week</span>
          <b>{formatDuration(summary.data?.weekSeconds ?? 0)}</b>
        </div>
        <div className="metric-item">
          <span className="metric-label">Open TODOs</span>
          <b>{openTodos}</b>
        </div>
        <div className="metric-item week">
          <WeekBars days={summary.data?.days ?? []} />
        </div>
      </div>

      <div className="dash-cols">
        <Section
          title={`Needs attention${attention.length ? ` · ${attention.length}` : ""}`}
          action={
            <button className="btn sm quiet" type="button" disabled={fetching || trackedRepos.length === 0} onClick={() => void doFetchAll()}>
              {fetching ? <Loader2 size={13} className="spin" /> : <RefreshCw size={13} />} Fetch all
            </button>
          }
        >
          {attention.length === 0 ? (
            <p className="dash-empty">
              <CheckCircle2 size={15} aria-hidden /> {trackedRepos.length === 0 ? "No git repositories tracked yet." : "Every repository is clean and in sync."}
            </p>
          ) : (
            attention.map((p) => (
              <div key={p.id} className="dash-row">
                <StatusDot state={projectState(p)} />
                <button type="button" className="dash-row-main" onClick={() => onOpenProject?.(p.id)}>
                  <span className="dash-row-title">{p.name}</span>
                  <GitSummary git={p.git} />
                </button>
                <GitButton project={p} small />
              </div>
            ))
          )}
        </Section>

        <Section title="Next up">
          {todosQuery.isPending ? (
            <p className="dash-empty">Loading…</p>
          ) : nextUp.length === 0 ? (
            <p className="dash-empty">No open TODOs.</p>
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
        </Section>
      </div>

      {runningSessions.length > 0 ? (
        <Section title="Running in terminals">
          {runningSessions.map((s) => (
            <div key={s.id} className="dash-row">
              <span className="term-status" data-state="running" aria-hidden />
              <button
                type="button"
                className="dash-row-main"
                onClick={() => {
                  useTerminals.getState().setActive(s.id);
                  useTerminals.getState().setDockOpen(true);
                }}
              >
                <span className="dash-row-title">{s.title}</span>
                <span className="faint dash-row-sub">{s.cwd}</span>
              </button>
              <button type="button" className="btn sm quiet" onClick={() => void killSession(s.id)}>
                <Square size={12} /> Stop
              </button>
              <button type="button" className="btn icon sm quiet" aria-label={`Close ${s.title}`} onClick={() => void closeSession(s.id)}>
                ×
              </button>
            </div>
          ))}
        </Section>
      ) : null}
    </div>
  );
}
