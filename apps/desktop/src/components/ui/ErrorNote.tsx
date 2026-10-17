import { formatError } from "../../lib/ipc";

export function ErrorNote({ error, onRetry }: { error: unknown; onRetry?: () => void }) {
  return (
    <div className="error-note" role="alert">
      <span>{formatError(error)}</span>
      {onRetry ? (
        <button className="btn" type="button" onClick={onRetry}>Retry</button>
      ) : null}
    </div>
  );
}
