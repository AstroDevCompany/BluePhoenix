import { useEffect, useRef } from "react";
import type { Category } from "../lib/types";
import { useOpenTransition } from "./ui/useOpenTransition";

export function WorkspaceSwitcher({
  categories,
  current,
  onSelect,
}: {
  categories: Category[];
  current: Category;
  onSelect: (id: string) => void;
}) {
  const { open, visible, hide, toggle } = useOpenTransition();
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onDoc = (e: MouseEvent) => {
      if (!root.current?.contains(e.target as Node)) hide();
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [hide]);

  return (
    <div ref={root} style={{ position: "relative" }}>
      <button
        className="workspace-switch"
        type="button"
        aria-haspopup="listbox"
        aria-expanded={open}
        onClick={toggle}
      >
        {current.name}
        <span className="muted">▼</span>
      </button>
      {visible ? (
        <div
          className={`switcher-menu menu-pop ${open ? "is-open" : "is-closing"}`}
          role="listbox"
        >
          {categories.map((c) => (
            <button
              key={c.id}
              type="button"
              role="option"
              aria-selected={c.id === current.id}
              className={`nav-btn ${c.id === current.id ? "active" : ""}`}
              onClick={() => {
                onSelect(c.id);
                hide();
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
