import { useEffect, useState } from "react";
import { api, formatError } from "../../lib/ipc";
import { useUi } from "../../stores/ui";
import type { AiStatus } from "../../lib/types";
import { Overlay } from "../../components/ui/Overlay";
import { settingsPayload } from "./settingsPayload";

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
    await api.aiSaveSettings(settingsPayload(status, { setupDismissed: true }));
    setOpen(false);
  };

  return (
    <Overlay>
      <div className="dialog" role="dialog" aria-modal="true" aria-labelledby="ai-setup-title">
        <h2 className="h2" id="ai-setup-title">Optional AI</h2>
        <p className="muted">Add an OpenRouter key for chat, search interpretation, and software helpers — or skip this and import a local GGUF model later in Settings → AI. You can keep using BluePhoenix without either.</p>
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
    </Overlay>
  );
}
