/**
 * QuarterlyFocusSlide — Quarter-to-quarter focus areas.
 * Slide 12: three columns — Retention, Expansion, Execution — each an editable bullet list.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { BookOfBusinessContent, QuarterlyFocus } from "@/types/reports";

interface QuarterlyFocusSlideProps {
  content: BookOfBusinessContent;
  onUpdate: (content: BookOfBusinessContent) => void;
}

const COLUMNS: { key: keyof QuarterlyFocus; label: string }[] = [
  { key: "retention", label: "Retention" },
  { key: "expansion", label: "Expansion" },
  { key: "execution", label: "Execution" },
];

export function QuarterlyFocusSlide({ content, onUpdate }: QuarterlyFocusSlideProps) {
  const [hoveredBullet, setHoveredBullet] = useState<{ col: string; idx: number } | null>(null);

  const updateBullet = (col: keyof QuarterlyFocus, index: number, value: string) => {
    const list = [...content.quarterlyFocus[col]];
    list[index] = value;
    onUpdate({ ...content, quarterlyFocus: { ...content.quarterlyFocus, [col]: list } });
  };

  const removeBullet = (col: keyof QuarterlyFocus, index: number) => {
    onUpdate({
      ...content,
      quarterlyFocus: {
        ...content.quarterlyFocus,
        [col]: content.quarterlyFocus[col].filter((_, i) => i !== index),
      },
    });
  };

  const addBullet = (col: keyof QuarterlyFocus) => {
    onUpdate({
      ...content,
      quarterlyFocus: {
        ...content.quarterlyFocus,
        [col]: [...content.quarterlyFocus[col], ""],
      },
    });
  };

  return (
    <section
      id="quarterly-focus"
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
      <div style={{ fontFamily: "var(--font-mono)", fontSize: 12, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.12em", color: "var(--color-spice-turmeric)", marginBottom: 40 }}>
        Quarterly Focus
      </div>

      <div style={{ display: "flex", gap: 64, maxWidth: 900 }}>
        {COLUMNS.map(({ key, label }) => (
          <div key={key} style={{ flex: 1 }}>
            <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--color-text-tertiary)", marginBottom: 16, paddingBottom: 8, borderBottom: "2px solid var(--color-rule-heavy)" }}>
              {label}
            </div>

            {content.quarterlyFocus[key].map((item, bi) => (
              <div
                key={bi}
                onMouseEnter={() => setHoveredBullet({ col: key, idx: bi })}
                onMouseLeave={() => setHoveredBullet(null)}
                style={{ display: "flex", alignItems: "baseline", gap: 10, paddingBottom: 8 }}
              >
                <span style={{ width: 4, height: 4, borderRadius: "50%", background: "var(--color-spice-turmeric)", flexShrink: 0, marginTop: 8 }} />
                <EditableText
                  value={item}
                  onChange={(v) => updateBullet(key, bi, v)}
                  multiline={false}
                  placeholder="Add focus item..."
                  style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--color-text-primary)", flex: 1 }}
                />
                {content.quarterlyFocus[key].length > 1 && (
                  <button
                    onClick={(e) => { e.stopPropagation(); removeBullet(key, bi); }}
                    style={{ opacity: hoveredBullet?.col === key && hoveredBullet?.idx === bi ? 0.6 : 0, transition: "opacity 0.15s", background: "none", border: "none", cursor: "pointer", padding: "2px 4px", fontSize: 12, color: "var(--color-text-tertiary)", flexShrink: 0 }}
                    aria-label="Remove"
                  >
                    ✕
                  </button>
                )}
              </div>
            ))}

            {content.quarterlyFocus[key].length === 0 && (
              <div style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-tertiary)", padding: "8px 0" }}>
                No items yet.
              </div>
            )}

            <button
              onClick={() => addBullet(key)}
              style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.06em", color: "var(--color-spice-turmeric)", background: "none", border: "none", cursor: "pointer", padding: "8px 0", textAlign: "left" }}
            >
              + Add
            </button>
          </div>
        ))}
      </div>
    </section>
  );
}
