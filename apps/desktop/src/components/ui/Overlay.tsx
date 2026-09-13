import { useRef, type PointerEvent, type ReactNode } from "react";

export function Overlay({
  children,
  onDismiss,
}: {
  children: ReactNode;
  onDismiss?: () => void;
}) {
  const startedOnBackdrop = useRef(false);

  const onPointerDown = (e: PointerEvent<HTMLDivElement>) => {
    startedOnBackdrop.current = e.target === e.currentTarget;
  };

  const onPointerUp = (e: PointerEvent<HTMLDivElement>) => {
    const dismiss =
      Boolean(onDismiss) &&
      startedOnBackdrop.current &&
      e.target === e.currentTarget &&
      e.button === 0;
    startedOnBackdrop.current = false;
    if (dismiss) onDismiss?.();
  };

  return (
    <div
      className="overlay"
      onPointerDown={onPointerDown}
      onPointerUp={onPointerUp}
      onPointerCancel={() => {
        startedOnBackdrop.current = false;
      }}
    >
      {children}
    </div>
  );
}
