import { ArrowRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { BrandMark } from "@/components/ui/BrandMark";

interface WelcomeProps {
  onNext: () => void;
}

export function Welcome({ onNext }: WelcomeProps) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 32 }}>
      {/* Brand mark */}
      <div style={{ color: "var(--color-spice-turmeric)" }}>
        <BrandMark size={48} />
      </div>

      {/* Hero headline — serif, left-aligned */}
      <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
        <h1
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 32,
            fontWeight: 400,
            lineHeight: 1.2,
            letterSpacing: "-0.01em",
            color: "var(--color-text-primary)",
            margin: 0,
          }}
        >
          Open the app. Your day is ready.
        </h1>
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 15,
            lineHeight: 1.6,
            color: "var(--color-text-secondary)",
            margin: 0,
            maxWidth: 480,
          }}
        >
          DailyOS prepares your day while you sleep — meeting prep,
          email triage, actions due, and a morning summary. You open it,
          read, and get to work.
        </p>
      </div>

      {/* Timeline block — editorial rule-separated */}
      <div
        style={{
          borderTop: "1px solid var(--color-rule-light)",
          paddingTop: 20,
        }}
      >
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            fontWeight: 500,
            textTransform: "uppercase" as const,
            letterSpacing: "0.1em",
            color: "var(--color-text-tertiary)",
            marginBottom: 12,
          }}
        >
          What it looks like
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <div style={{ display: "flex", alignItems: "baseline", gap: 12 }}>
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                color: "var(--color-text-tertiary)",
                flexShrink: 0,
                width: 56,
              }}
            >
              6:00 AM
            </span>
            <span style={{ fontSize: 14, color: "var(--color-text-secondary)" }}>
              Your briefing generates automatically
            </span>
          </div>
          <div style={{ display: "flex", alignItems: "baseline", gap: 12 }}>
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                color: "var(--color-text-tertiary)",
                flexShrink: 0,
                width: 56,
              }}
            >
              8:00 AM
            </span>
            <span style={{ fontSize: 14, color: "var(--color-text-secondary)" }}>
              You open the app. Everything's there.
            </span>
          </div>
          <div style={{ display: "flex", alignItems: "baseline", gap: 12 }}>
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                color: "var(--color-spice-turmeric)",
                flexShrink: 0,
                width: 56,
              }}
            >
              8:15 AM
            </span>
            <span style={{ fontSize: 14, color: "var(--color-text-primary)", fontWeight: 500 }}>
              You're prepared. Close the app. Do your work.
            </span>
          </div>
        </div>
      </div>

      {/* Tagline */}
      <p
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 17,
          fontStyle: "italic",
          fontWeight: 300,
          lineHeight: 1.55,
          color: "var(--color-text-tertiary)",
          margin: 0,
        }}
      >
        No setup to maintain. No inbox to clear.
        Skip a day, skip a week — it picks up where you are.
      </p>

      <div>
        <Button size="lg" onClick={onNext}>
          Let's get started
          <ArrowRight className="ml-2 size-4" />
        </Button>
      </div>
    </div>
  );
}
