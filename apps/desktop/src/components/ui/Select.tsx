import { useEffect, useId, useRef } from "react";
import { useOpenTransition } from "./useOpenTransition";

export function Select({
  value,
  onChange,
  options,
  ariaLabel,
}: {
  value: string;
  onChange: (value: string) => void;
  options: { value: string; label: string }[];
  ariaLabel?: string;
}) {
  const { open, visible, hide, toggle } = useOpenTransition();
  const root = useRef<HTMLDivElement>(null);
  const listId = useId();
  const current = options.find((o) => o.value === value) ?? options[0];

  useEffect(() => {
    const onDoc = (e: MouseEvent) => {
      if (!root.current?.contains(e.target as Node)) hide();
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [hide]);

  return (
    <div className="select-root" ref={root}>
      <button
        type="button"
        className="select-trigger"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listId}
        aria-label={ariaLabel}
        onClick={toggle}
      >
        <span>{current?.label ?? "Select"}</span>
        <span className="muted">▾</span>
      </button>
      {visible ? (
        <ul
          className={`select-menu menu-pop ${open ? "is-open" : "is-closing"}`}
          role="listbox"
          id={listId}
        >
          {options.map((opt) => (
            <li key={opt.value}>
              <button
                type="button"
                role="option"
                aria-selected={opt.value === value}
                className={`select-option ${opt.value === value ? "active" : ""}`}
                onClick={() => {
                  onChange(opt.value);
                  hide();
                }}
              >
                {opt.label}
              </button>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
