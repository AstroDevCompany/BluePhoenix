import { useRef, type ReactNode } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";

export function VirtualList<T>({
  items,
  estimateSize = 56,
  render,
}: {
  items: T[];
  estimateSize?: number;
  render: (item: T, index: number) => ReactNode;
}) {
  const parentRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => estimateSize,
    overscan: 8,
  });
  return (
    <div ref={parentRef} className="scroll" style={{ height: "min(70vh, 640px)" }}>
      <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
        {virtualizer.getVirtualItems().map((row) => (
          <div
            key={row.key}
            style={{
              position: "absolute",
              top: 0,
              left: 0,
              width: "100%",
              transform: `translateY(${row.start}px)`,
            }}
          >
            {render(items[row.index], row.index)}
          </div>
        ))}
      </div>
    </div>
  );
}
