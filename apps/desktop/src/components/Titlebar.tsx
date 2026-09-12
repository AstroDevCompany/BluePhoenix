import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Sparkles, Square, X } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import { useUi } from "../stores/ui";
import logo from "../assets/brand/logo.png";

export function Titlebar({ extra }: { extra?: ReactNode }) {
  const [os, setOs] = useState("windows");
  const aiOpen = useUi((s) => s.aiPanelOpen);
  const setAiPanel = useUi((s) => s.setAiPanel);
  useEffect(() => {
    setOs(navigator.userAgent.includes("Mac") ? "macos" : "windows");
  }, []);
  const win = () => getCurrentWindow();
  return (
    <header className="titlebar" data-platform={os} data-tauri-drag-region>
      <div className="titlebar-drag" data-tauri-drag-region>
        <img src={logo} alt="" className="titlebar-logo" />
        BluePhoenix
        {extra}
      </div>
      <button
        className={`btn ghost titlebar-ai ${aiOpen ? "primary" : ""}`}
        type="button"
        aria-pressed={aiOpen}
        aria-label="Toggle AI chat"
        onClick={() => setAiPanel(!aiOpen)}
      >
        <Sparkles size={14} /> AI
      </button>
      {os !== "macos" ? (
        <div className="titlebar-controls">
          <button type="button" onClick={() => win().minimize()} aria-label="Minimize">
            <Minus size={14} />
          </button>
          <button type="button" onClick={() => win().toggleMaximize()} aria-label="Maximize">
            <Square size={12} />
          </button>
          <button type="button" className="close" onClick={() => win().close()} aria-label="Close">
            <X size={14} />
          </button>
        </div>
      ) : null}
    </header>
  );
}
