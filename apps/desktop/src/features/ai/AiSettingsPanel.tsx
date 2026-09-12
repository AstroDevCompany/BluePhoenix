import { useEffect, useState } from "react";
import { api, formatError } from "../../lib/ipc";
import { useUi } from "../../stores/ui";
import type { AiStatus } from "../../lib/types";

export function AiSettingsPanel() {
  const toast = useUi((s) => s.showToast);
  const [status, setStatus] = useState<AiStatus | null>(null);
  const [key, setKey] = useState("");
  const [revealed, setRevealed] = useState<string | null>(null);
  const [testing, setTesting] = useState(false);

  const reload = () => {
    void api.aiStatus().then(setStatus).catch((e) => toast(formatError(e), "error"));
  };
  useEffect(() => { reload(); }, []);

  if (!status) return <p className="muted">Loading AI settings…</p>;

  const saveModels = async (models: string[]) => {
    const next = await api.aiSaveSettings({
      enabled: status.enabled,
      models,
      commitFollowStyle: status.commitFollowStyle,
      setupDismissed: status.setupDismissed,
    });
    setStatus({ ...status, ...next, hasKey: status.hasKey, maskedKey: status.maskedKey, signedIn: status.signedIn, cloudSecret: status.cloudSecret });
    toast("AI settings saved");
  };

  const move = (index: number, dir: -1 | 1) => {
    const next = [...status.models];
    const swap = index + dir;
    if (swap < 0 || swap >= next.length) return;
    [next[index], next[swap]] = [next[swap], next[index]];
    void saveModels(next);
  };

  return (
    <section className="glass-panel settings-card">
      <p className="muted">AI is optional. Search, Git, TODOs, and documents keep working without a key.</p>
      <label className="check-row">
        <input
          type="checkbox"
          checked={status.enabled}
          onChange={async (e) => {
            const next = await api.aiSaveSettings({
              enabled: e.target.checked,
              models: status.models,
              commitFollowStyle: status.commitFollowStyle,
              setupDismissed: true,
            });
            setStatus({ ...status, ...next });
          }}
        />
        Enable AI features
      </label>
      <label className="field">
        <span className="label">OpenRouter API key</span>
        {status.hasKey ? (
          <div className="row">
            <input className="input" readOnly value={revealed ?? status.maskedKey ?? "••••"} />
            <button className="btn" type="button" onClick={async () => {
              if (revealed) { setRevealed(null); return; }
              try { setRevealed(await api.aiRevealKey()); } catch (e) { toast(formatError(e), "error"); }
            }}>{revealed ? "Hide" : "Reveal"}</button>
            <button className="btn danger" type="button" onClick={async () => {
              setStatus(await api.aiClearKey());
              setRevealed(null);
              toast("Key removed");
            }}>Remove</button>
          </div>
        ) : (
          <div className="row">
            <input className="input" type="password" value={key} onChange={(e) => setKey(e.target.value)} placeholder="sk-or-…" />
            <button className="btn primary" type="button" onClick={async () => {
              try {
                setStatus(await api.aiSetKey(key));
                setKey("");
                toast("Key saved to the OS keychain");
              } catch (e) { toast(formatError(e), "error"); }
            }}>Save key</button>
          </div>
        )}
      </label>
      {status.hasKey ? (
        <div className="row">
          <input className="input" type="password" value={key} onChange={(e) => setKey(e.target.value)} placeholder="Replace key" />
          <button className="btn" type="button" onClick={async () => {
            try {
              setStatus(await api.aiSetKey(key));
              setKey("");
              setRevealed(null);
              toast("Key replaced");
            } catch (e) { toast(formatError(e), "error"); }
          }}>Replace</button>
        </div>
      ) : null}
      <button className="btn" type="button" disabled={testing} onClick={async () => {
        setTesting(true);
        try {
          const r = await api.aiTestConnection();
          toast(r.fallbackUsed ? `Connected via fallback (${r.model})` : `Connected (${r.model})`);
        } catch (e) {
          toast(formatError(e), "error");
        } finally {
          setTesting(false);
        }
      }}>{testing ? "Testing…" : "Test connection"}</button>
      {status.signedIn ? <p className="muted">Signed in: the key can sync as ciphertext only.</p> : <p className="muted">Local-only until you sign in. The wrap key never leaves this device.</p>}
      <div className="divider-block">
      <h2 className="h2">Models</h2>
      <p className="muted">Up to 8 OpenRouter identifiers such as <code>provider/model-name:free</code>. Only the first four non-empty slots are tried as fallback.</p>
      {status.models.map((model, i) => (
        <div key={i} className="row">
          <span className="badge">{i < 4 ? `Active fallback ${i + 1}` : `Reserved ${i + 1}`}</span>
          <input
            className="input"
            value={model}
            placeholder="provider/model-name:free"
            onChange={(e) => {
              const next = [...status.models];
              next[i] = e.target.value;
              setStatus({ ...status, models: next });
            }}
            onBlur={() => void saveModels(status.models)}
          />
          <button className="btn" type="button" onClick={() => move(i, -1)} aria-label="Move up">↑</button>
          <button className="btn" type="button" onClick={() => move(i, 1)} aria-label="Move down">↓</button>
        </div>
      ))}
      </div>
      <label className="check-row">
        <input
          type="checkbox"
          checked={status.commitFollowStyle}
          onChange={async (e) => {
            const next = await api.aiSaveSettings({
              enabled: status.enabled,
              models: status.models,
              commitFollowStyle: e.target.checked,
              setupDismissed: true,
            });
            setStatus({ ...status, ...next });
          }}
        />
        Match recent commit message style
      </label>
    </section>
  );
}
