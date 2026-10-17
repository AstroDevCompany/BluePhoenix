import { useQueryClient } from "@tanstack/react-query";
import { api, formatError } from "../../lib/ipc";
import { keys, useRunningTimer } from "../../lib/queries";
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
  const client = useQueryClient();
  const { data: timer } = useRunningTimer();
  const active = timer && timer.isLocalDevice && !timer.entry.endedAt ? timer : null;
  const runningHere = active?.entry.projectId === projectId;
  const runningElsewhere = Boolean(active) && !runningHere;

  const after = async () => {
    await client.invalidateQueries({ queryKey: keys.timer });
    await client.invalidateQueries({ queryKey: ["projects"] });
    onDone();
  };
  const fail = (e: unknown) => toast(formatError(e), "error");

  if (runningHere) {
    return (
      <>
        <button className="btn" type="button" onClick={() => api.pauseTimer().then(after).catch(fail)}>
          Pause
        </button>
        <button className="btn" type="button" onClick={() => api.stopTimer().then(after).catch(fail)}>
          Stop
        </button>
      </>
    );
  }

  return (
    <button
      className="btn primary"
      type="button"
      title={runningElsewhere ? `Stops the timer on ${active?.projectName} first` : undefined}
      onClick={() =>
        (async () => {
          if (runningElsewhere) await api.stopTimer();
          await api.startTimer(projectId, topicId);
        })()
          .then(after)
          .catch(fail)
      }
    >
      {runningElsewhere ? "Switch timer here" : label}
    </button>
  );
}
