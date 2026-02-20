/**
 * DashboardEmpty — Editorial empty state for the daily briefing.
 * Renders inside MagazinePageLayout's page container.
 * Warm language, serif typography, prominent generate action.
 *
 * When a briefing workflow is running, transitions to the shared
 * GeneratingProgress screen with phase steps and rotating quotes.
 */

import { Mail } from "lucide-react";
import { BrandMark } from "@/components/ui/BrandMark";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";
import { GeneratingProgress } from "@/components/editorial/GeneratingProgress";
import type { GoogleAuthStatus } from "@/types";
import type { WorkflowStatus } from "@/hooks/useWorkflow";

const BRIEFING_PHASES = [
  { key: "preparing", label: "Gathering your day", detail: "Pulling calendar, emails, and entity context" },
  { key: "enriching", label: "AI processing", detail: "Building meeting prep, priorities, and action items" },
  { key: "delivering", label: "Assembling the briefing", detail: "Composing your morning document" },
];

const BRIEFING_QUOTES = [
  "Grab a coffee — your day will be ready soon.",
  "Combobulating your priorities…",
  `"The secret of getting ahead is getting started." — Mark Twain`,
  "Teaching the AI about your calendar…",
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

  // Show the full generating progress screen when workflow is running
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
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
      <div style={{ paddingTop: 120, paddingBottom: 80, textAlign: "center" }}>
        {/* Sunrise mark */}
        <div
          style={{
            color: "var(--color-spice-turmeric)",
            marginBottom: 32,
          }}
        >
          <BrandMark size={32} />
        </div>

        {/* Heading */}
        <h1
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 36,
            fontWeight: 400,
            letterSpacing: "-0.02em",
            color: "var(--color-text-primary)",
            margin: "0 0 12px 0",
          }}
        >
          No briefing yet
        </h1>

        {/* Message */}
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 16,
            color: "var(--color-text-secondary)",
            maxWidth: 400,
            marginLeft: "auto",
            marginRight: "auto",
            lineHeight: 1.6,
            marginBottom: 32,
          }}
        >
          {message}
        </p>

        {/* Generate button */}
        {onGenerate && (
          <button
            onClick={onGenerate}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 13,
              fontWeight: 500,
              letterSpacing: "0.04em",
              padding: "12px 32px",
              borderRadius: 8,
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

        {/* Google connect card */}
        {isUnauthed && (
          <div
            style={{
              maxWidth: 400,
              marginLeft: "auto",
              marginRight: "auto",
              marginTop: 32,
              padding: "20px 24px",
              borderRadius: 16,
              border: "1px dashed var(--color-rule-heavy)",
              display: "flex",
              alignItems: "center",
              gap: 16,
              textAlign: "left",
            }}
          >
            <Mail size={20} style={{ color: "var(--color-text-tertiary)", flexShrink: 0 }} />
            <div style={{ flex: 1 }}>
              <div
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 14,
                  fontWeight: 500,
                  color: "var(--color-text-primary)",
                }}
              >
                Connect Google
              </div>
              <div
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 13,
                  color: "var(--color-text-tertiary)",
                  marginTop: 2,
                }}
              >
                Add calendar and email for a complete briefing
              </div>
            </div>
            <button
              onClick={connect}
              disabled={authLoading}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                fontWeight: 500,
                letterSpacing: "0.04em",
                textTransform: "uppercase" as const,
                padding: "6px 16px",
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

        {/* Footnote */}
        <p
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 14,
            fontStyle: "italic",
            color: "var(--color-text-tertiary)",
            marginTop: 48,
          }}
        >
          Grab a coffee — your day will be ready soon.
        </p>
      </div>
    </div>
  );
}
