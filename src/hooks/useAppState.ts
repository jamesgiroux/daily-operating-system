/**
 * useAppState.ts
 *
 * App-level state hook for demo mode, tour, and wizard progress.
 * Provides context via AppStateProvider, consumed by any component
 * that needs to know about demo mode or onboarding state.
 */

import { createContext, useContext, useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface AppState {
  demoModeActive: boolean;
  hasCompletedTour: boolean;
  wizardCompletedAt: string | null;
  wizardLastStep: string | null;
}

interface AppStateContext {
  appState: AppState;
  loading: boolean;
  /** True when user clicked "Resume setup" — forces onboarding re-entry */
  forceOnboarding: boolean;
  refresh: () => Promise<void>;
  installDemo: () => Promise<void>;
  clearDemo: () => Promise<void>;
  completeTour: () => Promise<void>;
  /** Trigger onboarding re-entry from Settings */
  resumeOnboarding: () => void;
  /** Dismiss the setup banner — marks wizard as completed */
  dismissSetupBanner: () => Promise<void>;
}

const defaultAppState: AppState = {
  demoModeActive: false,
  hasCompletedTour: false,
  wizardCompletedAt: null,
  wizardLastStep: null,
};

const AppStateCtx = createContext<AppStateContext>({
  appState: defaultAppState,
  loading: true,
  forceOnboarding: false,
  refresh: async () => {},
  installDemo: async () => {},
  clearDemo: async () => {},
  completeTour: async () => {},
  resumeOnboarding: () => {},
  dismissSetupBanner: async () => {},
});

export function useAppState() {
  return useContext(AppStateCtx);
}

export { AppStateCtx };

export function useAppStateProvider(): AppStateContext {
  const [appState, setAppState] = useState<AppState>(defaultAppState);
  const [loading, setLoading] = useState(true);
  const [forceOnboarding, setForceOnboarding] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const state = await invoke<AppState>("get_app_state");
      setAppState(state);
    } catch (err) {
      console.error("get_app_state failed:", err); // Expected: background init on mount
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const installDemo = useCallback(async () => {
    try {
      await invoke("install_demo_data");
      await refresh();
    } catch (err) {
      console.error("install_demo_data failed:", err); // Expected: demo install can fail gracefully
    }
  }, [refresh]);

  const clearDemo = useCallback(async () => {
    try {
      await invoke("clear_demo_data");
      await refresh();
      // After clearing demo data, re-enter the wizard so the user can connect real data
      setForceOnboarding(true);
    } catch (err) {
      console.error("clear_demo_data failed:", err); // Expected: demo clear can fail gracefully
    }
  }, [refresh]);

  const completeTour = useCallback(async () => {
    try {
      await invoke("set_tour_completed");
      setAppState((prev) => ({ ...prev, hasCompletedTour: true }));
    } catch (err) {
      console.error("set_tour_completed failed:", err); // Expected: tour state best-effort
    }
  }, []);

  const resumeOnboarding = useCallback(() => {
    setForceOnboarding(true);
  }, []);

  const dismissSetupBanner = useCallback(async () => {
    try {
      await invoke("set_wizard_completed");
      setAppState((prev) => ({ ...prev, wizardCompletedAt: new Date().toISOString() }));
    } catch (err) {
      console.error("set_wizard_completed failed:", err); // Expected: wizard state best-effort
    }
  }, []);

  return {
    appState,
    loading,
    forceOnboarding,
    refresh,
    installDemo,
    clearDemo,
    completeTour,
    resumeOnboarding,
    dismissSetupBanner,
  };
}
