import { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

interface UpdateState {
  available: boolean;
  version?: string;
  notes?: string;
  update?: Update;
  checking: boolean;
  installing: boolean;
  error?: string;
}

interface UpdateContextValue extends UpdateState {
  installAndRestart: () => Promise<void>;
  checkForUpdate: () => Promise<void>;
}

const UpdateContext = createContext<UpdateContextValue | null>(null);

export function UpdateProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<UpdateState>({
    available: false,
    checking: false,
    installing: false,
  });

  const checkForUpdate = useCallback(async () => {
    setState((s) => ({ ...s, checking: true, error: undefined }));
    try {
      const update = await check();
      if (update) {
        setState({
          available: true,
          version: update.version,
          notes: update.body ?? undefined,
          update,
          checking: false,
          installing: false,
        });
      } else {
        setState({ available: false, checking: false, installing: false });
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      console.warn("Update check failed:", message);
      setState({ available: false, checking: false, installing: false, error: message });
    }
  }, []);

  const installAndRestart = useCallback(async () => {
    if (!state.update) return;
    setState((s) => ({ ...s, installing: true }));
    try {
      // Store release notes before restart so WhatsNewModal can show them post-install
      if (state.notes) {
        localStorage.setItem("dailyos_release_notes", state.notes);
      }
      await state.update.downloadAndInstall();
      await relaunch();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setState((s) => ({ ...s, installing: false, error: message }));
    }
  }, [state.update]);

  // Silent check on mount
  useEffect(() => {
    checkForUpdate();
  }, [checkForUpdate]);

  return (
    <UpdateContext.Provider value={{ ...state, installAndRestart, checkForUpdate }}>
      {children}
    </UpdateContext.Provider>
  );
}

export function useUpdate(): UpdateContextValue {
  const ctx = useContext(UpdateContext);
  if (!ctx) {
    throw new Error("useUpdate must be used within an UpdateProvider");
  }
  return ctx;
}
