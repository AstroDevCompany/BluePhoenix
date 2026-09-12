import { useEffect, useState } from "react";
import { api, formatError } from "../../lib/ipc";
import { formatDuration } from "../../lib/format";
import { useUi } from "../../stores/ui";

export function elapsedFromStart(startedAt: string, nowMs = Date.now()) {
  const start = Date.parse(startedAt);
  if (Number.isNaN(start)) return 0;
  return Math.max(0, Math.floor((nowMs - start) / 1000));
}

export function TimerControls({
  projectId,
  label,
  topicId,
  onDone,
}: {
  projectId: string;
  label: string;
  topicId?: string;
  onDone: () => void;
}) {
  const toast = useUi((s) => s.showToast);
  return (
    <>
      <button
        className="btn primary"
        type="button"
        onClick={() => api.startTimer(projectId, topicId).then(onDone).catch((e) => toast(formatError(e), "error"))}
      >
        {label}
      </button>
      <button className="btn" type="button" onClick={() => api.pauseTimer().then(onDone).catch((e) => toast(formatError(e), "error"))}>
        Pause
      </button>
      <button className="btn" type="button" onClick={() => api.stopTimer().then(onDone).catch((e) => toast(formatError(e), "error"))}>
        Stop
      </button>
    </>
  );
}

export function LiveTimerBadge() {
  const [label, setLabel] = useState<string | null>(null);
  useEffect(() => {
    let cancelled = false;
    let startedAt: string | null = null;
    let name = "";
    let local = true;
    const refresh = () => {
      void api.runningTimer().then((t) => {
        if (cancelled) return;
        if (!t) {
          startedAt = null;
          setLabel(null);
          return;
        }
        startedAt = t.entry.startedAt;
        name = t.projectName;
        local = t.isLocalDevice;
      });
    };
    refresh();
    const poll = window.setInterval(refresh, 5000);
    const tick = window.setInterval(() => {
      if (!startedAt) return;
      const text = local
        ? `${name} · ${formatDuration(elapsedFromStart(startedAt))}`
        : `Running on another device`;
      setLabel(text);
    }, 1000);
    return () => {
      cancelled = true;
      window.clearInterval(poll);
      window.clearInterval(tick);
    };
  }, []);
  if (!label) return null;
  return <span className="badge" aria-live="polite">{label}</span>;
}
