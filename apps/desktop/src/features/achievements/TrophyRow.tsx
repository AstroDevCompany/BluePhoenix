import type { Unlock } from "../../lib/types";
import { formatDate, achievementSrc } from "../../lib/format";
import { Tooltip } from "../../components/ui/Tooltip";

export function TrophyRow({ items }: { items: Unlock[] }) {
  const visible = items.slice(0, 6);
  const extra = items.length - visible.length;
  if (items.length === 0) {
    return <div className="trophy-row muted text-sm">No trophies yet</div>;
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
          <div className="rainbow-text trophy-tip-name">{item.name.toUpperCase()}</div>
          <p className="muted trophy-tip-desc">{item.description}</p>
          <div className="muted text-xs">Earned {formatDate(item.earnedAt)}</div>
        </>
      }
    >
      <button
        type="button"
        className="btn ghost trophy-btn"
        aria-label={item.name}
      >
        <img src={achievementSrc(item.icon)} alt="" width={28} height={28} />
      </button>
    </Tooltip>
  );
}
