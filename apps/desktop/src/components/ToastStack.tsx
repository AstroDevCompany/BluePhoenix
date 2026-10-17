import { X } from "lucide-react";
import { useUi } from "../stores/ui";

export function ToastStack() {
  const toasts = useUi((s) => s.toasts);
  const dismiss = useUi((s) => s.dismissToast);
  if (toasts.length === 0) return null;
  return (
    <div className="toast-stack" role="region" aria-label="Notifications">
      {toasts.map((t) => (
        <div key={t.id} className={`toast ${t.tone === "error" ? "error" : ""}`} role={t.tone === "error" ? "alert" : "status"}>
          <span className="toast-message">{t.message}</span>
          {t.action ? (
            <button
              type="button"
              className="btn ghost toast-action"
              onClick={() => {
                t.action?.run();
                dismiss(t.id);
              }}
            >
              {t.action.label}
            </button>
          ) : null}
          <button type="button" className="toast-close" aria-label="Dismiss" onClick={() => dismiss(t.id)}>
            <X size={13} />
          </button>
        </div>
      ))}
    </div>
  );
}
