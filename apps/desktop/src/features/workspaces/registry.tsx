import type { Category, GpaDashboard, ProjectCard } from "../../lib/types";
import { formatDuration } from "../../lib/format";
import { SoftwareDashboard } from "../software/SoftwareDashboard";
import { UniversityDashboard } from "../university/UniversityDashboard";

export function KindDashboard({
  category,
  projects,
  gpa,
}: {
  category: Category;
  projects: ProjectCard[];
  gpa: GpaDashboard | null;
}) {
  if (category.kind === "university") {
    return gpa ? <UniversityDashboard dash={gpa} /> : null;
  }
  if (category.kind === "software") {
    return <SoftwareDashboard category={category} projects={projects} />;
  }
  return (
    <div className="stat-grid">
      <div className="stat">Projects<b>{projects.length}</b></div>
      <div className="stat">Time<b>{formatDuration(projects.reduce((a, p) => a + p.totalTrackedSeconds, 0))}</b></div>
    </div>
  );
}
