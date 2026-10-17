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

export type ThemeSetting = "dark" | "light" | "system";

let mediaQuery: MediaQueryList | null = null;
let mediaListener: (() => void) | null = null;

function resolve(setting: ThemeSetting): "dark" | "light" {
  if (setting === "system") {
    return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  }
  return setting === "light" ? "light" : "dark";
}

/** Applies the saved theme; "system" follows the OS and keeps following it while the app runs. */
export function applyTheme(value: string | undefined) {
  const setting: ThemeSetting = value === "light" || value === "system" ? value : "dark";
  document.documentElement.dataset.theme = resolve(setting);
  if (mediaQuery && mediaListener) mediaQuery.removeEventListener("change", mediaListener);
  mediaQuery = null;
  mediaListener = null;
  if (setting === "system") {
    mediaQuery = window.matchMedia("(prefers-color-scheme: light)");
    mediaListener = () => {
      document.documentElement.dataset.theme = resolve("system");
    };
    mediaQuery.addEventListener("change", mediaListener);
  }
}
