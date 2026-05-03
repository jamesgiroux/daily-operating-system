import { useMemo, useEffect, useRef, useState } from "react";
import { useSearch } from "@tanstack/react-router";
import { User, Link2, Monitor, Shield, Wrench, Loader2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useAppState } from "@/hooks/useAppState";
import { useClaudeStatus } from "@/hooks/useClaudeStatus";

import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { EditorialPageHeader } from "@/components/editorial/EditorialPageHeader";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import StatusDot from "@/components/shared/StatusDot";
import YouCard from "@/components/settings/YouCard";
import ConnectorsGrid from "@/components/settings/ConnectorsGrid";
import ContextSourceSection from "@/components/settings/ContextSourceSection";
import SystemStatus from "@/components/settings/SystemStatus";
import ActivityLogSection from "@/components/settings/ActivityLogSection";
import DiagnosticsSection from "@/components/settings/DiagnosticsSection";
import DatabaseRecoveryCard from "@/components/settings/DatabaseRecoveryCard";
import DataPrivacySection from "@/components/settings/DataPrivacySection";
import NotificationSection from "@/components/settings/NotificationSection";
import TextSizeSection from "@/components/settings/TextSizeSection";
import s from "./SettingsPage.module.css";

// ═══════════════════════════════════════════════════════════════════════════
// ClaudeCodeSection
// ═══════════════════════════════════════════════════════════════════════════

function ClaudeCodeSection() {
  const { status, aiUnavailable, checking, forceRefresh } = useClaudeStatus();
  const ready = status !== null && !aiUnavailable;
  const [installing, setInstalling] = useState(false);

  async function handleSignIn() {
    await invoke("launch_claude_login");
  }

  async function handleInstall() {
    setInstalling(true);
    try {
      await invoke("install_claude_cli");
      await forceRefresh();
    } catch {
      // Error is shown via the status check
    } finally {
      setInstalling(false);
    }
  }

  return (
    <div>
      <p className={s.claudeCodeLabel}>Claude Code</p>

      {/* Loading */}
      {checking && !status && (
        <div className={s.claudeCodeRow}>
          <Loader2 size={14} className={`animate-spin ${s.iconTertiary}`} />
          <span className={s.claudeCodeTextTertiary}>Checking...</span>
        </div>
      )}

      {/* Ready */}
      {status && ready && (
        <div className={s.claudeCodeRow}>
          <StatusDot status="connected" />
          <span className={s.claudeCodeText}>Claude Code is ready</span>
        </div>
      )}

      {/* Installed, not authenticated */}
      {status && status.installed && !status.authenticated && (
        <div className={s.claudeCodeColumn}>
          <div className={s.claudeCodeRow}>
            <StatusDot status="loading" />
            <span className={s.claudeCodeTextMuted}>
              Claude Code needs to be signed in
            </span>
          </div>
          <div className={s.claudeCodeActions}>
            <button onClick={handleSignIn} className={s.ctaButton}>
              Sign in to Claude &rarr;
            </button>
            <button onClick={forceRefresh} disabled={checking} className={s.ctaButton}>
              {checking && <Loader2 size={11} className="animate-spin" />}
              Check again
            </button>
          </div>
        </div>
      )}

      {/* Not installed — Node available: offer one-click install */}
      {status && !status.installed && status.nodeInstalled && (
        <div className={s.claudeCodeColumn}>
          <div className={s.claudeCodeRow}>
            <StatusDot status="disconnected" />
            <span className={s.claudeCodeTextMuted}>
              Claude Code isn't installed
            </span>
          </div>
          <div className={s.claudeCodeActions}>
            <button onClick={handleInstall} disabled={installing} className={s.ctaButton}>
              {installing && <Loader2 size={11} className="animate-spin" />}
              Install Claude Code &rarr;
            </button>
            <button onClick={forceRefresh} disabled={checking} className={s.ctaButton}>
              {checking && <Loader2 size={11} className="animate-spin" />}
              Check again
            </button>
          </div>
        </div>
      )}

      {/* Not installed — Node not available: show install instructions */}
      {status && !status.installed && !status.nodeInstalled && (
        <div className={s.claudeCodeColumn}>
          <div className={s.claudeCodeRow}>
            <StatusDot status="disconnected" />
            <span className={s.claudeCodeTextMuted}>
              Claude Code requires Node.js
            </span>
          </div>
          <p className={s.claudeCodeTextTertiary}>
            Install Node.js from{" "}
            <a href="https://nodejs.org" target="_blank" rel="noopener noreferrer" className={s.ctaButton}>
              nodejs.org
            </a>
            , then click Check again.
          </p>
          <div className={s.claudeCodeActions}>
            <button onClick={forceRefresh} disabled={checking} className={s.ctaButton}>
              {checking && <Loader2 size={11} className="animate-spin" />}
              Check again
            </button>
          </div>
        </div>
      )}
    </div>
  );
}


// ═══════════════════════════════════════════════════════════════════════════
// Deep-link tab mapping
// ═══════════════════════════════════════════════════════════════════════════

/** Map legacy tab IDs to new section IDs for backwards-compatible deep links */
const LEGACY_TAB_MAP: Record<string, string> = {
  profile: "you",
  role: "you",
  integrations: "connectors",
  workflows: "system",
  intelligence: "system",
  hygiene: "system",
};

const VALID_TABS = new Set<string>([
  "you",
  "connectors",
  "data",
  "system",
  "diagnostics",
  ...Object.keys(LEGACY_TAB_MAP),
]);

function resolveTab(value: unknown): string | null {
  if (typeof value !== "string" || !VALID_TABS.has(value)) return null;
  return LEGACY_TAB_MAP[value] ?? value;
}

// ═══════════════════════════════════════════════════════════════════════════
// Chapter definitions
// ═══════════════════════════════════════════════════════════════════════════

const CHAPTER_DEFS = [
  { id: "settings-you", label: "You", icon: <User size={18} strokeWidth={1.5} /> },
  { id: "settings-connectors", label: "Connectors", icon: <Link2 size={18} strokeWidth={1.5} /> },
  { id: "settings-data", label: "Data", icon: <Shield size={18} strokeWidth={1.5} /> },
  { id: "settings-system", label: "System", icon: <Monitor size={18} strokeWidth={1.5} /> },
];

const DIAGNOSTICS_CHAPTER = {
  id: "settings-diagnostics",
  label: "Diagnostics",
  icon: <Wrench size={18} strokeWidth={1.5} />,
};

// ═══════════════════════════════════════════════════════════════════════════
// SettingsPage
// ═══════════════════════════════════════════════════════════════════════════

export default function SettingsPage() {
  const search = useSearch({ from: "/settings" });
  const scrolledRef = useRef(false);
  const claudeCodeRef = useRef<HTMLDivElement>(null);
  const { appState, resumeOnboarding, dismissSetupBanner } = useAppState();
  const { status: claudeStatus, aiUnavailable } = useClaudeStatus();

  // Chapters: include diagnostics only in dev mode
  const chapters = useMemo(() => {
    if (import.meta.env.DEV) {
      return [...CHAPTER_DEFS, DIAGNOSTICS_CHAPTER];
    }
    return CHAPTER_DEFS;
  }, []);

  // Register magazine shell
  const shellConfig = useMemo(
    () => ({
      folioLabel: "Settings",
      atmosphereColor: "olive" as const,
      activePage: "settings" as const,
      chapters,
    }),
    [chapters],
  );
  useRegisterMagazineShell(shellConfig);

  // Deep-link scroll on mount
  useEffect(() => {
    if (scrolledRef.current) return;
    const tab = resolveTab(search.tab);
    if (tab && tab !== "you") {
      const el = document.getElementById(`settings-${tab}`);
      if (el) {
        el.scrollIntoView({ behavior: "smooth", block: "start" });
        scrolledRef.current = true;
      }
    }
  }, [search.tab]);

  return (
    <div className={s.container}>
      {/* Setup incomplete banner  */}
      {!appState.wizardCompletedAt && (
        <div className={s.banner}>
          <span className={s.bannerText}>
            Finish setting up DailyOS — briefings work best when the system knows about you.
          </span>
          <button onClick={resumeOnboarding} className={s.bannerAction}>
            Resume setup &rarr;
          </button>
          <button onClick={dismissSetupBanner} className={s.bannerDismiss} title="Dismiss">
            &times;
          </button>
        </div>
      )}

      {/* Claude Code not ready banner — shown only after wizard is complete */}
      {appState.wizardCompletedAt && claudeStatus !== null && aiUnavailable && (
        <div className={s.banner}>
          <span className={s.bannerText}>
            Claude Code isn't set up — without it, AI briefings won't be generated.
          </span>
          <button
            onClick={() => claudeCodeRef.current?.scrollIntoView({ behavior: "smooth", block: "start" })}
            className={s.bannerAction}
          >
            Set up Claude Code &rarr;
          </button>
        </div>
      )}

      {/* ═══ HERO ═══ */}
      <EditorialPageHeader title="Settings" scale="page" width="standard" />

      {/* ═══ YOU ═══ */}
      <section id="settings-you" className={s.section}>
        <ChapterHeading
          title="You"
          epigraph="Who you are and how your workspace is organized."
        />
        <YouCard />
      </section>

      {/* ═══ CONNECTORS ═══ */}
      <section id="settings-connectors" className={s.section}>
        <ChapterHeading
          title="Connectors"
          epigraph="External services that feed your daily briefings."
        />
        <ContextSourceSection />
        <ConnectorsGrid />
      </section>

      {/* ═══ DATA ═══ */}
      <section id="settings-data" className={s.section}>
        <ChapterHeading
          title="Data"
          epigraph="What happened, when, and whether anything looks unusual."
        />
        <DatabaseRecoveryCard />
        <ActivityLogSection />
        <div className={s.sectionInset}>
          <DataPrivacySection />
        </div>
      </section>

      {/* ═══ SYSTEM ═══ */}
      <section id="settings-system" className={s.section}>
        <ChapterHeading
          title="System"
          epigraph="Version, health, and advanced configuration."
        />
        <div ref={claudeCodeRef} className={s.claudeCodeWrapper}>
          <ClaudeCodeSection />
        </div>
        <SystemStatus />
        <NotificationSection />
        <TextSizeSection />
        {appState.wizardCompletedAt && (
          <div className={s.systemAction}>
            <button onClick={resumeOnboarding} className={s.systemActionButton}>
              Run Setup Again
            </button>
            <span className={s.systemActionHint}>Re-run the onboarding wizard to update your profile and connectors.</span>
          </div>
        )}
      </section>

      {/* ═══ DIAGNOSTICS (dev only) ═══ */}
      {import.meta.env.DEV && (
        <section id="settings-diagnostics" className={s.section}>
          <ChapterHeading
            title="Diagnostics"
            epigraph="Developer tools and debugging utilities."
          />
          <DiagnosticsSection />
        </section>
      )}

      <FinisMarker />
      <div className={s.bottomSpacer} />
    </div>
  );
}
