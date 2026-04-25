import { createContext, useCallback, useContext, useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { RolePreset } from "@/types/preset";
import type React from "react";
import { useTauriEvent } from "./useTauriEvent";

const ActivePresetContext = createContext<RolePreset | null>(null);

/**
 * Provider that loads the active role preset once at app root
 * and re-fetches whenever the backend emits "preset-changed".
 *
 * Mount alongside PersonalityProvider in RootLayout.
 * Consumers use useActivePreset().
 */
export function ActivePresetProvider({ children }: { children: React.ReactNode }) {
  const [preset, setPreset] = useState<RolePreset | null>(null);

  const refreshPreset = useCallback(() => {
    invoke<RolePreset | null>("get_active_preset")
      .then((p) => setPreset(p ?? null))
      .catch((err) => {
        console.error("get_active_preset failed:", err); // Expected: background init on mount
        setPreset(null);
      });
  }, []);

  useEffect(() => {
    refreshPreset();
  }, [refreshPreset]);

  useTauriEvent("preset-changed", refreshPreset);

  return (
    <ActivePresetContext.Provider value={preset}>
      {children}
    </ActivePresetContext.Provider>
  );
}

/**
 * Returns the active role preset from context.
 * Returns null if no preset is configured.
 */
export function useActivePreset(): RolePreset | null {
  return useContext(ActivePresetContext);
}
