import { useMemo, useEffect, useRef } from "react";
import { useSearch } from "@tanstack/react-router";
import { User, Link2, Monitor, Shield, Wrench, Loader2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";
import { useAppState } from "@/hooks/useAppState";
import { useClaudeStatus } from "@/hooks/useClaudeStatus";

import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import YouCard from "@/components/settings/YouCard";
import ConnectorsGrid from "@/components/settings/ConnectorsGrid";
import ContextSourceSection from "@/components/settings/ContextSourceSection";
import SystemStatus from "@/components/settings/SystemStatus";
import ActivityLogSection from "@/components/settings/ActivityLogSection";
import DiagnosticsSection from "@/components/settings/DiagnosticsSection";

// ═══════════════════════════════════════════════════════════════════════════
// ClaudeCodeSection
// ═══════════════════════════════════════════════════════════════════════════

const ctaButtonStyle = (disabled: boolean): React.CSSProperties => ({
  alignSelf: "flex-start",
  fontFamily: "var(--font-mono)",
  fontSize: 11,
  letterSpacing: "0.04em",
  color: disabled ? "var(--color-text-tertiary)" : "var(--color-spice-turmeric)",
  background: "none",
  border: "none",
  cursor: disabled ? "default" : "pointer",
  padding: 0,
  display: "inline-flex",
  alignItems: "center",
  gap: 6,
});

function StatusDot({ color }: { color: string }) {
  return (
    <div
      style={{
        width: 8,
        height: 8,
        borderRadius: "50%",
        background: color,
        flexShrink: 0,
      }}
    />
  );
}

function ClaudeCodeSection() {
  const { status, aiUnavailable, checking, refresh } = useClaudeStatus();
  const ready = status !== null && !aiUnavailable;

  async function handleSignIn() {
    await invoke("launch_claude_login");
  }

  async function handleDownload() {
    await open("https://claude.ai/download");
  }

  return (
    <div>
      <p
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 600,
          letterSpacing: "0.06em",
          textTransform: "uppercase",
          color: "var(--color-text-tertiary)",
          margin: 0,
          marginBottom: 12,
        }}
      >
        Claude Code
      </p>

      {/* Loading */}
      {checking && !status && (
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <Loader2 size={14} className="animate-spin" style={{ color: "var(--color-text-tertiary)" }} />
          <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-tertiary)" }}>
            Checking...
          </span>
        </div>
      )}

      {/* Ready */}
      {status && ready && (
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <StatusDot color="var(--color-garden-sage)" />
          <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
            Claude Code is ready
          </span>
        </div>
      )}

      {/* Installed, not authenticated */}
      {status && status.installed && !status.authenticated && (
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <StatusDot color="var(--color-spice-turmeric)" />
            <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-secondary)" }}>
              Claude Code needs to be signed in
            </span>
          </div>
          <div style={{ display: "flex", gap: 12 }}>
            <button onClick={handleSignIn} style={ctaButtonStyle(false)}>
              Sign in to Claude &rarr;
            </button>
            <button onClick={refresh} disabled={checking} style={ctaButtonStyle(checking)}>
              {checking && <Loader2 size={11} className="animate-spin" />}
              Check again
            </button>
          </div>
        </div>
      )}

      {/* Not installed */}
      {status && !status.installed && (
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <StatusDot color="var(--color-spice-terracotta)" />
            <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-secondary)" }}>
              Claude Code isn't installed
            </span>
          </div>
          <div style={{ display: "flex", gap: 12 }}>
            <button onClick={handleDownload} style={ctaButtonStyle(false)}>
              Download Claude Code &rarr;
            </button>
            <button onClick={refresh} disabled={checking} style={ctaButtonStyle(checking)}>
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
  const { appState, resumeOnboarding } = useAppState();
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
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
      {/* Setup incomplete banner (I57) */}
      {!appState.wizardCompletedAt && (
        <div
          style={{
            padding: "12px 20px",
            borderBottom: "1px solid var(--color-rule-light)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: 16,
          }}
        >
          <span
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              color: "var(--color-text-secondary)",
            }}
          >
            Finish setting up DailyOS — briefings work best when the system knows about you.
          </span>
          <button
            onClick={resumeOnboarding}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              letterSpacing: "0.04em",
              color: "var(--color-spice-turmeric)",
              background: "none",
              border: "none",
              cursor: "pointer",
              whiteSpace: "nowrap",
              padding: 0,
            }}
          >
            Resume setup &rarr;
          </button>
        </div>
      )}

      {/* Claude Code not ready banner — shown only after wizard is complete */}
      {appState.wizardCompletedAt && claudeStatus !== null && aiUnavailable && (
        <div
          style={{
            padding: "12px 20px",
            borderBottom: "1px solid var(--color-rule-light)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: 16,
          }}
        >
          <span
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              color: "var(--color-text-secondary)",
            }}
          >
            Claude Code isn't set up — without it, AI briefings won't be generated.
          </span>
          <button
            onClick={() => claudeCodeRef.current?.scrollIntoView({ behavior: "smooth", block: "start" })}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              letterSpacing: "0.04em",
              color: "var(--color-spice-turmeric)",
              background: "none",
              border: "none",
              cursor: "pointer",
              whiteSpace: "nowrap",
              padding: 0,
            }}
          >
            Set up Claude Code &rarr;
          </button>
        </div>
      )}

      {/* ═══ HERO ═══ */}
      <section style={{ paddingTop: 80, paddingBottom: 40 }}>
        <h1
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 42,
            fontWeight: 400,
            letterSpacing: "-0.02em",
            color: "var(--color-text-primary)",
            margin: 0,
          }}
        >
          Settings
        </h1>
        <div
          style={{
            height: 2,
            background: "var(--color-desk-charcoal)",
            marginTop: 16,
          }}
        />
      </section>

      {/* ═══ YOU ═══ */}
      <section id="settings-you" style={{ marginBottom: 48 }}>
        <ChapterHeading
          title="You"
          epigraph="Who you are and how your workspace is organized."
        />
        <YouCard />
      </section>

      {/* ═══ CONNECTORS ═══ */}
      <section id="settings-connectors" style={{ marginBottom: 48 }}>
        <ChapterHeading
          title="Connectors"
          epigraph="External services that feed your daily briefings."
        />
        <ContextSourceSection />
        <ConnectorsGrid />
      </section>

      {/* ═══ DATA ═══ */}
      <section id="settings-data" style={{ marginBottom: 48 }}>
        <ChapterHeading
          title="Data"
          epigraph="What happened, when, and whether anything looks unusual."
        />
        <ActivityLogSection />
      </section>

      {/* ═══ SYSTEM ═══ */}
      <section id="settings-system" style={{ marginBottom: 48 }}>
        <ChapterHeading
          title="System"
          epigraph="Version, health, and advanced configuration."
        />
        <div ref={claudeCodeRef} style={{ marginBottom: 32 }}>
          <ClaudeCodeSection />
        </div>
        <SystemStatus />
      </section>

      {/* ═══ DIAGNOSTICS (dev only) ═══ */}
      {import.meta.env.DEV && (
        <section id="settings-diagnostics" style={{ marginBottom: 48 }}>
          <ChapterHeading
            title="Diagnostics"
            epigraph="Developer tools and debugging utilities."
          />
          <DiagnosticsSection />
        </section>
      )}

      <FinisMarker />
      <div style={{ height: 80 }} />
    </div>
  );
}
