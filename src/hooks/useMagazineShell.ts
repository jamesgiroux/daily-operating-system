/**
 * useMagazineShell.ts
 *
 * React context for page → shell communication in the magazine layout.
 * Pages register their shell configuration (chapters, labels, colors)
 * and MagazinePageLayout consumes it. This inverts the dependency so the
 * router doesn't need to import page internals like CHAPTERS constants.
 */

import { createContext, useContext, useState, useEffect, useLayoutEffect, useCallback, useRef } from "react";
import type { ChapterItem } from "@/components/layout/FloatingNavIsland";
import type { ReadinessStat } from "@/components/layout/FolioBar";

export interface MagazineShellConfig {
  /** Publication label for FolioBar, e.g., "Account" */
  folioLabel: string;
  /** Atmosphere gradient color */
  atmosphereColor: "turmeric" | "terracotta" | "larkspur" | "olive" | "eucalyptus";
  /** Which nav icon is highlighted */
  activePage: "today" | "week" | "emails" | "dropbox" | "actions" | "me" | "people" | "accounts" | "projects" | "settings";
  /** Back link for FolioBar detail pages */
  backLink?: { label: string; onClick: () => void };
  /** Chapter definitions for FloatingNavIsland scroll mode */
  chapters?: ChapterItem[];
  /** Actions slot for FolioBar */
  folioActions?: React.ReactNode;
  /** Date text for FolioBar center, e.g., "SUNDAY, FEBRUARY 15, 2026" */
  folioDateText?: string;
  /** Readiness stats for FolioBar right section */
  folioReadinessStats?: ReadinessStat[];
  /** Status text for FolioBar right section, e.g., ">_ ready" */
  folioStatusText?: string;
}

/** I563: Volatile folio state that changes frequently (enrichment progress, save status).
 * Delivered via ref so updates don't re-trigger shell registration. */
export interface FolioVolatileState {
  folioActions?: React.ReactNode;
  folioStatusText?: string;
  folioReadinessStats?: ReadinessStat[];
}

interface MagazineShellContextValue {
  config: MagazineShellConfig | null;
  register: (config: MagazineShellConfig) => void;
  unregister: () => void;
  /** I563: Ref for volatile folio state — reads don't trigger re-renders. */
  volatileRef: React.MutableRefObject<FolioVolatileState>;
  /** I563: Bump counter to request a folio repaint without re-registering config. */
  requestFolioRepaint: () => void;
  folioPaintCount: number;
}

const defaultVolatileRef = { current: {} as FolioVolatileState };
const MagazineShellContext = createContext<MagazineShellContextValue>({
  config: null,
  register: () => {},
  unregister: () => {},
  volatileRef: defaultVolatileRef,
  requestFolioRepaint: () => {},
  folioPaintCount: 0,
});

export function useMagazineShellProvider() {
  const [config, setConfig] = useState<MagazineShellConfig | null>(null);
  const volatileRef = useRef<FolioVolatileState>({});
  const [folioPaintCount, setFolioPaintCount] = useState(0);

  const register = useCallback((c: MagazineShellConfig) => setConfig(c), []);
  // I563: unregister clears config only. Volatile ref is NOT wiped here because
  // during same-route navigation (Account A → B), React's effect cleanup ordering
  // means the old page's cleanup runs AFTER the new render's synchronous ref write.
  // Wiping volatile in cleanup would destroy the new page's already-written actions.
  // The volatile ref is overwritten synchronously by useUpdateFolioVolatile on each
  // render, so stale data is never a concern — the new page always wins.
  const unregister = useCallback(() => { setConfig(null); }, []);
  const requestFolioRepaint = useCallback(() => setFolioPaintCount(n => n + 1), []);

  return { config, register, unregister, volatileRef, requestFolioRepaint, folioPaintCount };
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
 * I563: Hook for pages to update volatile folio state (actions, status text)
 * without triggering shell re-registration. Updates are ref-based for performance.
 *
 * The ref write is immediate (every render), but MagazinePageLayout won't
 * re-read it unless requestFolioRepaint() bumps the paint counter.
 *
 * @param state — volatile folio state (actions, status text, readiness stats)
 * @param repaintKey — optional primitive key that triggers a repaint when it
 *   changes. Use the entity ID (accountId, personId, etc.) so that navigating
 *   between entities of the same type forces MagazinePageLayout to pick up the
 *   new volatile actions. Also repaint when folioStatusText changes (save/enrich
 *   status transitions).
 */
export function useUpdateFolioVolatile(
  state: FolioVolatileState,
  repaintKey?: string | null,
) {
  const ctx = useContext(MagazineShellContext);
  ctx.volatileRef.current = state;

  // Repaint when entity identity changes (e.g., account-to-account navigation)
  useLayoutEffect(() => {
    ctx.requestFolioRepaint();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [repaintKey]);

  // Repaint when status text changes (save/enrich transitions)
  useLayoutEffect(() => {
    ctx.requestFolioRepaint();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state.folioStatusText]);
}

/**
 * Hook for MagazinePageLayout to read the current shell configuration.
 */
export function useMagazineShellConfig(): MagazineShellConfig | null {
  return useContext(MagazineShellContext).config;
}

/**
 * I563: Hook for MagazinePageLayout to read volatile folio state.
 */
export function useFolioVolatile(): FolioVolatileState {
  const ctx = useContext(MagazineShellContext);
  // Reading folioPaintCount subscribes the consumer to repaints
  void ctx.folioPaintCount;
  return ctx.volatileRef.current;
}
