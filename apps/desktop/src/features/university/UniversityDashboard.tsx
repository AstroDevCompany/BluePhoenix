import type { GpaDashboard } from "../../lib/types";
import { formatDuration } from "../../lib/format";

export function UniversityDashboard({ dash }: { dash: GpaDashboard }) {
  const average = dash.usedWeighted && dash.weightedAverage != null
    ? dash.weightedAverage
    : dash.simpleAverage;
  return (
    <>
      <div className="stat-grid">
        <div className="stat">Overall average<b>{average != null ? `${Number(average).toFixed(1)} / 30` : "—"}</b></div>
        <div className="stat">CFU<b>{Number(dash.completedCfu)} / {Number(dash.totalCfu) || 180}</b></div>
        <div className="stat">Study time<b>{formatDuration(Number(dash.studySeconds ?? 0))}</b></div>
        <div className="stat">Attendance<b>{Number(dash.attendancePercent ?? 0).toFixed(0)}%</b></div>
      </div>
      <p className="muted">
        GPA uses stored final grades only. {dash.usedWeighted ? "CFU-weighted average." : "Simple average — some courses are missing CFU."}
      </p>
      <p className="muted">Included courses: {dash.included.join(", ") || "none with a stored final grade"}.</p>
      {dash.excludedMissingFinal.length ? <p className="muted">Missing final grade: {dash.excludedMissingFinal.join(", ")}</p> : null}
      {dash.excludedFailed.length ? <p className="muted">Excluded failed: {dash.excludedFailed.join(", ")}</p> : null}
    </>
  );
}
