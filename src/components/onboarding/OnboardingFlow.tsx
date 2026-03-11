/**
 * OnboardingFlow.tsx — First-run wizard (I57 refactor)
 *
 * Trimmed from 11 chapters to 4 essential steps:
 * Landing → Claude Code → Google → YouCard → Role Preset → Dashboard
 *
 * The Claude Code step is required (no skip). All others are skippable.
 * Each step persists immediately via Tauri commands.
 */

import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { homeDir, join } from "@tauri-apps/api/path";
import type { EntityMode, FeatureFlags } from "@/types";

import { AtmosphereLayer } from "@/components/layout/AtmosphereLayer";
import { FolioBar } from "@/components/layout/FolioBar";
import { FloatingNavIsland, type ChapterItem } from "@/components/layout/FloatingNavIsland";
import {
  Sparkles,
  Mail,
  Terminal,
  User,
  Briefcase,
  Building,
} from "lucide-react";

import { Welcome } from "./chapters/Welcome";
import { GoogleConnect } from "./chapters/GoogleConnect";
import { ClaudeCode } from "./chapters/ClaudeCode";
import { YouCardStep, type YouCardFormData } from "./chapters/YouCardStep";
import { FirstAccountStep } from "./chapters/FirstAccountStep";
import { EntityMode as EntityModeChapter } from "./chapters/EntityMode";
import { PrimeBriefing } from "./chapters/PrimeBriefing";

interface OnboardingFlowProps {
  onComplete: () => void;
}

const CHAPTERS = [
  "welcome",
  "claude-code",
  "google",
  "youcard",
  "first-account",
  "role",
  "prime",
] as const;

type Chapter = (typeof CHAPTERS)[number];

const CHAPTER_ICONS: Record<Chapter, React.ReactNode> = {
  "welcome": <Sparkles size={16} strokeWidth={1.8} />,
  "claude-code": <Terminal size={16} strokeWidth={1.8} />,
  "google": <Mail size={16} strokeWidth={1.8} />,
  "youcard": <User size={16} strokeWidth={1.8} />,
  "first-account": <Building size={16} strokeWidth={1.8} />,
  "role": <Briefcase size={16} strokeWidth={1.8} />,
  "prime": <Sparkles size={16} strokeWidth={1.8} />,
};

const CHAPTER_LABELS: Record<Chapter, string> = {
  "welcome": "Welcome",
  "claude-code": "Claude",
  "google": "Google",
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

  // Fetch feature flags on mount
  useEffect(() => {
    invoke<FeatureFlags>("get_feature_flags")
      .then((flags) => setRolePresetsEnabled(flags.role_presets_enabled))
      .catch(() => setRolePresetsEnabled(false));
  }, []);

  // Resume from last completed step on mount
  useEffect(() => {
    if (resumeChecked) return;
    invoke<{ wizardLastStep?: string | null }>("get_app_state")
      .then((state) => {
        const resumeTo = resolveResumeChapter(state.wizardLastStep);
        if (resumeTo !== "welcome") {
          // Mark all prior chapters as visited
          const visited = new Set<Chapter>();
          for (const c of CHAPTERS) {
            visited.add(c);
            if (c === resumeTo) break;
          }
          setChapter(resumeTo);
          setVisitedChapters(visited);
        }
      })
      .catch(() => {}) // Non-fatal — just start from beginning
      .finally(() => setResumeChecked(true));
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

  // Complete wizard — mark done, trigger calendar poll if connected
  async function handleWizardComplete(_mode: EntityMode) {
    try {
      // Ensure workspace exists — required for the post-reload config check
      await autoCreateWorkspace();
      await invoke("set_wizard_step", { step: "role" }).catch(() => {});
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
    <div
      style={{
        minHeight: "100vh",
        background: "var(--color-paper-cream)",
        position: "relative",
      }}
    >
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
      <div
        style={{
          maxWidth: 1080,
          margin: "0 auto",
          paddingTop: 80,
          paddingBottom: 120,
          paddingLeft: "var(--page-padding-horizontal)",
          paddingRight: "var(--page-padding-horizontal)",
          position: "relative",
          zIndex: "var(--z-page-content)",
        }}
      >
        {/* Step content */}
        {chapter === "welcome" && (
          <Welcome
            onNext={() => goToChapter("claude-code")}
            onDemoMode={handleDemoMode}
            onSkipSetup={handleSkipSetup}
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
              goToChapter("google");
            }}
          />
        )}

        {chapter === "google" && (
          <GoogleConnect
            onNext={async () => {
              await invoke("set_wizard_step", { step: "google" }).catch(() => {});
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
          />
        )}

        {chapter === "first-account" && (
          <FirstAccountStep
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
