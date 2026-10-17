import { createContext, useContext } from "react";
import type { Bootstrap } from "./types";

export type ShellApi = {
  bootstrap: Bootstrap;
  reload: () => void;
  /** Reloads bootstrap data and invalidates every cached query. */
  bump: () => void;
};

const Ctx = createContext<ShellApi | null>(null);

export function useShell() {
  const value = useContext(Ctx);
  if (!value) throw new Error("Shell context missing");
  return value;
}

export function ShellProvider({ value, children }: { value: ShellApi; children: React.ReactNode }) {
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}
