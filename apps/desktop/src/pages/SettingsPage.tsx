import { useEffect, useState } from "react";
import { Select } from "../components/ui/Select";
import { AiSettingsPanel } from "../features/ai";
import type { Settings } from "../lib/types";
import { api, formatError } from "../lib/ipc";
import { useUi } from "../stores/ui";
import type { Bootstrap } from "../lib/types";
import { applyAccent } from "../lib/theme";

const TABS = ["appearance", "workspace", "updates", "developer", "account", "data", "ai"] as const;

export function SettingsPage({
  bootstrap,
  tab,
  onTab,
  onReload,
}: {
  bootstrap: Bootstrap;
  tab: string;
  onTab: (tab: string) => void;
  onReload: () => void;
}) {
  const [settings, setSettings] = useState<Settings>(bootstrap.settings);
  const toast = useUi((s) => s.showToast);
  const current = TABS.includes(tab as typeof TABS[number]) ? tab : "appearance";
  useEffect(() => { setSettings(bootstrap.settings); }, [bootstrap.settings]);

  const save = async (next: Settings) => {
    try {
      const saved = await api.saveSettings(next);
      setSettings(saved);
      toast("Settings saved");
      document.documentElement.dataset.reducedMotion = next.reducedMotion ? "true" : "false";
      applyAccent(next.accent);
      onReload();
    } catch (e) {
      toast(formatError(e), "error");
    }
  };

  return (
    <div>
      <h1 className="h1">Settings</h1>
      <div className="settings-tabs" role="tablist" aria-label="Settings">
        {TABS.map((t) => (
          <button
            key={t}
            type="button"
            role="tab"
            aria-selected={current === t}
            onClick={() => onTab(t)}
          >
            {t === "ai" ? "AI" : t[0].toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>
      <div key={current}>
      {current === "appearance" ? (
        <section className="glass-panel settings-card">
          <label className="field"><span className="label">Accent</span>
            <Select value={settings.accent} onChange={(accent) => void save({ ...settings, accent })} options={[{ value: "cyan", label: "Cyan" }, { value: "teal", label: "Teal" }]} />
          </label>
          <label className="check-row"><input type="checkbox" checked={settings.reducedMotion} onChange={(e) => void save({ ...settings, reducedMotion: e.target.checked })} /> Reduced motion</label>
        </section>
      ) : null}
      {current === "workspace" ? (
        <section className="glass-panel settings-card">
          <div className="choice-list">
          {bootstrap.categories.map((c) => (
            <label key={c.id} className="check-row">
              <input type="checkbox" checked={c.enabled} onChange={async (e) => {
                await api.setCategoryEnabled(c.id, e.target.checked);
                onReload();
              }} />
              {c.name}
            </label>
          ))}
          </div>
          <label className="field">
            <span className="label">Default workspace</span>
            <Select
              value={settings.defaultWorkspace ?? ""}
              onChange={(defaultWorkspace) => void save({ ...settings, defaultWorkspace })}
              options={bootstrap.categories.filter((c) => c.enabled).map((c) => ({ value: c.id, label: c.name }))}
            />
          </label>
          <label className="check-row">
            <input type="checkbox" checked={settings.includeFailedGrades} onChange={(e) => void save({ ...settings, includeFailedGrades: e.target.checked })} />
            Include failed exam grades in GPA
          </label>
          <CustomCategory onCreated={onReload} />
        </section>
      ) : null}
      {current === "updates" ? (
        <Updates settings={settings} onSave={save} />
      ) : null}
      {current === "developer" ? (
        <section className="glass-panel settings-card">
          <label className="field"><span className="label">Git executable</span><input className="input" value={settings.gitPath} onChange={(e) => setSettings({ ...settings, gitPath: e.target.value })} onBlur={() => void save(settings)} /></label>
          <label className="field"><span className="label">VS Code executable</span><input className="input" value={settings.vscodePath} onChange={(e) => setSettings({ ...settings, vscodePath: e.target.value })} onBlur={() => void save(settings)} /></label>
          <label className="field"><span className="label">Terminal</span>
            <Select
              value={settings.terminal}
              onChange={(terminal) => void save({ ...settings, terminal })}
              options={[
                { value: "default", label: "Default" },
                { value: "cmd", label: "Command Prompt" },
                { value: "wt", label: "Windows Terminal" },
              ]}
            />
          </label>
        </section>
      ) : null}
      {current === "account" ? (
        <Account />
      ) : null}
      {current === "ai" ? <AiSettingsPanel /> : null}
      {current === "data" ? (
        <section className="glass-panel settings-card">
          <p className="muted">Export a portable JSON copy of your local database. Cloud is never required.</p>
          <div className="row">
            <button className="btn primary" type="button" onClick={async () => {
              const data = await api.exportData();
              const blob = new Blob([JSON.stringify(data, null, 2)], { type: "application/json" });
              const url = URL.createObjectURL(blob);
              const a = document.createElement("a");
              a.href = url;
              a.download = "bluephoenix-export.json";
              a.click();
              URL.revokeObjectURL(url);
            }}>Export JSON</button>
            <label className="btn">
              Import JSON
              <input type="file" accept="application/json" hidden onChange={async (e) => {
                const file = e.target.files?.[0];
                if (!file) return;
                try {
                  const data = JSON.parse(await file.text());
                  await api.importData(data);
                  toast("Imported");
                  onReload();
                } catch (err) {
                  toast(formatError(err), "error");
                }
              }} />
            </label>
          </div>
        </section>
      ) : null}
      </div>
    </div>
  );
}

function Updates({ settings, onSave }: { settings: Settings; onSave: (s: Settings) => void }) {
  const [result, setResult] = useState<string>("");
  return (
    <section className="glass-panel settings-card">
      <label className="check-row">
        <input type="checkbox" checked={settings.automaticUpdates} onChange={(e) => onSave({ ...settings, automaticUpdates: e.target.checked })} />
        Check for updates on launch
      </label>
      <label className="check-row">
        <input type="checkbox" checked={Boolean(settings.launchAtStartup)} onChange={(e) => onSave({ ...settings, launchAtStartup: e.target.checked })} />
        Launch BluePhoenix with the system
      </label>
      <label className="field"><span className="label">Channel</span>
        <Select
          value={settings.updateChannel}
          onChange={(updateChannel) => onSave({ ...settings, updateChannel })}
          options={[{ value: "stable", label: "Stable" }, { value: "beta", label: "Beta" }]}
        />
      </label>
      <button className="btn" type="button" onClick={async () => {
        const r = await api.checkForUpdates() as { local: string; remote?: string; newer: boolean; error?: string };
        setResult(r.error ? r.error : r.newer ? `Update available: ${r.remote}` : `Up to date (${r.local})`);
      }}>Check now</button>
      {result ? <p className="muted">{result}</p> : null}
    </section>
  );
}

function Account() {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const toast = useUi((s) => s.showToast);
  return (
    <section className="glass-panel settings-card">
      <label className="field"><span className="label">Email</span><input className="input" value={email} onChange={(e) => setEmail(e.target.value)} /></label>
      <label className="field"><span className="label">Password</span><input className="input" type="password" value={password} onChange={(e) => setPassword(e.target.value)} /></label>
      <div className="row">
        <button className="btn" type="button" onClick={() => api.authLogin(email, password).then(() => toast("Signed in")).catch((e) => toast(formatError(e), "error"))}>Sign in</button>
        <button className="btn" type="button" onClick={() => api.authRegister(email, password).then(() => toast("Account created")).catch((e) => toast(formatError(e), "error"))}>Create account</button>
        <button className="btn" type="button" onClick={() => api.authForgot(email).then(() => toast("If the account exists, a reset was issued"))}>Reset password</button>
        <button className="btn" type="button" onClick={() => api.pushSync().then(() => toast("Sync complete")).catch((e) => toast(formatError(e), "error"))}>Sync now</button>
        <button className="btn danger" type="button" onClick={() => api.authLogout().then(() => toast("Signed out"))}>Log out</button>
      </div>
    </section>
  );
}

function CustomCategory({ onCreated }: { onCreated: () => void }) {
  const [name, setName] = useState("");
  const toast = useUi((s) => s.showToast);
  return (
    <div className="divider-block">
      <h2 className="h2">Custom workspace</h2>
      <p className="muted">Creates a generic category. It cannot grow exam or Git tables.</p>
      <div className="row">
        <input className="input" placeholder="Name" value={name} onChange={(e) => setName(e.target.value)} />
        <button className="btn" type="button" onClick={async () => {
          if (!name.trim()) return;
          try {
            await api.createCustomCategory(name, "sparkles", "#3dd6c6");
            setName("");
            toast("Workspace created");
            onCreated();
          } catch (e) {
            toast(formatError(e), "error");
          }
        }}>Add</button>
      </div>
    </div>
  );
}
