import * as RTooltip from "@radix-ui/react-tooltip";
import type { ButtonHTMLAttributes, ReactNode } from "react";

export type TooltipPlacement = "top" | "bottom";

export function TooltipProvider({ children }: { children: ReactNode }) {
  return (
    <RTooltip.Provider delayDuration={350} skipDelayDuration={150}>
      {children}
    </RTooltip.Provider>
  );
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
  return (
    <RTooltip.Root>
      <RTooltip.Trigger asChild>
        <span className="tooltip-host">{children}</span>
      </RTooltip.Trigger>
      <RTooltip.Portal>
        <RTooltip.Content
          side={placement}
          sideOffset={8}
          collisionPadding={12}
          className={rich ? "ui-tooltip achievement-tip" : "ui-tooltip"}
          data-placement={placement}
        >
          {content}
        </RTooltip.Content>
      </RTooltip.Portal>
    </RTooltip.Root>
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
