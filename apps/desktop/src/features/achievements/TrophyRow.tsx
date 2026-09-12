import type { Unlock } from "../../lib/types";
import { formatDate, achievementSrc } from "../../lib/format";
import { Tooltip } from "../../components/ui/Tooltip";

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
  return (
    <Tooltip
      rich
      content={
        <>
          <img src={achievementSrc(item.icon)} alt="" width={64} height={64} />
          <div className="rainbow-text" style={{ marginTop: 10 }}>{item.name.toUpperCase()}</div>
          <p className="muted" style={{ margin: "8px 0 6px" }}>{item.description}</p>
          <div className="muted" style={{ fontSize: 11 }}>Earned {formatDate(item.earnedAt)}</div>
        </>
      }
    >
      <button
        type="button"
        className="btn ghost"
        style={{ width: 36, height: 36, padding: 0 }}
        aria-label={item.name}
      >
        <img src={achievementSrc(item.icon)} alt="" width={28} height={28} />
      </button>
    </Tooltip>
  );
}
