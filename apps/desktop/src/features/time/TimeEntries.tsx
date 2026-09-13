import { useEffect, useState } from "react";
import type { TimeEntry } from "../../lib/types";
import { api, formatError } from "../../lib/ipc";
import { formatDate, formatDuration } from "../../lib/format";
import { useUi } from "../../stores/ui";

export function TimeEntries({
  projectId,
  onChanged,
  stage = false,
}: {
  projectId: string;
  onChanged: () => void;
  stage?: boolean;
}) {
  const [rows, setRows] = useState<TimeEntry[]>([]);
  const [start, setStart] = useState("");
  const [end, setEnd] = useState("");
  const [notes, setNotes] = useState("");
  const toast = useUi((s) => s.showToast);

  const load = () => {
    void api.listTimeEntries(projectId).then(setRows);
  };
  useEffect(() => {
    load();
  }, [projectId]);

  const addForm = (
    <div className="row">
      <input className="input" type="datetime-local" value={start} onChange={(e) => setStart(e.target.value)} aria-label="Start" />
      <input className="input" type="datetime-local" value={end} onChange={(e) => setEnd(e.target.value)} aria-label="End" />
      <input className="input" placeholder="Notes" value={notes} onChange={(e) => setNotes(e.target.value)} />
      <button
        className="btn"
        type="button"
        onClick={() => {
          if (!start || !end) return;
          void api
            .addManualEntry({
              projectId,
              startedAt: new Date(start).toISOString(),
              endedAt: new Date(end).toISOString(),
              notes,
            })
            .then(() => {
              setNotes("");
              load();
              onChanged();
            })
            .catch((e) => toast(formatError(e), "error"));
        }}
      >
        Add range
      </button>
    </div>
  );

  return (
    <div className={stage ? undefined : "section"}>
      {stage ? null : <h2 className="h2">Time entries</h2>}
      {rows.length === 0 ? (
        <div className="detail-empty">
          <p className="detail-empty-title">No entries yet</p>
          <p className="muted">Start the timer or add a manual range.</p>
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
      {stage ? (
        <details className="add-disclosure">
          <summary className="btn">Add range</summary>
          <div className="add-disclosure-body glass-panel">{addForm}</div>
        </details>
      ) : (
        <div className="row" style={{ marginTop: 10 }}>{addForm}</div>
      )}
    </div>
  );
}
