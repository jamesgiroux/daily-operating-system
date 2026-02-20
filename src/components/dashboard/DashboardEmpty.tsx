/**
 * DashboardEmpty — Editorial empty state for the daily briefing.
 * Renders inside MagazinePageLayout's page container.
 *
 * Uses the editorial margin grid (100px label column + content) to feel
 * like a page ready to receive its edition, not a blank screen. Section
 * rules, hero-scale typography, and a pull quote treatment for the footnote.
 *
 * When a briefing workflow is running, transitions to the shared
 * GeneratingProgress screen with phase steps and rotating quotes.
 */

import { Mail } from "lucide-react";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";
import { GeneratingProgress } from "@/components/editorial/GeneratingProgress";
import type { GoogleAuthStatus } from "@/types";
import type { WorkflowStatus } from "@/hooks/useWorkflow";

const BRIEFING_PHASES = [
  { key: "preparing", label: "Gathering your day", detail: "Pulling calendar, emails, and entity context" },
  { key: "enriching", label: "Building context", detail: "Assembling meeting prep, priorities, and action items" },
  { key: "delivering", label: "Composing the briefing", detail: "Writing your morning document" },
];

const BRIEFING_QUOTES = [
  "Grab a coffee — your day will be ready soon.",
  "Combobulating your priorities…",
  `"The secret of getting ahead is getting started." — Mark Twain`,
  "Teaching the system about your calendar…",
  `"By failing to prepare, you are preparing to fail." — Benjamin Franklin`,
  "Cross-referencing all the things…",
  "Turning chaos into calendar clarity…",
  `"Plans are nothing; planning is everything." — Dwight D. Eisenhower`,
  "Consulting the schedule oracle…",
  "Almost done thinking about thinking…",
  `"Preparation is the key to success." — Alexander Graham Bell`,
  "Crunching context like it owes us money…",
];

interface DashboardEmptyProps {
  message: string;
  onGenerate?: () => void;
  isRunning?: boolean;
  workflowStatus?: WorkflowStatus;
  googleAuth?: GoogleAuthStatus;
}

export function DashboardEmpty({ message, onGenerate, isRunning, workflowStatus, googleAuth }: DashboardEmptyProps) {
  const { connect, loading: authLoading } = useGoogleAuth();
  const isUnauthed = googleAuth?.status === "notconfigured";

  if (isRunning && workflowStatus?.status === "running") {
    return (
      <GeneratingProgress
        title="Preparing Daily Briefing"
        accentColor="var(--color-spice-turmeric)"
        phases={BRIEFING_PHASES}
        currentPhaseKey={workflowStatus.phase}
        quotes={BRIEFING_QUOTES}
      />
    );
  }

  return (
    <div style={{ paddingTop: 72, paddingBottom: 96 }}>

      {/* ── Margin grid ─────────────────────────────────────────────────── */}
      <div style={{ display: "grid", gridTemplateColumns: "100px 32px 1fr" }}>

        {/* Left label column */}
        <div style={{ paddingTop: 6 }}>
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              fontWeight: 600,
              letterSpacing: "0.1em",
              textTransform: "uppercase" as const,
              color: "var(--color-spice-turmeric)",
            }}
          >
            Today
          </span>
        </div>

        <div />

        {/* Content column */}
        <div>

          {/* Section rule */}
          <div style={{ borderTop: "1px solid var(--color-rule-heavy)", marginBottom: 36 }} />

          {/* Hero headline */}
          <h1
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 52,
              fontWeight: 400,
              letterSpacing: "-0.025em",
              lineHeight: 1.06,
              color: "var(--color-text-primary)",
              margin: "0 0 20px",
              maxWidth: 580,
            }}
          >
            {isUnauthed ? "Connect your Google account." : "No briefing yet."}
          </h1>

          {/* Narrative subtext */}
          <p
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 19,
              fontStyle: "italic",
              fontWeight: 300,
              color: "var(--color-text-secondary)",
              lineHeight: 1.55,
              margin: "0 0 48px",
              maxWidth: 500,
            }}
          >
            {isUnauthed
              ? "Link your calendar and email to receive your daily briefing."
              : message}
          </p>

          {/* Light section rule before action */}
          <div style={{ borderTop: "1px solid var(--color-rule-light)", marginBottom: 28 }} />

          {/* Primary action */}
          {onGenerate && (
            <button
              onClick={onGenerate}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 13,
                fontWeight: 500,
                letterSpacing: "0.04em",
                padding: "10px 28px",
                borderRadius: 4,
                border: "none",
                background: "var(--color-desk-charcoal)",
                color: "var(--color-paper-cream)",
                cursor: "pointer",
                transition: "opacity 0.15s ease",
              }}
              onMouseEnter={(e) => (e.currentTarget.style.opacity = "0.85")}
              onMouseLeave={(e) => (e.currentTarget.style.opacity = "1")}
            >
              Prepare my day
            </button>
          )}

          {/* Google connect — section rule row, not a card */}
          {isUnauthed && (
            <div
              style={{
                borderTop: "1px solid var(--color-rule-light)",
                marginTop: onGenerate ? 32 : 0,
                paddingTop: 20,
                paddingBottom: 4,
                display: "flex",
                alignItems: "center",
                gap: 16,
              }}
            >
              <Mail size={16} style={{ color: "var(--color-text-tertiary)", flexShrink: 0 }} />
              <div style={{ flex: 1 }}>
                <span
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 14,
                    fontWeight: 500,
                    color: "var(--color-text-primary)",
                  }}
                >
                  Connect Google
                </span>
                <span
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 13,
                    color: "var(--color-text-tertiary)",
                    marginLeft: 12,
                  }}
                >
                  Calendar and email for a complete briefing
                </span>
              </div>
              <button
                onClick={connect}
                disabled={authLoading}
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  fontWeight: 500,
                  letterSpacing: "0.06em",
                  textTransform: "uppercase" as const,
                  padding: "6px 14px",
                  borderRadius: 4,
                  border: "1px solid var(--color-rule-heavy)",
                  background: "none",
                  color: "var(--color-text-primary)",
                  cursor: authLoading ? "wait" : "pointer",
                  opacity: authLoading ? 0.5 : 1,
                }}
              >
                Connect
              </button>
            </div>
          )}
        </div>
      </div>

      {/* ── Pull quote ──────────────────────────────────────────────────── */}
      {/* Aligns to the content column, sits below the grid */}
      <div
        style={{
          marginTop: 80,
          marginLeft: 132, /* 100px label + 32px gap */
          maxWidth: 360,
        }}
      >
        <div style={{ borderTop: "1px solid var(--color-rule-light)", marginBottom: 20 }} />
        <p
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 15,
            fontStyle: "italic",
            fontWeight: 300,
            color: "var(--color-text-tertiary)",
            lineHeight: 1.65,
            margin: 0,
          }}
        >
          Grab a coffee — your day will be ready soon.
        </p>
        <div style={{ borderTop: "1px solid var(--color-rule-light)", marginTop: 20 }} />
      </div>

    </div>
  );
}
