import * as DropdownMenu from "@radix-ui/react-dropdown-menu";
import type { ComponentPropsWithoutRef, ReactNode } from "react";

/** Thin styled wrappers over Radix DropdownMenu so every menu shares keyboard handling and glass styling. */
export const Menu = DropdownMenu.Root;
export const MenuTrigger = DropdownMenu.Trigger;

export function MenuContent({
  children,
  className = "",
  align = "start",
  ...props
}: ComponentPropsWithoutRef<typeof DropdownMenu.Content>) {
  return (
    <DropdownMenu.Portal>
      <DropdownMenu.Content
        align={align}
        sideOffset={6}
        collisionPadding={12}
        className={`menu-content ${className}`}
        {...props}
      >
        {children}
      </DropdownMenu.Content>
    </DropdownMenu.Portal>
  );
}

export function MenuItem({
  children,
  icon,
  hint,
  danger,
  className = "",
  ...props
}: ComponentPropsWithoutRef<typeof DropdownMenu.Item> & {
  icon?: ReactNode;
  hint?: ReactNode;
  danger?: boolean;
}) {
  return (
    <DropdownMenu.Item className={`menu-item ${danger ? "danger" : ""} ${className}`} {...props}>
      {icon ? <span className="menu-item-icon">{icon}</span> : null}
      <span className="menu-item-label">{children}</span>
      {hint ? <kbd className="kbd">{hint}</kbd> : null}
    </DropdownMenu.Item>
  );
}

export function MenuLabel({ children }: { children: ReactNode }) {
  return <DropdownMenu.Label className="menu-label">{children}</DropdownMenu.Label>;
}

export function MenuSeparator() {
  return <DropdownMenu.Separator className="menu-sep" />;
}
