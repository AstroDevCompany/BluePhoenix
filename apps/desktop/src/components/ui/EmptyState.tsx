export function EmptyState({
  title,
  body,
  action,
}: {
  title: string;
  body: string;
  action?: { label: string; onClick: () => void };
}) {
  return (
    <div className="empty" role="status">
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
