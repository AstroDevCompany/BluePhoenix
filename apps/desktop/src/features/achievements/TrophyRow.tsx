import { useEffect, useRef, useState } from "react";
import type { Unlock } from "../../lib/types";
import { formatDate, achievementSrc } from "../../lib/format";

export function TrophyRow({ items }: { items: Unlock[] }) {
  const visible = items.slice(0, 6);
  const extra = items.length - visible.length;
  if (items.length === 0) {
    return <div className="trophy-row muted" style={{ fontSize: 12 }}>No trophies yet</div>;
  }
  return (
    <div className="trophy-row row" onClick={(e) => e.stopPropagation()} onKeyDown={(e) => e.stopPropagation()}>
      {visible.map((item) => (
        <Trophy key={item.id} item={item} />
      ))}
      {extra > 0 ? <span className="badge">+{extra}</span> : null}
    </div>
  );
}

function Trophy({ item }: { item: Unlock }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLButtonElement>(null);
  const [pos, setPos] = useState({ top: 0, left: 0 });
  useEffect(() => {
    if (!open || !ref.current) return;
    const rect = ref.current.getBoundingClientRect();
    const width = 240;
    const left = Math.min(window.innerWidth - width - 12, Math.max(12, rect.left + rect.width / 2 - width / 2));
    const top = rect.top < 220 ? rect.bottom + 8 : rect.top - 188;
    setPos({ top, left });
  }, [open]);
  return (
    <>
      <button
        ref={ref}
        type="button"
        className="btn ghost"
        style={{ width: 36, height: 36, padding: 0 }}
        onMouseEnter={() => setOpen(true)}
        onMouseLeave={() => setOpen(false)}
        onFocus={() => setOpen(true)}
        onBlur={() => setOpen(false)}
        aria-label={item.name}
      >
        <img src={achievementSrc(item.icon)} alt="" width={28} height={28} />
      </button>
      {open ? (
        <div className="achievement-tip" style={{ position: "fixed", top: pos.top, left: pos.left, zIndex: 80 }} role="tooltip">
          <img src={achievementSrc(item.icon)} alt="" width={64} height={64} />
          <div className="rainbow-text" style={{ marginTop: 10 }}>{item.name.toUpperCase()}</div>
          <p className="muted" style={{ margin: "8px 0 6px" }}>{item.description}</p>
          <div className="muted" style={{ fontSize: 11 }}>Earned {formatDate(item.earnedAt)}</div>
        </div>
      ) : null}
    </>
  );
}
