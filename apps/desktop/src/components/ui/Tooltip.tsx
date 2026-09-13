import {
  useLayoutEffect,
  useRef,
  useState,
  type ButtonHTMLAttributes,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";

const GAP = 8;
const BOTTOM_GAP = 12;
const MARGIN = 12;

export type TooltipPlacement = "top" | "bottom";

function placeTooltip(
  anchor: DOMRect,
  tipWidth: number,
  tipHeight: number,
  placement: TooltipPlacement,
) {
  const below = anchor.bottom + (placement === "bottom" ? BOTTOM_GAP : GAP);
  const above = anchor.top - tipHeight - GAP;
  const top =
    placement === "bottom"
      ? Math.min(below, window.innerHeight - tipHeight - MARGIN)
      : above < MARGIN
        ? below
        : above;
  const left = Math.min(
    window.innerWidth - tipWidth - MARGIN,
    Math.max(MARGIN, anchor.left + anchor.width / 2 - tipWidth / 2),
  );
  return { top, left };
}

export function Tooltip({
  content,
  children,
  rich,
  placement = "top",
}: {
  content: ReactNode;
  children: ReactNode;
  rich?: boolean;
  placement?: TooltipPlacement;
}) {
  const hostRef = useRef<HTMLSpanElement>(null);
  const tipRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [coords, setCoords] = useState<{ top: number; left: number } | null>(null);

  useLayoutEffect(() => {
    if (!open) {
      setCoords(null);
      return;
    }
    const update = () => {
      const host = hostRef.current;
      const tip = tipRef.current;
      if (!host || !tip) return;
      setCoords(placeTooltip(host.getBoundingClientRect(), tip.offsetWidth, tip.offsetHeight, placement));
    };
    update();
    const extra = requestAnimationFrame(update);
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    const ro = new ResizeObserver(update);
    if (tipRef.current) ro.observe(tipRef.current);
    return () => {
      cancelAnimationFrame(extra);
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
      ro.disconnect();
    };
  }, [open, placement]);

  return (
    <span
      className="tooltip-host"
      ref={hostRef}
      onMouseEnter={() => setOpen(true)}
      onMouseLeave={() => setOpen(false)}
      onFocusCapture={() => setOpen(true)}
      onBlurCapture={() => setOpen(false)}
    >
      {children}
      {open
        ? createPortal(
            <div
              ref={tipRef}
              role="tooltip"
              className={rich ? "ui-tooltip achievement-tip" : "ui-tooltip"}
              data-placement={placement}
              style={{
                top: coords?.top ?? 0,
                left: coords?.left ?? 0,
                visibility: coords ? "visible" : "hidden",
              }}
            >
              {content}
            </div>,
            document.body,
          )
        : null}
    </span>
  );
}

export function IconButton({
  label,
  className = "btn icon",
  type = "button",
  children,
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { label: string }) {
  return (
    <Tooltip content={label}>
      <button type={type} className={className} aria-label={label} {...props}>
        {children}
      </button>
    </Tooltip>
  );
}
