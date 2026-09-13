import { useState } from "react";
import { Select } from "../components/ui/Select";
import type { Bootstrap, Category } from "../lib/types";
import { api, formatError } from "../lib/ipc";
import { useUi } from "../stores/ui";

const STEPS = ["Account", "Workspace", "Default", "First item"];

export function Onboarding({
  bootstrap,
  onDone,
}: {
  bootstrap: Bootstrap;
  onDone: (workspaceId: string) => void;
}) {
  const [step, setStep] = useState(0);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [enabled, setEnabled] = useState<Record<string, boolean>>(
    Object.fromEntries(bootstrap.categories.map((c) => [c.id, c.enabled])),
  );
  const [workspace, setWorkspace] = useState(bootstrap.settings.defaultWorkspace ?? bootstrap.categories[0]?.id ?? "");
  const toast = useUi((s) => s.showToast);

  const saveEnabled = async () => {
    for (const c of bootstrap.categories) {
      const want = Boolean(enabled[c.id]);
      if (c.enabled !== want) await api.setCategoryEnabled(c.id, want);
    }
  };

  return (
    <div className="onboard-flow">
      <h1 className="h1">Welcome to BluePhoenix</h1>
      <p className="muted">A short setup. You can change everything later.</p>
      <div className="onboard-steps" aria-label={`Step ${step + 1} of ${STEPS.length}`}>
        {STEPS.map((label, i) => (
          <span key={label} className={i < step ? "is-done" : i === step ? "is-current" : ""} title={label} />
        ))}
      </div>
      <p className="faint">{STEPS[step]} · {step + 1}/{STEPS.length}</p>
      {step === 0 ? (
        <div className="glass-panel settings-card" style={{ marginTop: 20 }}>
          <h2 className="h2">Account</h2>
          <p className="muted">Create an account to sync later, or continue locally.</p>
          <label className="field"><span className="label">Email</span><input className="input" value={email} onChange={(e) => setEmail(e.target.value)} /></label>
          <label className="field"><span className="label">Password</span><input className="input" type="password" value={password} onChange={(e) => setPassword(e.target.value)} /></label>
          <div className="row" style={{ justifyContent: "flex-end" }}>
            <button className="btn" type="button" onClick={async () => {
              try {
                await api.authRegister(email, password);
                toast("Account created");
                setStep(1);
              } catch (e) {
                toast(formatError(e), "error");
              }
            }}>Create account</button>
            <button className="btn" type="button" onClick={async () => {
              try {
                await api.authLogin(email, password);
                toast("Signed in");
                setStep(1);
              } catch (e) {
                toast(formatError(e), "error");
              }
            }}>Sign in</button>
            <button className="btn primary" type="button" onClick={() => setStep(1)}>Continue locally</button>
          </div>
        </div>
      ) : null}
      {step === 1 ? (
        <div className="glass-panel settings-card" style={{ marginTop: 20 }}>
          <h2 className="h2">Workspace</h2>
          <div className="choice-list">
          {bootstrap.categories.map((c: Category) => (
            <label key={c.id} className="check-row">
              <input type="checkbox" checked={Boolean(enabled[c.id])} onChange={(e) => setEnabled((s) => ({ ...s, [c.id]: e.target.checked }))} />
              <span>{c.name}</span>
            </label>
          ))}
          </div>
          <div className="row" style={{ justifyContent: "flex-end" }}>
            <button className="btn primary" type="button" onClick={async () => { await saveEnabled(); setStep(2); }}>Continue</button>
          </div>
        </div>
      ) : null}
      {step === 2 ? (
        <div className="glass-panel settings-card" style={{ marginTop: 20 }}>
          <h2 className="h2">Default workspace</h2>
          <Select
            value={workspace}
            onChange={setWorkspace}
            options={bootstrap.categories.filter((c) => enabled[c.id]).map((c) => ({ value: c.id, label: c.name }))}
          />
          <div className="row" style={{ justifyContent: "flex-end", marginTop: 16 }}>
            <button className="btn primary" type="button" onClick={() => setStep(3)}>Continue</button>
          </div>
        </div>
      ) : null}
      {step === 3 ? (
        <FirstItem
          categoryId={workspace}
          categoryName={bootstrap.categories.find((c) => c.id === workspace)?.terminology.itemSingular ?? "item"}
          onSkip={async () => {
            await api.completeOnboarding(workspace);
            onDone(workspace);
          }}
          onCreated={async () => {
            await api.completeOnboarding(workspace);
            onDone(workspace);
          }}
        />
      ) : null}
    </div>
  );
}

function FirstItem({
  categoryId,
  categoryName,
  onSkip,
  onCreated,
}: {
  categoryId: string;
  categoryName: string;
  onSkip: () => Promise<void>;
  onCreated: () => Promise<void>;
}) {
  const [name, setName] = useState("");
  const toast = useUi((s) => s.showToast);
  return (
    <div className="glass-panel settings-card" style={{ marginTop: 20 }}>
      <h2 className="h2">First {categoryName.toLowerCase()}</h2>
      <p className="muted">Optional. You can skip and create it later from the home screen.</p>
      <label className="field"><span className="label">Name</span>
        <input className="input" value={name} onChange={(e) => setName(e.target.value)} />
      </label>
      <div className="row" style={{ justifyContent: "flex-end", marginTop: 16 }}>
        <button className="btn" type="button" onClick={() => void onSkip()}>Skip for now</button>
        <button className="btn primary" type="button" disabled={!name.trim()} onClick={async () => {
          try {
            await api.createProject({ categoryId, name });
            await onCreated();
          } catch (e) {
            toast(formatError(e), "error");
          }
        }}>Create and finish</button>
      </div>
    </div>
  );
}
