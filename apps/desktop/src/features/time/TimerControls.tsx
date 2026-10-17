import { useEffect, useState } from "react";
import { api, formatError } from "../../lib/ipc";
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
  const [runningHere, setRunningHere] = useState(false);

  const refresh = () => {
    void api.runningTimer().then((t) => {
      setRunningHere(Boolean(t?.isLocalDevice && t.entry.projectId === projectId && !t.entry.endedAt));
    }).catch(() => setRunningHere(false));
  };

  useEffect(() => {
    refresh();
    const id = window.setInterval(refresh, 4000);
    return () => window.clearInterval(id);
  }, [projectId]);

  const after = () => {
    refresh();
    onDone();
  };

  if (runningHere) {
    return (
      <>
        <button className="btn" type="button" onClick={() => api.pauseTimer().then(after).catch((e) => toast(formatError(e), "error"))}>
          Pause
        </button>
        <button className="btn" type="button" onClick={() => api.stopTimer().then(after).catch((e) => toast(formatError(e), "error"))}>
          Stop
        </button>
      </>
    );
  }

  return (
    <button
      className="btn primary"
      type="button"
      onClick={() => api.startTimer(projectId, topicId).then(after).catch((e) => toast(formatError(e), "error"))}
    >
      {label}
    </button>
  );
}
