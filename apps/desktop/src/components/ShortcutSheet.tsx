import { useUi } from "../stores/ui";
import { Overlay } from "./ui/Overlay";
import { SHORTCUTS, formatCombo } from "../lib/shortcuts";

const GROUPS = ["Navigate", "Project", "General"] as const;

function comboLabel(keys: string) {
  return keys.includes("…") ? keys.replace("mod", formatCombo("mod")).replace("alt", formatCombo("alt")) : formatCombo(keys);
}

export function ShortcutSheet() {
  const open = useUi((s) => s.shortcutsOpen);
  const setOpen = useUi((s) => s.setShortcuts);
  if (!open) return null;
  return (
    <Overlay onDismiss={() => setOpen(false)} label="Keyboard shortcuts">
      <div className="dialog shortcut-sheet">
        <h2 className="h2">Keyboard shortcuts</h2>
        {GROUPS.map((group) => (
          <section key={group} className="shortcut-group">
            <h3 className="nav-section">{group.toUpperCase()}</h3>
            {SHORTCUTS.filter((s) => s.group === group).map((s) => (
              <div key={s.id} className="shortcut-row">
                <span>{s.label}</span>
                <kbd className="kbd">{comboLabel(s.keys)}</kbd>
              </div>
            ))}
          </section>
        ))}
        <div className="row" style={{ justifyContent: "flex-end" }}>
          <button className="btn" type="button" onClick={() => setOpen(false)}>Close</button>
        </div>
      </div>
    </Overlay>
  );
}
