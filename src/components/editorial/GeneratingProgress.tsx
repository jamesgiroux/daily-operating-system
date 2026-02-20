/**
 * GeneratingProgress — shared phase-step loading screen for long-running
 * workflows (daily briefing, weekly forecast, risk briefing, etc.).
 *
 * Renders a vertical step list with circle indicators (complete / current /
 * pending), a rotating editorial quote with pull-quote treatment, and an
 * elapsed timer. Quotes rotate on an 8-second interval.
 */
import { useState, useEffect, useRef } from "react";

export interface ProgressPhase {
  key: string;
  label: string;
  detail: string;
}

interface GeneratingProgressProps {
  /** Heading displayed above the phase list */
  title: string;
  /** Accent color CSS variable (e.g. "var(--color-garden-larkspur)") */
  accentColor: string;
  /** Phase definitions — order determines display order */
  phases: ProgressPhase[];
  /** Key of the currently active phase */
  currentPhaseKey: string;
  /** Rotating quotes shown below the phase list */
  quotes: string[];
  /** Elapsed seconds (optional — if omitted, the component tracks its own) */
  elapsed?: number;
}

export function GeneratingProgress({
  title,
  accentColor,
  phases,
  currentPhaseKey,
  quotes: quotesProp,
  elapsed: elapsedProp,
}: GeneratingProgressProps) {
  const [quoteIndex, setQuoteIndex] = useState(0);
  const [localElapsed, setLocalElapsed] = useState(0);
  const startTime = useRef(Date.now());
  const shuffled = useRef([...quotesProp].sort(() => Math.random() - 0.5));

  const elapsed = elapsedProp ?? localElapsed;
  const currentPhaseIndex = phases.findIndex((p) => p.key === currentPhaseKey);

  useEffect(() => {
    const interval = setInterval(() => {
      setQuoteIndex((i) => (i + 1) % shuffled.current.length);
    }, 8000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    if (elapsedProp != null) return;
    const interval = setInterval(() => {
      setLocalElapsed(Math.floor((Date.now() - startTime.current) / 1000));
    }, 1000);
    return () => clearInterval(interval);
  }, [elapsedProp]);

  const formatElapsed = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return m > 0 ? `${m}m ${s}s` : `${s}s`;
  };

  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "100px 32px 1fr",
        paddingTop: 80,
        paddingBottom: 96,
      }}
    >
      {/* ── Label column ──────────────────────────────────────────────── */}
      <div style={{ paddingTop: 6 }}>
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            fontWeight: 600,
            letterSpacing: "0.1em",
            textTransform: "uppercase" as const,
            color: accentColor,
          }}
        >
          {formatElapsed(elapsed)}
        </div>
      </div>

      <div />

      {/* ── Content column ────────────────────────────────────────────── */}
      <div style={{ maxWidth: 520 }}>

        {/* Section rule */}
        <div style={{ borderTop: "1px solid var(--color-rule-heavy)", marginBottom: 32 }} />

        {/* Title */}
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            fontWeight: 600,
            textTransform: "uppercase" as const,
            letterSpacing: "0.1em",
            color: accentColor,
            marginBottom: 32,
          }}
        >
          {title}
        </div>

        {/* Phase list */}
        <div style={{ marginBottom: 56 }}>
          {phases.map((phase, i) => {
            const isComplete = i < currentPhaseIndex;
            const isCurrent = i === currentPhaseIndex;
            const isPending = i > currentPhaseIndex;

            return (
              <div
                key={phase.key}
                style={{
                  display: "flex",
                  gap: 16,
                  alignItems: "flex-start",
                  padding: "10px 0",
                  borderBottom: i < phases.length - 1
                    ? "1px solid var(--color-rule-light)"
                    : "none",
                  opacity: isPending ? 0.3 : 1,
                  transition: "opacity 0.5s ease",
                }}
              >
                {/* Phase indicator circle */}
                <div
                  style={{
                    width: 20,
                    height: 20,
                    borderRadius: "50%",
                    border: `2px solid ${
                      isComplete
                        ? "var(--color-garden-sage)"
                        : isCurrent
                          ? accentColor
                          : "var(--color-rule-light)"
                    }`,
                    background: isComplete ? "var(--color-garden-sage)" : "transparent",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    flexShrink: 0,
                    marginTop: 2,
                    transition: "all 0.3s ease",
                  }}
                >
                  {isComplete && (
                    <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
                      <path
                        d="M2 6l3 3 5-5"
                        stroke="white"
                        strokeWidth="2"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                      />
                    </svg>
                  )}
                  {isCurrent && (
                    <div
                      style={{
                        width: 6,
                        height: 6,
                        borderRadius: "50%",
                        background: accentColor,
                        animation: "generating-pulse 1.5s ease infinite",
                      }}
                    />
                  )}
                </div>

                {/* Phase text */}
                <div style={{ paddingTop: 1 }}>
                  <div
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 14,
                      fontWeight: isCurrent ? 600 : 400,
                      color: isCurrent
                        ? "var(--color-text-primary)"
                        : isComplete
                          ? "var(--color-text-secondary)"
                          : "var(--color-text-tertiary)",
                      transition: "all 0.3s ease",
                    }}
                  >
                    {phase.label}
                  </div>
                  {isCurrent && (
                    <div
                      style={{
                        fontFamily: "var(--font-sans)",
                        fontSize: 12,
                        color: "var(--color-text-tertiary)",
                        marginTop: 3,
                      }}
                    >
                      {phase.detail}
                    </div>
                  )}
                </div>
              </div>
            );
          })}
        </div>

        {/* ── Pull quote ──────────────────────────────────────────────── */}
        <div>
          <div style={{ borderTop: "1px solid var(--color-rule-light)", marginBottom: 20 }} />
          <p
            key={quoteIndex}
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 16,
              fontStyle: "italic",
              fontWeight: 300,
              color: "var(--color-text-tertiary)",
              lineHeight: 1.6,
              margin: 0,
              animation: "quote-fade 0.6s ease",
            }}
          >
            {shuffled.current[quoteIndex]}
          </p>
          <div style={{ borderTop: "1px solid var(--color-rule-light)", marginTop: 20 }} />
        </div>

        {/* Navigate away hint */}
        <div
          style={{
            marginTop: 20,
            fontFamily: "var(--font-sans)",
            fontSize: 12,
            color: "var(--color-text-tertiary)",
            opacity: 0.6,
          }}
        >
          This runs in the background — feel free to navigate away
        </div>
      </div>

      <style>{`
        @keyframes generating-pulse {
          0%, 100% { opacity: 1; transform: scale(1); }
          50% { opacity: 0.5; transform: scale(0.8); }
        }
        @keyframes quote-fade {
          from { opacity: 0; transform: translateY(4px); }
          to { opacity: 1; transform: translateY(0); }
        }
      `}</style>
    </div>
  );
}
