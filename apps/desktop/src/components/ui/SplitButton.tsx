import { ChevronDown } from "lucide-react";
import type { ReactNode } from "react";
import { Menu, MenuContent, MenuTrigger } from "./Menu";

/**
 * A primary action with an attached caret that opens a menu of alternatives
 * (for example "run in embedded terminal" vs "run in system terminal").
 */
export function SplitButton({
  icon,
  label,
  title,
  onClick,
  disabled,
  primary,
  compact,
  menuLabel = "More options",
  onMenuOpenChange,
  children,
}: {
  icon?: ReactNode;
  label?: string;
  title?: string;
  onClick: () => void;
  disabled?: boolean;
  primary?: boolean;
  compact?: boolean;
  menuLabel?: string;
  onMenuOpenChange?: (open: boolean) => void;
  children: ReactNode;
}) {
  return (
    <div className={`split-btn ${compact ? "compact" : ""}`}>
      <button
        type="button"
        className={`btn split-main ${primary ? "primary" : ""}`}
        onClick={onClick}
        disabled={disabled}
        title={title}
        aria-label={label ? undefined : title}
      >
        {icon}
        {label ? <span>{label}</span> : null}
      </button>
      <Menu onOpenChange={onMenuOpenChange}>
        <MenuTrigger asChild>
          <button
            type="button"
            className={`btn split-caret ${primary ? "primary" : ""}`}
            aria-label={menuLabel}
            disabled={disabled}
          >
            <ChevronDown size={13} />
          </button>
        </MenuTrigger>
        <MenuContent align="end">{children}</MenuContent>
      </Menu>
    </div>
  );
}
