import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Search, Sparkles, Square, X } from "lucide-react";
import { useState, type ReactNode } from "react";
import { useUi } from "../stores/ui";
import { Tooltip } from "./ui/Tooltip";
import logo from "../assets/brand/logo.png";
import { formatCombo } from "../lib/shortcuts";

function detectOs() {
  if (typeof navigator === "undefined") return "windows";
  return /mac/i.test(navigator.userAgent) || navigator.platform === "MacIntel" ? "macos" : "windows";
}

export function Titlebar({ extra, showSearch = true }: { extra?: ReactNode; showSearch?: boolean }) {
  const [os] = useState(detectOs);
  const aiOpen = useUi((s) => s.aiPanelOpen);
  const setAiPanel = useUi((s) => s.setAiPanel);
  const openPalette = useUi((s) => s.openPalette);
  const win = () => getCurrentWindow();
  return (
    <header className="titlebar" data-platform={os}>
      {os === "macos" ? <div className="titlebar-lights" aria-hidden /> : null}
      <div className="titlebar-drag" data-tauri-drag-region>
        <img src={logo} alt="" className="titlebar-logo" />
        <span className="titlebar-wordmark">BluePhoenix</span>
        {extra}
      </div>
      {showSearch ? (
        <button className="titlebar-search" type="button" onClick={() => openPalette("all")} aria-label="Search or run a command">
          <Search size={13} aria-hidden />
          <span>Search or jump to…</span>
          <kbd className="kbd">{formatCombo("mod+k")}</kbd>
        </button>
      ) : null}
      <div className="titlebar-spacer" data-tauri-drag-region />
      <Tooltip content={aiOpen ? "Hide AI chat" : "Show AI chat"} placement="bottom">
        <button
          className={`btn titlebar-ai ${aiOpen ? "primary" : "ghost"}`}
          type="button"
          aria-pressed={aiOpen}
          aria-label="Toggle AI chat"
          onClick={() => setAiPanel(!aiOpen)}
        >
          <Sparkles size={14} /> AI
        </button>
      </Tooltip>
      {os !== "macos" ? (
        <div className="titlebar-controls">
          <Tooltip content="Minimize" placement="bottom">
            <button type="button" onClick={() => win().minimize()} aria-label="Minimize">
              <Minus size={14} />
            </button>
          </Tooltip>
          <Tooltip content="Maximize" placement="bottom">
            <button type="button" onClick={() => win().toggleMaximize()} aria-label="Maximize">
              <Square size={12} />
            </button>
          </Tooltip>
          <Tooltip content="Close" placement="bottom">
            <button type="button" className="close" onClick={() => win().close()} aria-label="Close">
              <X size={14} />
            </button>
          </Tooltip>
        </div>
      ) : null}
    </header>
  );
}
