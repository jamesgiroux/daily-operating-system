/**
 * OnboardingFlow.tsx — First-run wizard (I57 refactor, I561 three connectors)
 *
 * Step sequence:
 * Welcome → Google → Claude Code → Glean → YouCard → FirstAccount → Role → Prime
 *
 * Google moves first (provides email identity for Glean discovery).
 * Claude Code is skippable (recommended, not blocking).
 * Glean is new and optional — enables account discovery + profile pre-fill.
 * Each step persists immediately via Tauri commands.
 */

import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { homeDir, join } from "@tauri-apps/api/path";
import type { EntityMode, FeatureFlags, DiscoveredAccount, GleanAuthStatus } from "@/types";

import { AtmosphereLayer } from "@/components/layout/AtmosphereLayer";
import { FolioBar } from "@/components/layout/FolioBar";
import { FloatingNavIsland, type ChapterItem } from "@/components/layout/FloatingNavIsland";
import {
  Sparkles,
  Mail,
  Terminal,
  Globe,
  User,
  Briefcase,
  Building,
} from "lucide-react";

import { Welcome } from "./chapters/Welcome";
import { GoogleConnect } from "./chapters/GoogleConnect";
import { ClaudeCode } from "./chapters/ClaudeCode";
import { GleanConnect } from "./chapters/GleanConnect";
import { YouCardStep, type YouCardFormData } from "./chapters/YouCardStep";
import { FirstAccountStep } from "./chapters/FirstAccountStep";
import { EntityMode as EntityModeChapter } from "./chapters/EntityMode";
import { PrimeBriefing } from "./chapters/PrimeBriefing";
import styles from "./onboarding.module.css";

interface OnboardingFlowProps {
  onComplete: () => void;
}

const CHAPTERS = [
  "welcome",
  "google",
  "claude-code",
  "glean",
  "youcard",
  "first-account",
  "role",
  "prime",
] as const;

type Chapter = (typeof CHAPTERS)[number];

const CHAPTER_ICONS: Record<Chapter, React.ReactNode> = {
  "welcome": <Sparkles size={16} strokeWidth={1.8} />,
  "google": <Mail size={16} strokeWidth={1.8} />,
  "claude-code": <Terminal size={16} strokeWidth={1.8} />,
  "glean": <Globe size={16} strokeWidth={1.8} />,
  "youcard": <User size={16} strokeWidth={1.8} />,
  "first-account": <Building size={16} strokeWidth={1.8} />,
  "role": <Briefcase size={16} strokeWidth={1.8} />,
  "prime": <Sparkles size={16} strokeWidth={1.8} />,
};

const CHAPTER_LABELS: Record<Chapter, string> = {
  "welcome": "Welcome",
  "google": "Google",
  "claude-code": "Claude",
  "glean": "Glean",
  "youcard": "About You",
  "first-account": "Account",
  "role": "Your Role",
  "prime": "Prime",
};

const DEFAULT_WORKSPACE = "~/Documents/DailyOS";

/** Map wizard_last_step to the NEXT chapter to show */
function resolveResumeChapter(lastStep: string | null | undefined): Chapter {
  if (!lastStep) return CHAPTERS[0];
  const idx = CHAPTERS.indexOf(lastStep as Chapter);
  if (idx === -1) return CHAPTERS[0];
  // Advance to the step after the last completed one
  const next = idx + 1;
  return next < CHAPTERS.length ? CHAPTERS[next] : CHAPTERS[CHAPTERS.length - 1];
}

export function OnboardingFlow({ onComplete }: OnboardingFlowProps) {
  const [chapter, setChapter] = useState<Chapter>(CHAPTERS[0]);
  const [visitedChapters, setVisitedChapters] = useState<Set<Chapter>>(new Set([CHAPTERS[0]]));
  const [resumeChecked, setResumeChecked] = useState(false);
  const [rolePresetsEnabled, setRolePresetsEnabled] = useState(false);

  // I561: Glean state
  const [gleanConnected, setGleanConnected] = useState(false);
  const [discoveredAccounts, setDiscoveredAccounts] = useState<DiscoveredAccount[]>([]);
  const [discoveryLoading, setDiscoveryLoading] = useState(false);

  // Fetch feature flags on mount
  useEffect(() => {
    invoke<FeatureFlags>("get_feature_flags")
      .then((flags) => setRolePresetsEnabled(flags.role_presets_enabled))
      .catch(() => setRolePresetsEnabled(false));
  }, []);

  // Resume from last completed step on mount
  useEffect(() => {
    if (resumeChecked) return;

    const doResume = async () => {
      try {
        const state = await invoke<{ wizardLastStep?: string | null }>("get_app_state");
        const resumeTo = resolveResumeChapter(state.wizardLastStep);

        // Re-check Glean auth on resume to restore gleanConnected state
        try {
          const gleanStatus = await invoke<GleanAuthStatus>("get_glean_auth_status");
          if (gleanStatus.status === "authenticated") {
            setGleanConnected(true);
          }
        } catch {
          // Non-fatal
        }

        if (resumeTo !== "welcome") {
          const visited = new Set<Chapter>();
          for (const c of CHAPTERS) {
            visited.add(c);
            if (c === resumeTo) break;
          }
          setChapter(resumeTo);
          setVisitedChapters(visited);
        }
      } catch {
        // Non-fatal — just start from beginning
      } finally {
        setResumeChecked(true);
      }
    };

    doResume();
  }, [resumeChecked]);

  // Lifted form state
  const [youCardData, setYouCardData] = useState<YouCardFormData>({
    name: "",
    company: "",
    title: "",
    domains: [],
  });

  function goToChapter(c: Chapter) {
    setChapter(c);
    setVisitedChapters((prev) => new Set([...prev, c]));
  }

  // Auto-create workspace at default path (dev-aware)
  const autoCreateWorkspace = useCallback(async () => {
    try {
      // Check if workspace is already set
      const existing = await invoke<{ workspacePath?: string }>("get_config")
        .then((c) => c.workspacePath)
        .catch(() => null);

      if (!existing) {
        const home = await homeDir();
        // Use DailyOS-dev when dev sandbox is active, DailyOS otherwise
        const isDevDb = import.meta.env.DEV
          ? await invoke<{ isDevDbMode?: boolean }>("dev_get_state")
              .then((s) => s.isDevDbMode === true)
              .catch(() => false)
          : false;
        const dirName = isDevDb ? "DailyOS-dev" : "DailyOS";
        const absPath = await join(home, "Documents", dirName);
        await invoke("set_workspace_path", { path: absPath });
      }
    } catch (e) {
      console.error("Auto-create workspace failed:", e);
    }
  }, []);

  // "Skip setup" — auto-create workspace, land on empty dashboard
  async function handleSkipSetup() {
    try {
      await autoCreateWorkspace();
      // Set lock timeout to "Never" for new installs
      await invoke("set_lock_timeout", { minutes: null }).catch(() => {});
    } catch {
      // Non-fatal
    }
    onComplete();
  }

  // Handle demo mode entry from Welcome
  async function handleDemoMode() {
    try {
      await autoCreateWorkspace();
      await invoke("install_demo_data");
    } catch (e) {
      console.error("Demo install failed:", e);
    }
    onComplete();
  }

  // Fire Glean account discovery in background
  async function triggerGleanDiscovery() {
    setDiscoveryLoading(true);
    try {
      const accounts = await invoke<DiscoveredAccount[]>("discover_accounts_from_glean");
      setDiscoveredAccounts(accounts);
    } catch (e) {
      console.error("Glean discovery failed:", e);
    } finally {
      setDiscoveryLoading(false);
    }
  }

  // Complete wizard — mark done, trigger calendar poll if connected
  async function handleWizardComplete(_mode: EntityMode) {
    try {
      // Ensure workspace exists — required for the post-reload config check
      await autoCreateWorkspace();
      await invoke("set_wizard_step", { step: "prime" }).catch(() => {});
      await invoke("set_wizard_completed");
      // Trigger immediate calendar poll if Google is connected
      try {
        const authStatus = await invoke<{ status: string }>("get_google_auth_status");
        if (authStatus.status === "authenticated") {
          // Non-blocking calendar poll
          invoke("run_workflow", { workflowId: "today" }).catch(() => {});
        }
      } catch {
        // Non-fatal
      }
    } catch (e) {
      console.error("Wizard completion failed:", e);
    }
    onComplete();
  }

  // Build chapter items for FloatingNavIsland (step dots, not labels)
  // Hide "role" chapter when role presets are gated off (I537)
  const navChapters: ChapterItem[] = CHAPTERS
    .filter((c) => c !== "welcome" && (c !== "role" || rolePresetsEnabled))
    .map((c) => ({
      id: c,
      label: CHAPTER_LABELS[c],
      icon: CHAPTER_ICONS[c],
    }));

  return (
    <div className={styles.wrapper}>
      <AtmosphereLayer color="turmeric" />
      <FolioBar publicationLabel="Setup" />

      {/* FloatingNavIsland — show step dots (skip welcome) */}
      {chapter !== "welcome" && (
        <FloatingNavIsland
          mode="chapters"
          chapters={navChapters}
          activeChapterId={chapter}
          activeColor="turmeric"
          onChapterClick={(id) => {
            if (visitedChapters.has(id as Chapter)) {
              goToChapter(id as Chapter);
            }
          }}
        />
      )}

      {/* Content column */}
      <div className={styles.contentColumn}>
        {/* Step content */}
        {chapter === "welcome" && (
          <Welcome
            onNext={() => goToChapter("google")}
            onDemoMode={handleDemoMode}
            onSkipSetup={handleSkipSetup}
          />
        )}

        {chapter === "google" && (
          <GoogleConnect
            onNext={async () => {
              await invoke("set_wizard_step", { step: "google" }).catch(() => {});
              goToChapter("claude-code");
            }}
          />
        )}

        {chapter === "claude-code" && (
          <ClaudeCode
            workspacePath={DEFAULT_WORKSPACE}
            onNext={async (_installed) => {
              // Silently auto-create workspace on Claude Code success
              await autoCreateWorkspace();
              // Set lock timeout to "Never" for new installs
              await invoke("set_lock_timeout", { minutes: null }).catch(() => {});
              // Check iCloud warning inline — returns warning message or null
              try {
                const icloudMsg = await invoke<string | null>("check_icloud_warning");
                if (icloudMsg) {
                  console.warn("Workspace may be iCloud-synced:", icloudMsg);
                }
              } catch {
                // Non-fatal
              }
              await invoke("set_wizard_step", { step: "claude-code" }).catch(() => {});
              goToChapter("glean");
            }}
            onSkip={async () => {
              await autoCreateWorkspace();
              await invoke("set_lock_timeout", { minutes: null }).catch(() => {});
              await invoke("set_wizard_step", { step: "claude-code" }).catch(() => {});
              goToChapter("glean");
            }}
          />
        )}

        {chapter === "glean" && (
          <GleanConnect
            onNext={async (connected) => {
              setGleanConnected(connected);
              await invoke("set_wizard_step", { step: "glean" }).catch(() => {});
              if (connected) {
                // Fire discovery in background — results appear on FirstAccountStep
                triggerGleanDiscovery();
              }
              goToChapter("youcard");
            }}
            onSkip={async () => {
              await invoke("set_wizard_step", { step: "glean" }).catch(() => {});
              goToChapter("youcard");
            }}
          />
        )}

        {chapter === "youcard" && (
          <YouCardStep
            formData={youCardData}
            onFormChange={setYouCardData}
            onNext={() => goToChapter("first-account")}
            onSkip={() => goToChapter("first-account")}
            gleanConnected={gleanConnected}
          />
        )}

        {chapter === "first-account" && (
          <FirstAccountStep
            gleanConnected={gleanConnected}
            discoveredAccounts={discoveredAccounts}
            discoveryLoading={discoveryLoading}
            onNext={async () => {
              if (rolePresetsEnabled) {
                goToChapter("role");
              } else {
                // Auto-set CS preset and "both" entity mode as defaults
                await invoke("set_role", { role: "customer-success" }).catch(() => {});
                await invoke("set_entity_mode", { mode: "both" }).catch(() => {});
                await invoke("set_wizard_step", { step: "role" }).catch(() => {});
                goToChapter("prime");
              }
            }}
            onSkip={async () => {
              if (rolePresetsEnabled) {
                goToChapter("role");
              } else {
                await invoke("set_role", { role: "customer-success" }).catch(() => {});
                await invoke("set_entity_mode", { mode: "both" }).catch(() => {});
                await invoke("set_wizard_step", { step: "role" }).catch(() => {});
                goToChapter("prime");
              }
            }}
          />
        )}

        {chapter === "role" && rolePresetsEnabled && (
          <EntityModeChapter
            onNext={async (_mode) => {
              await invoke("set_wizard_step", { step: "role" }).catch(() => {});
              goToChapter("prime");
            }}
          />
        )}

        {chapter === "prime" && (
          <PrimeBriefing
            onComplete={() => handleWizardComplete("both")}
          />
        )}
      </div>
    </div>
  );
}
