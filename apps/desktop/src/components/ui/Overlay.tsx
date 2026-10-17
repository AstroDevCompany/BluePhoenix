import * as Dialog from "@radix-ui/react-dialog";
import type { ReactNode } from "react";

/**
 * Modal layer built on Radix Dialog: focus trap, Escape handling (topmost layer only),
 * scroll lock and outside-press dismissal come from the primitive.
 */
export function Overlay({
  children,
  onDismiss,
  label = "Dialog",
  guardEscape,
}: {
  children: ReactNode;
  onDismiss?: () => void;
  /** Accessible name announced for the modal. */
  label?: string;
  /** Return true to ignore this Escape press (e.g. an inline editor inside the dialog handles it). */
  guardEscape?: (event: KeyboardEvent) => boolean;
}) {
  return (
    <Dialog.Root
      open
      onOpenChange={(open) => {
        if (!open) onDismiss?.();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className="overlay" />
        <Dialog.Content
          className="overlay-layer"
          aria-describedby={undefined}
          onEscapeKeyDown={(e) => {
            if (!onDismiss || guardEscape?.(e)) e.preventDefault();
          }}
          onPointerDownOutside={(e) => {
            if (!onDismiss) e.preventDefault();
          }}
        >
          <Dialog.Title className="sr-only">{label}</Dialog.Title>
          {children}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
