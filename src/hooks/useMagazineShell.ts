/**
 * useMagazineShell.ts
 *
 * React context for page → shell communication in the magazine layout.
 * Pages register their shell configuration (chapters, labels, colors)
 * and MagazinePageLayout consumes it. This inverts the dependency so the
 * router doesn't need to import page internals like CHAPTERS constants.
 */

import { createContext, useContext, useState, useEffect, useCallback } from "react";
import type { ChapterItem } from "@/components/layout/FloatingNavIsland";

export interface MagazineShellConfig {
  /** Publication label for FolioBar, e.g., "Account" */
  folioLabel: string;
  /** Atmosphere gradient color */
  atmosphereColor: "turmeric" | "terracotta" | "larkspur";
  /** Which nav icon is highlighted */
  activePage: "today" | "week" | "inbox" | "actions" | "people" | "accounts" | "settings";
  /** Back link for FolioBar detail pages */
  backLink?: { label: string; onClick: () => void };
  /** Chapter definitions for FloatingNavIsland scroll mode */
  chapters?: ChapterItem[];
  /** Actions slot for FolioBar */
  folioActions?: React.ReactNode;
}

interface MagazineShellContextValue {
  config: MagazineShellConfig | null;
  register: (config: MagazineShellConfig) => void;
  unregister: () => void;
}

const MagazineShellContext = createContext<MagazineShellContextValue>({
  config: null,
  register: () => {},
  unregister: () => {},
});

export function useMagazineShellProvider() {
  const [config, setConfig] = useState<MagazineShellConfig | null>(null);

  const register = useCallback((c: MagazineShellConfig) => setConfig(c), []);
  const unregister = useCallback(() => setConfig(null), []);

  return { config, register, unregister };
}

export { MagazineShellContext };

/**
 * Hook for pages to register their magazine shell configuration.
 * Call this in editorial page components to configure the shell.
 *
 * @example
 * useRegisterMagazineShell({
 *   folioLabel: "Account",
 *   atmosphereColor: "turmeric",
 *   activePage: "accounts",
 *   backLink: { label: "Accounts", onClick: () => navigate({ to: "/accounts" }) },
 *   chapters: CHAPTERS,
 * });
 */
export function useRegisterMagazineShell(config: MagazineShellConfig) {
  const ctx = useContext(MagazineShellContext);

  useEffect(() => {
    ctx.register(config);
    return () => ctx.unregister();
    // Re-register when config identity changes (new object = new config)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ctx.register, ctx.unregister, config]);
}

/**
 * Hook for MagazinePageLayout to read the current shell configuration.
 */
export function useMagazineShellConfig(): MagazineShellConfig | null {
  return useContext(MagazineShellContext).config;
}
