import type { Category, ProjectCard } from "../../lib/types";
import { formatDuration } from "../../lib/format";

export function SoftwareDashboard({ projects }: { category: Category; projects: ProjectCard[] }) {
  return (
    <div className="stat-grid">
      <div className="stat">Active projects<b>{projects.filter((p) => p.status === "active").length}</b></div>
      <div className="stat">Tracked time<b>{formatDuration(projects.reduce((a, p) => a + p.totalTrackedSeconds, 0))}</b></div>
      <div className="stat">Open TODOs<b>{projects.reduce((a, p) => a + p.openTodos, 0)}</b></div>
    </div>
  );
}
