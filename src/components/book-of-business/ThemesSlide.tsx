/**
 * ThemesSlide — Cross-portfolio key themes.
 * Slide 13: patterns across the book.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { BookOfBusinessContent } from "@/types/reports";

interface ThemesSlideProps {
  content: BookOfBusinessContent;
  onUpdate: (content: BookOfBusinessContent) => void;
}

export function ThemesSlide({ content, onUpdate }: ThemesSlideProps) {
  const [hoveredTheme, setHoveredTheme] = useState<number | null>(null);

  const addTheme = () => {
    onUpdate({
      ...content,
      keyThemes: [...content.keyThemes, { title: "New Theme", narrative: "", citedAccounts: [] }],
    });
  };

  return (
    <section
      id="themes"
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
        Key Themes
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

        {content.keyThemes.length === 0 && (
          <div style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--color-text-tertiary)", padding: "20px 0" }}>
            No themes identified.
          </div>
        )}

        <button
          onClick={addTheme}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            fontWeight: 600,
            textTransform: "uppercase",
            letterSpacing: "0.06em",
            color: "var(--color-spice-turmeric)",
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: "12px 0",
          }}
        >
          + Add Theme
        </button>
      </div>
    </section>
  );
}
