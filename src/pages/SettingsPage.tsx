import { useMemo, useEffect, useRef } from "react";
import { useSearch } from "@tanstack/react-router";
import { User, Link2, Monitor, Shield, Wrench } from "lucide-react";
import { useAppState } from "@/hooks/useAppState";

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
  const { appState, resumeOnboarding } = useAppState();

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
