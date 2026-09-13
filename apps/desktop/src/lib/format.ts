export function formatDuration(seconds: number) {
  const s = Math.max(0, Math.floor(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h > 0) return `${h}h ${m.toString().padStart(2, "0")}m`;
  return `${m}m`;
}

export function formatDate(value?: string | null) {
  if (!value) return "—";
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return value;
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
}

export function achievementSrc(icon: string) {
  return `/assets/achievements/${icon}`;
}

const EXAM_STATUS_LABELS: Record<string, string> = {
  scheduled: "Scheduled",
  attempted: "Attempted",
  passed: "Passed",
  failed: "Failed",
  withdrawn: "Withdrawn",
  no_show: "No show",
};

export const EXAM_STATUS_OPTIONS = Object.entries(EXAM_STATUS_LABELS).map(([value, label]) => ({
  value,
  label,
}));

export function formatExamStatus(status: string) {
  return EXAM_STATUS_LABELS[status] ?? status.replaceAll("_", " ").replace(/^\w/, (c) => c.toUpperCase());
}
