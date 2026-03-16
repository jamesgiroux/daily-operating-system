/**
 * SpotlightSlide — One account deep dive per slide.
 * Each notable account gets a full-viewport editorial treatment.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import { formatArr } from "@/lib/utils";
import type { AccountDeepDive, BookOfBusinessContent } from "@/types/reports";

interface SpotlightSlideProps {
  dive: AccountDeepDive;
  index: number;
  total: number;
  content: BookOfBusinessContent;
  onUpdate: (content: BookOfBusinessContent) => void;
}

export function SpotlightSlide({ dive, index, total, content, onUpdate }: SpotlightSlideProps) {
  const di = content.deepDives.findIndex((d) => d.accountId === dive.accountId);
  const [hoveredWs, setHoveredWs] = useState<number | null>(null);
  const [hoveredRg, setHoveredRg] = useState<number | null>(null);

  return (
    <section
      id={`spotlight-${index}`}
      style={{
        scrollMarginTop: 60,
        minHeight: "100vh",
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "120px 120px 80px",
        scrollSnapAlign: "start",
      }}
    >
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "var(--color-text-secondary)",
          marginBottom: 24,
        }}
      >
        Account Spotlight {index} of {total}
      </div>

      {/* Account name — large serif */}
      <h2
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 40,
          fontWeight: 400,
          lineHeight: 1.15,
          letterSpacing: "-0.02em",
          color: "var(--color-text-primary)",
          margin: "0 0 16px",
          maxWidth: 700,
        }}
      >
        {dive.accountName}
      </h2>

      {/* ARR + Renewal callout */}
      <div
        style={{
          display: "flex",
          gap: 24,
          alignItems: "baseline",
          marginBottom: 32,
        }}
      >
        {dive.arr != null && (
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 22,
              fontWeight: 600,
              color: "var(--color-spice-turmeric)",
            }}
          >
            ${formatArr(dive.arr)}
          </span>
        )}
        {dive.renewalDate && (
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 14,
              color: "var(--color-text-secondary)",
              letterSpacing: "0.04em",
            }}
          >
            Renewal: {dive.renewalDate}
          </span>
        )}
      </div>

      {/* Status narrative */}
      <EditableText
        as="p"
        value={dive.statusNarrative}
        onChange={(v) => {
          if (di < 0) return;
          const next = [...content.deepDives];
          next[di] = { ...next[di], statusNarrative: v };
          onUpdate({ ...content, deepDives: next });
        }}
        multiline
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 20,
          fontWeight: 400,
          lineHeight: 1.5,
          color: "var(--color-text-primary)",
          maxWidth: 800,
          margin: "0 0 32px",
        }}
      />

      {/* Renewal / Growth Impact */}
      <div
        style={{
          borderLeft: "3px solid var(--color-spice-turmeric)",
          paddingLeft: 16,
          marginBottom: 32,
          maxWidth: 800,
        }}
      >
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            fontWeight: 600,
            textTransform: "uppercase",
            letterSpacing: "0.1em",
            color: "var(--color-text-tertiary)",
            marginBottom: 6,
          }}
        >
          Revenue Impact
        </div>
        <EditableText
          value={dive.renewalOrGrowthImpact}
          onChange={(v) => {
            if (di < 0) return;
            const next = [...content.deepDives];
            next[di] = { ...next[di], renewalOrGrowthImpact: v };
            onUpdate({ ...content, deepDives: next });
          }}
          multiline={false}
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 17,
            color: "var(--color-text-primary)",
          }}
        />
      </div>

      {/* Workstreams + Risks side by side */}
      <div style={{ display: "flex", gap: 64, maxWidth: 800 }}>
        {dive.activeWorkstreams.length > 0 && (
          <div style={{ flex: 1 }}>
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                fontWeight: 600,
                textTransform: "uppercase",
                letterSpacing: "0.1em",
                color: "var(--color-text-tertiary)",
                marginBottom: 12,
              }}
            >
              Active Workstreams
            </div>
            {dive.activeWorkstreams.map((ws, wi) => (
              <div
                key={wi}
                onMouseEnter={() => setHoveredWs(wi)}
                onMouseLeave={() => setHoveredWs(null)}
                style={{
                  display: "flex",
                  alignItems: "baseline",
                  gap: 10,
                  paddingBottom: 8,
                }}
              >
                <span
                  style={{
                    width: 4,
                    height: 4,
                    borderRadius: "50%",
                    background: "var(--color-spice-turmeric)",
                    flexShrink: 0,
                    marginTop: 8,
                  }}
                />
                <EditableText
                  value={ws}
                  onChange={(v) => {
                    if (di < 0) return;
                    const next = [...content.deepDives];
                    const nextWs = [...next[di].activeWorkstreams];
                    nextWs[wi] = v;
                    next[di] = { ...next[di], activeWorkstreams: nextWs };
                    onUpdate({ ...content, deepDives: next });
                  }}
                  multiline={false}
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 15,
                    color: "var(--color-text-primary)",
                    flex: 1,
                  }}
                />
                {dive.activeWorkstreams.length > 1 && (
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      if (di < 0) return;
                      const next = [...content.deepDives];
                      next[di] = { ...next[di], activeWorkstreams: next[di].activeWorkstreams.filter((_, j) => j !== wi) };
                      onUpdate({ ...content, deepDives: next });
                    }}
                    style={{
                      opacity: hoveredWs === wi ? 0.6 : 0,
                      transition: "opacity 0.15s",
                      background: "none",
                      border: "none",
                      cursor: "pointer",
                      padding: "2px 4px",
                      fontSize: 12,
                      color: "var(--color-text-tertiary)",
                      flexShrink: 0,
                    }}
                    aria-label="Remove"
                  >
                    ✕
                  </button>
                )}
              </div>
            ))}
          </div>
        )}

        {dive.risksAndGaps.length > 0 && (
          <div style={{ flex: 1 }}>
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                fontWeight: 600,
                textTransform: "uppercase",
                letterSpacing: "0.1em",
                color: "var(--color-text-tertiary)",
                marginBottom: 12,
              }}
            >
              Risks & Gaps
            </div>
            {dive.risksAndGaps.map((rg, ri) => (
              <div
                key={ri}
                onMouseEnter={() => setHoveredRg(ri)}
                onMouseLeave={() => setHoveredRg(null)}
                style={{
                  display: "flex",
                  alignItems: "baseline",
                  gap: 10,
                  paddingBottom: 8,
                }}
              >
                <span
                  style={{
                    width: 4,
                    height: 4,
                    borderRadius: "50%",
                    background: "var(--color-spice-terracotta)",
                    flexShrink: 0,
                    marginTop: 8,
                  }}
                />
                <EditableText
                  value={rg}
                  onChange={(v) => {
                    if (di < 0) return;
                    const next = [...content.deepDives];
                    const nextRg = [...next[di].risksAndGaps];
                    nextRg[ri] = v;
                    next[di] = { ...next[di], risksAndGaps: nextRg };
                    onUpdate({ ...content, deepDives: next });
                  }}
                  multiline={false}
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 15,
                    color: "var(--color-text-primary)",
                    flex: 1,
                  }}
                />
                {dive.risksAndGaps.length > 1 && (
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      if (di < 0) return;
                      const next = [...content.deepDives];
                      next[di] = { ...next[di], risksAndGaps: next[di].risksAndGaps.filter((_, j) => j !== ri) };
                      onUpdate({ ...content, deepDives: next });
                    }}
                    style={{
                      opacity: hoveredRg === ri ? 0.6 : 0,
                      transition: "opacity 0.15s",
                      background: "none",
                      border: "none",
                      cursor: "pointer",
                      padding: "2px 4px",
                      fontSize: 12,
                      color: "var(--color-text-tertiary)",
                      flexShrink: 0,
                    }}
                    aria-label="Remove"
                  >
                    ✕
                  </button>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}
