import { createContext, useContext, useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type React from "react";

export type Personality = "professional" | "friendly" | "playful";

const VALID_PERSONALITIES = new Set<string>(["professional", "friendly", "playful"]);

interface PersonalityContextValue {
  personality: Personality;
  userName: string | null;
  /** Update personality — call from SettingsPage after successful backend save. */
  setPersonality: (p: Personality) => void;
}

const PersonalityContext = createContext<PersonalityContextValue>({
  personality: "professional",
  userName: null,
  setPersonality: () => {},
});

/**
 * Provider that loads personality + userName once at app root.
 * Eliminates N+1 get_config IPC calls that the per-page hook created.
 *
 * Mount in RootLayout. Consumers use usePersonalityContext().
 */
export function PersonalityProvider({ children }: { children: React.ReactNode }) {
  const [personality, setPersonality] = useState<Personality>("professional");
  const [userName, setUserName] = useState<string | null>(null);

  useEffect(() => {
    invoke<{ personality?: string; userName?: string }>("get_config")
      .then((c) => {
        const p = c.personality;
        if (p && VALID_PERSONALITIES.has(p)) {
          setPersonality(p as Personality);
        }
        setUserName(c.userName ?? null);
      })
      .catch((err) => {
        console.error("get_config (personality) failed:", err);
      });
  }, []);

  const handleSetPersonality = useCallback((p: Personality) => {
    setPersonality(p);
  }, []);

  return (
    <PersonalityContext.Provider
      value={{ personality, userName, setPersonality: handleSetPersonality }}
    >
      {children}
    </PersonalityContext.Provider>
  );
}

/**
 * Returns personality + userName from the centralized context.
 * Used by UI chrome (empty states, loading messages).
 * Never used for intelligence content.
 */
export function usePersonality(): PersonalityContextValue {
  return useContext(PersonalityContext);
}
