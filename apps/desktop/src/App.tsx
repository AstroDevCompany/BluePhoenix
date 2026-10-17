import { useCallback, useEffect, useState } from "react";
import type { Bootstrap } from "./lib/types";
import { api, formatError } from "./lib/ipc";
import { AppRouter, router } from "./appRouter";
import { ShellProvider } from "./lib/shellContext";
import { parsePath, pathFor } from "./components/router";
import { LoadingScreen } from "./components/LoadingScreen";
import { useUi } from "./stores/ui";
import { applyAccent } from "./lib/theme";
import { queryClient } from "./lib/queryClient";

function hideHtmlSplash() {
  document.getElementById("boot-splash")?.setAttribute("hidden", "");
}

export default function App() {
  const [bootstrap, setBootstrap] = useState<Bootstrap | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [minTimeElapsed, setMinTimeElapsed] = useState(false);

  const reload = useCallback(() => {
    api.bootstrap().then(setBootstrap).catch((e) => setError(formatError(e)));
  }, []);

  useEffect(() => {
    const id = window.setTimeout(() => setMinTimeElapsed(true), 700);
    return () => window.clearTimeout(id);
  }, []);

  useEffect(() => {
    void api.bootstrap().then((b) => {
      setBootstrap(b);
      const fallback = b.settings.defaultWorkspace ?? b.categories.find((c) => c.enabled)?.id ?? "";
      if (!b.onboardingComplete) {
        void router.navigate({ to: "/onboarding" });
      } else {
        const parsed = parsePath(window.location.pathname, fallback);
        void router.navigate({ to: pathFor(parsed) as never });
      }
      document.documentElement.dataset.reducedMotion = b.settings.reducedMotion ? "true" : "false";
      document.documentElement.dataset.os = navigator.userAgent.includes("Mac") ? "macos" : "windows";
      applyAccent(b.settings.accent);
    }).catch((e) => setError(formatError(e)));
  }, []);

  useEffect(() => {
    const id = window.setInterval(() => {
      void api.pushSync().then((status) => {
        setBootstrap((b) => (b ? { ...b, sync: status as Bootstrap["sync"] } : b));
      }).catch(() => undefined);
    }, 30_000);
    return () => window.clearInterval(id);
  }, []);

  const ready = Boolean(bootstrap) && minTimeElapsed;

  useEffect(() => {
    if (!ready || !bootstrap?.settings.automaticUpdates) return;
    void api.checkForUpdates().then((r) => {
      const result = r as { newer?: boolean; remote?: string };
      if (result.newer) {
        useUi.getState().showToast(`Update available${result.remote ? `: ${result.remote}` : ""}`);
      }
    }).catch(() => undefined);
  }, [ready, bootstrap?.settings.automaticUpdates]);

  useEffect(() => {
    if (ready || error) {
      hideHtmlSplash();
      document.body.classList.add("app-ready");
    }
  }, [ready, error]);

  if (error) {
    return <div className="main"><h1 className="h1">Unable to start</h1><p className="muted">{error}</p></div>;
  }
  if (!ready || !bootstrap) {
    return <LoadingScreen />;
  }

  return (
    <ShellProvider
      value={{
        bootstrap,
        reload,
        bump: () => {
          void queryClient.invalidateQueries();
          reload();
        },
      }}
    >
      <AppRouter />
    </ShellProvider>
  );
}
