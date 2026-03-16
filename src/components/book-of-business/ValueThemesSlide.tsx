/**
 * ValueThemesSlide — Value delivered + cross-portfolio themes.
 * Combined into one slide for presentation flow.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { BookOfBusinessContent } from "@/types/reports";

interface ValueThemesSlideProps {
  content: BookOfBusinessContent;
  onUpdate: (content: BookOfBusinessContent) => void;
}

export function ValueThemesSlide({ content, onUpdate }: ValueThemesSlideProps) {
  const [hoveredValue, setHoveredValue] = useState<number | null>(null);
  const [hoveredTheme, setHoveredTheme] = useState<number | null>(null);

  return (
    <section
      id="value-themes"
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
      {/* Value Delivered */}
      {content.valueDelivered.length > 0 && (
        <>
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.12em",
              color: "var(--color-spice-turmeric)",
              marginBottom: 24,
            }}
          >
            Value Delivered
          </div>

          <div style={{ marginBottom: 48, maxWidth: 800 }}>
            {content.valueDelivered.map((item, vi) => (
              <div
                key={vi}
                onMouseEnter={() => setHoveredValue(vi)}
                onMouseLeave={() => setHoveredValue(null)}
                style={{
                  display: "flex",
                  gap: 16,
                  alignItems: "baseline",
                  padding: "14px 0",
                  borderBottom: "1px solid var(--color-rule-light)",
                }}
              >
                <span
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 10,
                    fontWeight: 600,
                    textTransform: "uppercase",
                    letterSpacing: "0.08em",
                    color: "var(--color-text-tertiary)",
                    minWidth: 100,
                    flexShrink: 0,
                  }}
                >
                  {item.accountName}
                </span>
                <div style={{ flex: 1 }}>
                  <EditableText
                    value={item.headlineOutcome}
                    onChange={(v) => {
                      const next = [...content.valueDelivered];
                      next[vi] = { ...next[vi], headlineOutcome: v };
                      onUpdate({ ...content, valueDelivered: next });
                    }}
                    multiline={false}
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 17,
                      fontWeight: 500,
                      color: "var(--color-text-primary)",
                    }}
                  />
                  <EditableText
                    as="div"
                    value={item.whyItMatters}
                    onChange={(v) => {
                      const next = [...content.valueDelivered];
                      next[vi] = { ...next[vi], whyItMatters: v };
                      onUpdate({ ...content, valueDelivered: next });
                    }}
                    multiline={false}
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 14,
                      color: "var(--color-text-secondary)",
                      marginTop: 4,
                    }}
                  />
                </div>
                {content.valueDelivered.length > 1 && (
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      onUpdate({ ...content, valueDelivered: content.valueDelivered.filter((_, j) => j !== vi) });
                    }}
                    style={{
                      opacity: hoveredValue === vi ? 0.6 : 0,
                      transition: "opacity 0.15s",
                      background: "none",
                      border: "none",
                      cursor: "pointer",
                      padding: "4px 6px",
                      fontSize: 14,
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
        </>
      )}

      {/* Key Themes */}
      {content.keyThemes.length > 0 && (
        <>
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
            Portfolio Themes
          </div>

          <div style={{ maxWidth: 800 }}>
            {content.keyThemes.map((theme, ti) => (
              <div
                key={ti}
                onMouseEnter={() => setHoveredTheme(ti)}
                onMouseLeave={() => setHoveredTheme(null)}
                style={{
                  marginBottom: 28,
                  paddingBottom: 28,
                  borderBottom:
                    ti < content.keyThemes.length - 1
                      ? "1px solid var(--color-rule-light)"
                      : "none",
                  display: "flex",
                  gap: 16,
                }}
              >
                <div style={{ flex: 1 }}>
                  <EditableText
                    as="h2"
                    value={theme.title}
                    onChange={(v) => {
                      const next = [...content.keyThemes];
                      next[ti] = { ...next[ti], title: v };
                      onUpdate({ ...content, keyThemes: next });
                    }}
                    multiline={false}
                    style={{
                      fontFamily: "var(--font-serif)",
                      fontSize: 22,
                      fontWeight: 400,
                      color: "var(--color-text-primary)",
                      margin: "0 0 8px",
                    }}
                  />
                  <EditableText
                    as="p"
                    value={theme.narrative}
                    onChange={(v) => {
                      const next = [...content.keyThemes];
                      next[ti] = { ...next[ti], narrative: v };
                      onUpdate({ ...content, keyThemes: next });
                    }}
                    multiline
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 16,
                      lineHeight: 1.5,
                      color: "var(--color-text-primary)",
                      margin: 0,
                    }}
                  />
                  {theme.citedAccounts.length > 0 && (
                    <div
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 11,
                        color: "var(--color-text-tertiary)",
                        marginTop: 8,
                      }}
                    >
                      {theme.citedAccounts.join(" \u00b7 ")}
                    </div>
                  )}
                </div>
                {content.keyThemes.length > 1 && (
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      onUpdate({ ...content, keyThemes: content.keyThemes.filter((_, j) => j !== ti) });
                    }}
                    style={{
                      opacity: hoveredTheme === ti ? 0.6 : 0,
                      transition: "opacity 0.15s",
                      background: "none",
                      border: "none",
                      cursor: "pointer",
                      padding: "4px 6px",
                      fontSize: 14,
                      color: "var(--color-text-tertiary)",
                      flexShrink: 0,
                      alignSelf: "flex-start",
                    }}
                    aria-label="Remove"
                  >
                    ✕
                  </button>
                )}
              </div>
            ))}
          </div>
        </>
      )}
    </section>
  );
}
