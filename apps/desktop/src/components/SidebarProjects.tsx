import { ChevronRight } from "lucide-react";
import type { Category, ProjectCard } from "../lib/types";
import { formatCombo } from "../lib/shortcuts";

export const SIDEBAR_LIMIT = 8;

/** Favorites first, then most recently opened, capped for the sidebar. */
export function pickSidebarProjects(projects: ProjectCard[], recentIds: string[]): ProjectCard[] {
  const live = projects.filter((p) => !p.archived);
  const favorites = live.filter((p) => p.favorite);
  const recents = recentIds
    .map((id) => live.find((p) => p.id === id))
    .filter((p): p is ProjectCard => Boolean(p) && !p?.favorite);
  const rest = live.filter((p) => !p.favorite && !recentIds.includes(p.id));
  return [...favorites, ...recents, ...rest].slice(0, SIDEBAR_LIMIT);
}

export type ProjectState = "running" | "dirty" | "behind" | "ahead" | "clean" | "unknown";

export function projectState(p: ProjectCard, runningProjectId?: string | null): ProjectState {
  if (runningProjectId && p.id === runningProjectId) return "running";
  const g = p.git;
  if (!g || g.pending || !g.isRepo) return "unknown";
  if (g.error) return "unknown";
  if (g.dirty) return "dirty";
  if (g.behind > 0) return "behind";
  if (g.ahead > 0) return "ahead";
  return "clean";
}

const STATE_LABEL: Record<ProjectState, string> = {
  running: "Timer running",
  dirty: "Uncommitted changes",
  behind: "Behind remote",
  ahead: "Unpushed commits",
  clean: "Up to date",
  unknown: "",
};

export function StatusDot({ state }: { state: ProjectState }) {
  return <span className="status-dot" data-state={state} title={STATE_LABEL[state]} aria-label={STATE_LABEL[state] || undefined} />;
}

export function SidebarProjects({
  category,
  projects,
  activeId,
  runningProjectId,
  onOpen,
  onAll,
}: {
  category: Category;
  projects: ProjectCard[];
  activeId?: string;
  runningProjectId?: string | null;
  onOpen: (id: string) => void;
  onAll: () => void;
}) {
  if (projects.length === 0) return null;
  return (
    <div className="sidebar-projects">
      <div className="nav-section">{category.terminology.itemPlural.toUpperCase()}</div>
      {projects.map((p, i) => {
        const g = p.git && !p.git.pending && p.git.isRepo ? p.git : null;
        return (
          <button
            key={p.id}
            type="button"
            className={`side-project ${p.id === activeId ? "active" : ""}`}
            onClick={() => onOpen(p.id)}
            title={i < 9 ? `${p.name} · ${formatCombo(`alt+${i + 1}`)}` : p.name}
          >
            <StatusDot state={projectState(p, runningProjectId)} />
            <span className="side-project-name">{p.name}</span>
            {g && (g.ahead > 0 || g.behind > 0) ? (
              <span className="side-project-meta">
                {g.ahead > 0 ? `↑${g.ahead}` : ""}
                {g.behind > 0 ? `↓${g.behind}` : ""}
              </span>
            ) : null}
          </button>
        );
      })}
      <button type="button" className="side-project all" onClick={onAll}>
        <span className="side-project-name">All {category.terminology.itemPlural.toLowerCase()}</span>
        <ChevronRight size={13} aria-hidden />
      </button>
    </div>
  );
}
