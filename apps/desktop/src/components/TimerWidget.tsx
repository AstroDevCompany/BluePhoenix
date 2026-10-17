import { useEffect, useState } from "react";
import { Square } from "lucide-react";
import { useRunningTimer } from "../lib/queries";
import { formatDuration } from "../lib/format";
import { stopTimer } from "../lib/projectActions";
import { elapsedFromStart } from "../features/time/TimerControls";

function useElapsed(startedAt: string | undefined) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!startedAt) return;
    setNow(Date.now());
    const id = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, [startedAt]);
  return startedAt ? elapsedFromStart(startedAt, now) : 0;
}

/** Live timer shown in the sidebar footer: click to jump to the project, square to stop. */
export function TimerWidget({ onOpenProject }: { onOpenProject: (projectId: string) => void }) {
  const { data: timer } = useRunningTimer();
  const active = timer && !timer.entry.endedAt ? timer : null;
  const elapsed = useElapsed(active?.entry.startedAt);
  if (!active) return null;
  if (!active.isLocalDevice) {
    return <div className="timer-widget muted">Timer running on another device</div>;
  }
  return (
    <div className="timer-widget" role="status" aria-live="off">
      <button type="button" className="timer-widget-main" onClick={() => onOpenProject(active.entry.projectId)}>
        <span className="timer-pulse" aria-hidden />
        <span className="timer-widget-text">
          <span className="timer-widget-name">{active.projectName}</span>
          <span className="timer-widget-time">{formatDuration(elapsed)}</span>
        </span>
      </button>
      <button type="button" className="btn icon ghost" aria-label="Stop timer" onClick={() => void stopTimer()}>
        <Square size={13} />
      </button>
    </div>
  );
}
