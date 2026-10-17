export function EmptyState({
  title,
  body,
  action,
  compact,
}: {
  title: string;
  body: string;
  action?: { label: string; onClick: () => void };
  /** Smaller, quieter variant for empty tabs and lists inside a page. */
  compact?: boolean;
}) {
  return (
    <div className={`empty ${compact ? "compact" : ""}`} role="status">
      <h2 className="h2">{title}</h2>
      <p className="muted">{body}</p>
      {action ? (
        <button className="btn primary" type="button" onClick={action.onClick}>
          {action.label}
        </button>
      ) : null}
    </div>
  );
}
