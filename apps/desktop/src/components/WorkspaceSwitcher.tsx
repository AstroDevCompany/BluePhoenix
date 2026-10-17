import { Check, ChevronsUpDown } from "lucide-react";
import type { Category } from "../lib/types";
import { Menu, MenuContent, MenuItem, MenuTrigger } from "./ui/Menu";
import { formatCombo } from "../lib/shortcuts";

export function WorkspaceSwitcher({
  categories,
  current,
  onSelect,
}: {
  categories: Category[];
  current: Category;
  onSelect: (id: string) => void;
}) {
  return (
    <Menu>
      <MenuTrigger asChild>
        <button className="workspace-switch" type="button" aria-label={`Workspace: ${current.name}`}>
          <span className="workspace-dot" aria-hidden />
          <span className="workspace-name">{current.name}</span>
          <ChevronsUpDown size={14} className="muted" aria-hidden />
        </button>
      </MenuTrigger>
      <MenuContent className="workspace-menu" style={{ minWidth: "var(--radix-dropdown-menu-trigger-width)" }}>
        {categories.map((c, i) => (
          <MenuItem
            key={c.id}
            icon={c.id === current.id ? <Check size={14} /> : undefined}
            hint={i < 9 ? formatCombo(`mod+${i + 1}`) : undefined}
            onSelect={() => onSelect(c.id)}
          >
            {c.name}
          </MenuItem>
        ))}
      </MenuContent>
    </Menu>
  );
}
