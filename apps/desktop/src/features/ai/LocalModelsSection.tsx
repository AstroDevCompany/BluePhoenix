import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, formatError } from "../../lib/ipc";
import { Overlay } from "../../components/ui/Overlay";
import { settingsPayload } from "./settingsPayload";
import type { AiStatus, LocalModel } from "../../lib/types";

function formatBytes(bytes?: number | null) {
  if (!bytes || bytes <= 0) return "";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value >= 10 || unit === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[unit]}`;
}

function engineLabel(status: AiStatus) {
  const engine = status.localEngine;
  const active = status.localModels.find((m) => m.id === (engine.modelId ?? status.localModelId));
  const name = active?.name;
  switch (engine.state) {
    case "loading":
      return name ? `Loading ${name}…` : "Loading model…";
    case "ready":
      return name ? `Ready (${name})` : "Ready";
    case "busy":
      return name ? `Generating with ${name}` : "Generating…";
    case "error":
      return engine.error || "Local model error";
    default:
      return "Not loaded";
  }
}

export function LocalModelsSection({
  status,
  onStatus,
  toast,
}: {
  status: AiStatus;
  onStatus: (status: AiStatus) => void;
  toast: (message: string, kind?: "ok" | "error") => void;
}) {
  const [pendingPath, setPendingPath] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);
  const [progress, setProgress] = useState<{ bytes: number; total: number } | null>(null);
  const [removeTarget, setRemoveTarget] = useState<LocalModel | null>(null);
  const [deleteCopiedFile, setDeleteCopiedFile] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<{ bytes: number; total: number }>("local-model-import-progress", (ev) => {
      setProgress(ev.payload);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  const save = async (overrides: Parameters<typeof settingsPayload>[1]) => {
    await api.aiSaveSettings(settingsPayload(status, overrides));
    onStatus(await api.aiStatus());
  };

  const addModel = async () => {
    try {
      const path = await api.pickGgufFile();
      if (!path) return;
      setPendingPath(path);
    } catch (e) {
      toast(formatError(e), "error");
    }
  };

  const importModel = async (copy: boolean) => {
    if (!pendingPath) return;
    setImporting(true);
    setProgress(copy ? { bytes: 0, total: 1 } : null);
    try {
      const next = await api.localModelImport(pendingPath, copy);
      onStatus(next);
      setPendingPath(null);
      toast(copy ? "Model copied into BluePhoenix" : "Model linked");
    } catch (e) {
      toast(formatError(e), "error");
    } finally {
      setImporting(false);
      setProgress(null);
    }
  };

  const removeModel = async () => {
    if (!removeTarget) return;
    try {
      const next = await api.localModelRemove(removeTarget.id, deleteCopiedFile && removeTarget.managed);
      onStatus(next);
      setRemoveTarget(null);
      setDeleteCopiedFile(false);
      toast("Model removed");
    } catch (e) {
      toast(formatError(e), "error");
    }
  };

  const pct = progress && progress.total > 0 ? Math.min(100, Math.round((progress.bytes / progress.total) * 100)) : 0;

  return (
    <div className="divider-block">
      <h2 className="h2">Local models</h2>
      <p className="muted">
        Import GGUF files for llama.cpp. Linking keeps the original path; copying stores a file in the BluePhoenix models folder so it is not lost if the original is moved.
      </p>
      <div className="row">
        <button className="btn primary" type="button" onClick={() => void addModel()} disabled={importing}>
          Add model…
        </button>
        <button
          className="btn"
          type="button"
          disabled={status.localEngine.state === "unloaded" || importing}
          onClick={async () => {
            try {
              onStatus(await api.localModelUnload());
              toast("Model unloaded");
            } catch (e) {
              toast(formatError(e), "error");
            }
          }}
        >
          Unload now
        </button>
      </div>
      <p className={`muted ${status.localEngine.state === "error" ? "danger-text" : ""}`}>{engineLabel(status)}</p>
      {status.localModels.length === 0 ? (
        <p className="muted">No local models yet. Add a .gguf file to run AI without OpenRouter.</p>
      ) : (
        <div className="local-model-list">
          {status.localModels.map((model) => {
            const active = status.localModelId === model.id;
            return (
              <div key={model.id} className={`local-model-row ${active ? "is-active" : ""}`}>
                <div className="local-model-meta">
                  <div className="row" style={{ justifyContent: "space-between" }}>
                    <strong>{model.name}</strong>
                    <span className="badge">{model.managed ? "Copied" : "Linked"}</span>
                  </div>
                  <p className="muted local-model-path">{model.path}</p>
                  <p className="muted">
                    {[model.arch, formatBytes(model.sizeBytes), model.nCtxTrain ? `${model.nCtxTrain.toLocaleString()} ctx` : null]
                      .filter(Boolean)
                      .join(" · ")}
                  </p>
                </div>
                <div className="row">
                  <button
                    className={`btn ${active ? "primary" : ""}`}
                    type="button"
                    disabled={active}
                    onClick={() => void save({ localModelId: model.id })}
                  >
                    {active ? "In use" : "Use"}
                  </button>
                  <button className="btn danger" type="button" onClick={() => {
                    setDeleteCopiedFile(false);
                    setRemoveTarget(model);
                  }}>
                    Remove
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
      <label className="field">
        <span className="label">Context length</span>
        <input
          className="input"
          type="number"
          min={512}
          max={131072}
          value={status.localCtxLen}
          onChange={(e) => onStatus({ ...status, localCtxLen: Number(e.target.value) || 512 })}
          onBlur={() => void save({ localCtxLen: status.localCtxLen })}
        />
      </label>
      <label className="check-row">
        <input
          type="checkbox"
          checked={status.localGpuOffload}
          onChange={(e) => void save({ localGpuOffload: e.target.checked })}
        />
        Offload layers to GPU (remainder on CPU)
      </label>
      <label className="field">
        <span className="label">Unload after idle minutes (0 = keep loaded)</span>
        <input
          className="input"
          type="number"
          min={0}
          max={1440}
          value={status.localIdleUnloadMinutes}
          onChange={(e) => onStatus({ ...status, localIdleUnloadMinutes: Math.max(0, Number(e.target.value) || 0) })}
          onBlur={() => void save({ localIdleUnloadMinutes: status.localIdleUnloadMinutes })}
        />
      </label>

      {pendingPath ? (
        <Overlay onDismiss={importing ? undefined : () => setPendingPath(null)}>
          <div className="dialog" role="dialog" aria-modal="true" aria-labelledby="import-gguf-title">
            <h2 className="h2" id="import-gguf-title">Import GGUF</h2>
            <p className="muted local-model-path">{pendingPath}</p>
            <p className="muted">
              Copy the file into BluePhoenix so it is not lost if the original is moved, or link it in place.
            </p>
            {importing && progress ? (
              <div className="progress" aria-label="Copy progress">
                <span style={{ width: `${pct}%` }} />
              </div>
            ) : null}
            <div className="row" style={{ justifyContent: "flex-end", marginTop: 16 }}>
              <button className="btn" type="button" disabled={importing} onClick={() => setPendingPath(null)}>Cancel</button>
              <button className="btn" type="button" disabled={importing} onClick={() => void importModel(false)}>Link in place</button>
              <button className="btn primary" type="button" disabled={importing} onClick={() => void importModel(true)}>
                {importing ? `Copying${progress ? ` ${pct}%` : "…"}` : "Copy into BluePhoenix"}
              </button>
            </div>
          </div>
        </Overlay>
      ) : null}

      {removeTarget ? (
        <Overlay onDismiss={() => setRemoveTarget(null)}>
          <div className="dialog" role="dialog" aria-modal="true" aria-labelledby="remove-gguf-title">
            <h2 className="h2" id="remove-gguf-title">Remove model</h2>
            <p className="muted">Remove {removeTarget.name} from BluePhoenix? Linked files are never deleted.</p>
            {removeTarget.managed ? (
              <label className="check-row">
                <input
                  type="checkbox"
                  checked={deleteCopiedFile}
                  onChange={(e) => setDeleteCopiedFile(e.target.checked)}
                />
                Also delete the copied GGUF file
              </label>
            ) : null}
            <div className="row" style={{ justifyContent: "flex-end", marginTop: 16 }}>
              <button className="btn" type="button" onClick={() => setRemoveTarget(null)}>Cancel</button>
              <button className="btn danger" type="button" onClick={() => void removeModel()}>Remove</button>
            </div>
          </div>
        </Overlay>
      ) : null}
    </div>
  );
}
