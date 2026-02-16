/**
 * GeneratingProgress — shared phase-step loading screen for long-running
 * workflows (weekly forecast, risk briefing, etc.).
 *
 * Renders a vertical step list with circle indicators (complete / current / pending),
 * a rotating editorial quote, and an elapsed timer.
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
    if (elapsedProp != null) return; // parent manages elapsed
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
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        minHeight: "70vh",
        padding: "80px 40px",
      }}
    >
      {/* Title */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: accentColor,
          marginBottom: 40,
        }}
      >
        {title}
      </div>

      {/* Phase list */}
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          width: "100%",
          maxWidth: 480,
          marginBottom: 48,
        }}
      >
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
                opacity: isPending ? 0.3 : 1,
                transition: "opacity 0.5s ease",
              }}
            >
              <div
                style={{
                  width: 24,
                  height: 24,
                  borderRadius: "50%",
                  border: `2px solid ${
                    isComplete
                      ? "var(--color-garden-sage)"
                      : isCurrent
                        ? accentColor
                        : "var(--color-rule-light)"
                  }`,
                  background: isComplete
                    ? "var(--color-garden-sage)"
                    : "transparent",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  flexShrink: 0,
                  transition: "all 0.3s ease",
                }}
              >
                {isComplete && (
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
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
                      width: 8,
                      height: 8,
                      borderRadius: "50%",
                      background: accentColor,
                      animation: "generating-pulse 1.5s ease infinite",
                    }}
                  />
                )}
              </div>

              <div>
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
                      marginTop: 2,
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

      {/* Rotating quote */}
      <p
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 17,
          fontStyle: "italic",
          color: "var(--color-text-tertiary)",
          textAlign: "center",
          maxWidth: 400,
          lineHeight: 1.5,
          marginBottom: 24,
          transition: "opacity 0.5s ease",
        }}
      >
        &ldquo;{shuffled.current[quoteIndex]}&rdquo;
      </p>

      {/* Timer + hint */}
      <div style={{ textAlign: "center" }}>
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            color: "var(--color-text-tertiary)",
            letterSpacing: "0.04em",
          }}
        >
          {formatElapsed(elapsed)}
        </div>
        <div
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 12,
            color: "var(--color-text-tertiary)",
            opacity: 0.6,
            marginTop: 8,
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
      `}</style>
    </div>
  );
}
