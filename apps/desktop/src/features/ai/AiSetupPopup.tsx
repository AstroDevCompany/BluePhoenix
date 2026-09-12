import { useEffect, useState } from "react";
import { api, formatError } from "../../lib/ipc";
import { useUi } from "../../stores/ui";
import type { AiStatus } from "../../lib/types";

export function AiSetupPopup() {
  const toast = useUi((s) => s.showToast);
  const [status, setStatus] = useState<AiStatus | null>(null);
  const [key, setKey] = useState("");
  const [open, setOpen] = useState(false);

  useEffect(() => {
    void api.aiStatus().then((s) => {
      setStatus(s);
      if (!s.setupDismissed) setOpen(true);
    }).catch(() => undefined);
  }, []);

  if (!open || !status) return null;

  const dismiss = async () => {
    await api.aiSaveSettings({
      enabled: status.enabled,
      models: status.models,
      commitFollowStyle: status.commitFollowStyle,
      setupDismissed: true,
    });
    setOpen(false);
  };

  return (
    <div className="overlay" onClick={() => undefined}>
      <div className="dialog" role="dialog" aria-modal="true" aria-labelledby="ai-setup-title" onClick={(e) => e.stopPropagation()}>
        <h2 className="h2" id="ai-setup-title">Optional AI</h2>
        <p className="muted">Add an OpenRouter key for chat, search interpretation, and software helpers. You can skip this and use BluePhoenix as usual.</p>
        <label className="field">
          <span className="label">OpenRouter API key</span>
          <input className="input" type="password" value={key} onChange={(e) => setKey(e.target.value)} placeholder="sk-or-…" />
        </label>
        <div className="row" style={{ justifyContent: "flex-end", marginTop: 16 }}>
          <button className="btn" type="button" onClick={() => void dismiss()}>Skip for now</button>
          <button className="btn primary" type="button" onClick={async () => {
            try {
              if (key.trim()) await api.aiSetKey(key.trim());
              await dismiss();
              toast("AI setup saved");
            } catch (e) {
              toast(formatError(e), "error");
            }
          }}>Set up AI</button>
        </div>
      </div>
    </div>
  );
}
