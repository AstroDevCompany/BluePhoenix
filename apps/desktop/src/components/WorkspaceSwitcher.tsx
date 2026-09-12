import { useState } from "react";
import type { Category } from "../lib/types";

export function WorkspaceSwitcher({
  categories,
  current,
  onSelect,
}: {
  categories: Category[];
  current: Category;
  onSelect: (id: string) => void;
}) {
  const [open, setOpen] = useState(false);
  return (
    <div style={{ position: "relative" }}>
      <button className="workspace-switch" type="button" aria-haspopup="listbox" aria-expanded={open} onClick={() => setOpen((v) => !v)}>
        {current.name}
        <span className="muted">▼</span>
      </button>
      {open ? (
        <div className="glass-panel menu-pop" role="listbox" style={{ position: "absolute", insetInline: 0, top: 44, zIndex: 6, padding: 6 }}>
          {categories.map((c) => (
            <button
              key={c.id}
              type="button"
              role="option"
              aria-selected={c.id === current.id}
              className={`nav-btn ${c.id === current.id ? "active" : ""}`}
              onClick={() => {
                onSelect(c.id);
                setOpen(false);
              }}
            >
              {c.name}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}
