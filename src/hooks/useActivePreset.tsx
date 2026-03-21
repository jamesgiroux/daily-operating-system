import { createContext, useContext, useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { RolePreset } from "@/types/preset";
import type React from "react";

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

  useEffect(() => {
    invoke<RolePreset | null>("get_active_preset")
      .then((p) => setPreset(p ?? null))
      .catch((err) => {
        console.error("get_active_preset failed:", err); // Expected: background init on mount
        setPreset(null);
      });

    const unlisten = listen("preset-changed", () => {
      invoke<RolePreset | null>("get_active_preset")
        .then((p) => setPreset(p ?? null))
        .catch((err) => {
          console.error("get_active_preset (event refresh) failed:", err); // Expected: event-driven refresh
        });
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

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
