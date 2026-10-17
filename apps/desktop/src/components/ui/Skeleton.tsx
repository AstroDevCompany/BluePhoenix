/** Placeholder blocks shown while a list loads, so the layout does not jump when data arrives. */
export function SkeletonCards({ count = 4 }: { count?: number }) {
  return (
    <div className="grid-cards" aria-busy="true" aria-label="Loading">
      {Array.from({ length: count }, (_, i) => (
        <div key={i} className="skeleton skeleton-card" />
      ))}
    </div>
  );
}

export function SkeletonRows({ count = 5 }: { count?: number }) {
  return (
    <div className="skeleton-rows" aria-busy="true" aria-label="Loading">
      {Array.from({ length: count }, (_, i) => (
        <div key={i} className="skeleton skeleton-row" />
      ))}
    </div>
  );
}
