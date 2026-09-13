import { useEffect, useState } from "react";
import type { TimeEntry } from "../../lib/types";
import { api, formatError } from "../../lib/ipc";
import { formatDate, formatDuration } from "../../lib/format";
import { useUi } from "../../stores/ui";

const MINUTE_PRESETS = [
  { label: "10 min", seconds: 10 * 60 },
  { label: "15 min", seconds: 15 * 60 },
  { label: "30 min", seconds: 30 * 60 },
];

const HOUR_PRESETS = [
  { label: "1 hour", seconds: 60 * 60 },
  { label: "2 hours", seconds: 2 * 60 * 60 },
  { label: "3 hours", seconds: 3 * 60 * 60 },
];

function rangeForDuration(seconds: number) {
  const endedAt = new Date();
  const startedAt = new Date(endedAt.getTime() - seconds * 1000);
  return { startedAt: startedAt.toISOString(), endedAt: endedAt.toISOString() };
}

export function TimeEntries({
  projectId,
  topicId,
  onChanged,
  stage = false,
}: {
  projectId: string;
  topicId?: string;
  onChanged: () => void;
  stage?: boolean;
}) {
  const [rows, setRows] = useState<TimeEntry[]>([]);
  const [start, setStart] = useState("");
  const [end, setEnd] = useState("");
  const [notes, setNotes] = useState("");
  const [busy, setBusy] = useState(false);
  const toast = useUi((s) => s.showToast);

  const load = () => {
    void api.listTimeEntries(projectId).then(setRows);
  };
  useEffect(() => {
    load();
  }, [projectId]);

  const addRange = (startedAt: string, endedAt: string, entryNotes?: string) => {
    setBusy(true);
    void api
      .addManualEntry({
        projectId,
        topicId: topicId || null,
        startedAt,
        endedAt,
        notes: entryNotes ?? "",
      })
      .then(() => {
        setNotes("");
        load();
        onChanged();
      })
      .catch((e) => toast(formatError(e), "error"))
      .finally(() => setBusy(false));
  };

  const addDuration = (seconds: number) => {
    const range = rangeForDuration(seconds);
    addRange(range.startedAt, range.endedAt);
  };

  const addForm = (
    <div className="row">
      <input className="input" type="datetime-local" value={start} onChange={(e) => setStart(e.target.value)} aria-label="Start" />
      <input className="input" type="datetime-local" value={end} onChange={(e) => setEnd(e.target.value)} aria-label="End" />
      <input className="input" placeholder="Notes" value={notes} onChange={(e) => setNotes(e.target.value)} />
      <button
        className="btn"
        type="button"
        disabled={busy || !start || !end}
        onClick={() => {
          if (!start || !end) return;
          addRange(new Date(start).toISOString(), new Date(end).toISOString(), notes);
        }}
      >
        Add range
      </button>
    </div>
  );

  const rangeDisclosure = (
    <details className="add-disclosure">
      <summary className="btn">Add a time range</summary>
      <div className="add-disclosure-body glass-panel">{addForm}</div>
    </details>
  );

  return (
    <div className={stage ? undefined : "section"}>
      {stage ? null : <h2 className="h2">Time entries</h2>}
      <section className="detail-section">
        <h3 className="detail-section-title">Add time</h3>
        <div className="field">
          <span className="label">Minutes</span>
          <div className="row">
            {MINUTE_PRESETS.map((preset) => (
              <button
                key={preset.seconds}
                className="btn"
                type="button"
                disabled={busy}
                onClick={() => addDuration(preset.seconds)}
              >
                {preset.label}
              </button>
            ))}
          </div>
        </div>
        <div className="field">
          <span className="label">Hours</span>
          <div className="row">
            {HOUR_PRESETS.map((preset) => (
              <button
                key={preset.seconds}
                className="btn"
                type="button"
                disabled={busy}
                onClick={() => addDuration(preset.seconds)}
              >
                {preset.label}
              </button>
            ))}
          </div>
        </div>
      </section>
      {rows.length === 0 ? (
        <div className="detail-empty">
          <p className="detail-empty-title">No entries yet</p>
          <p className="muted">Start the timer or add time above.</p>
        </div>
      ) : (
        rows.slice(0, 40).map((row) => (
          <div key={row.id} className="list-row spread">
            <span>
              {formatDate(row.startedAt)} · {formatDuration(row.seconds)}
              {row.topicTitle ? ` · ${row.topicTitle}` : ""}
              {row.notes ? ` · ${row.notes}` : ""}
            </span>
            <button
              className="btn ghost"
              type="button"
              onClick={() => api.updateTimeEntry({ id: row.id, notes: "" }).then(() => { load(); onChanged(); }).catch((e) => toast(formatError(e), "error"))}
            >
              Clear note
            </button>
          </div>
        ))
      )}
      {rangeDisclosure}
    </div>
  );
}
