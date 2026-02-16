/**
 * DashboardEmpty — Editorial empty state for the daily briefing.
 * Renders inside MagazinePageLayout's page container.
 * Warm language, serif typography, prominent generate action.
 */

import { Mail } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";
import type { GoogleAuthStatus } from "@/types";

interface DashboardEmptyProps {
  message: string;
  onGenerate?: () => void;
  googleAuth?: GoogleAuthStatus;
}

export function DashboardEmpty({ message, onGenerate, googleAuth }: DashboardEmptyProps) {
  const { connect, loading: authLoading } = useGoogleAuth();
  const isUnauthed = googleAuth?.status === "notconfigured";

  return (
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
      <div style={{ paddingTop: 120, paddingBottom: 80, textAlign: "center" }}>
        {/* Sunrise mark */}
        <div
          style={{
            fontFamily: "var(--font-mark)",
            fontSize: 32,
            letterSpacing: "0.4em",
            color: "var(--color-spice-turmeric)",
            marginBottom: 32,
          }}
        >
          *
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
            Generate Briefing
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
            <Button
              size="sm"
              variant="outline"
              onClick={connect}
              disabled={authLoading}
              style={{ fontFamily: "var(--font-sans)", fontSize: 13 }}
            >
              Connect
            </Button>
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
