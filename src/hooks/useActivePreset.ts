import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { RolePreset } from "@/types/preset";

/**
 * Fetch the active role preset from the backend (I312).
 * Returns null if no preset is configured or the call fails.
 * The result is cached for the component's lifetime.
 */
export function useActivePreset(): RolePreset | null {
  const [preset, setPreset] = useState<RolePreset | null>(null);

  useEffect(() => {
    invoke<RolePreset | null>("get_active_preset")
      .then((p) => setPreset(p ?? null))
      .catch(() => setPreset(null));
  }, []);

  return preset;
}
