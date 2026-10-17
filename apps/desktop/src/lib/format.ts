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

const ACTIVITY_LABELS: Record<string, string> = {
  "todo.created": "TODO created",
  "todo.completed": "TODO completed",
  "version.released": "Version released",
  "topic.completed": "Topic completed",
  "lesson.attended": "Lesson attended",
  "exam.passed": "Exam passed",
  "exam.failed": "Exam failed",
  "exam.attempted": "Exam recorded",
};

export function formatActivityEvent(eventType: string) {
  if (ACTIVITY_LABELS[eventType]) return ACTIVITY_LABELS[eventType];
  return eventType.replaceAll(".", " · ").replaceAll("_", " ").replace(/^\w/, (c) => c.toUpperCase());
}

/** Compact relative time such as "5m ago" or "3d ago". */
export function formatRelative(value?: string | null, nowMs = Date.now()) {
  if (!value) return "";
  const t = Date.parse(value);
  if (Number.isNaN(t)) return "";
  const seconds = Math.max(0, Math.floor((nowMs - t) / 1000));
  if (seconds < 60) return "just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;
  const months = Math.floor(days / 30);
  return months < 12 ? `${months}mo ago` : `${Math.floor(months / 12)}y ago`;
}
