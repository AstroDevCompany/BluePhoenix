export function applyAccent(accent: string | undefined) {
  document.documentElement.dataset.accent = accent === "teal" ? "teal" : "cyan";
}

export function applyWorkspaceColor(hex: string | undefined) {
  const root = document.documentElement;
  if (hex && /^#([0-9a-fA-F]{6}|[0-9a-fA-F]{3})$/.test(hex)) {
    root.style.setProperty("--workspace", hex);
  } else {
    root.style.removeProperty("--workspace");
  }
}
