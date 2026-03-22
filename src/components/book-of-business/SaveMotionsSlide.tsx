/**
 * SaveMotionsSlide — Save motion table.
 * Slide 5: account save motions with risk, timeline, success signals.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { BookOfBusinessContent, SaveMotion } from "@/types/reports";

interface SaveMotionsSlideProps {
  content: BookOfBusinessContent;
  onUpdate: (content: BookOfBusinessContent) => void;
}

export function SaveMotionsSlide({ content, onUpdate }: SaveMotionsSlideProps) {
  const [hoveredRow, setHoveredRow] = useState<number | null>(null);

  const updateRow = (index: number, patch: Partial<SaveMotion>) => {
    const next = [...content.saveMotions];
    next[index] = { ...next[index], ...patch };
    onUpdate({ ...content, saveMotions: next });
  };

  const removeRow = (index: number) => {
    onUpdate({ ...content, saveMotions: content.saveMotions.filter((_, i) => i !== index) });
  };

  const addRow = () => {
    onUpdate({
      ...content,
      saveMotions: [
        ...content.saveMotions,
        { accountName: "New Account", risk: "", saveMotion: "", timeline: "", successSignals: "" },
      ],
    });
  };

  return (
    <section
      id="save-motions"
      className="report-surface-slide"
      style={{ scrollMarginTop: 60 }}
    >
      <div style={{ fontFamily: "var(--font-mono)", fontSize: 12, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.12em", color: "var(--color-spice-turmeric)", marginBottom: 24 }}>
        Save Motions
      </div>

      {/* Table header */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 100px 1fr 100px 1fr 32px", gap: 12, paddingBottom: 8, borderBottom: "2px solid var(--color-rule-heavy)", maxWidth: 960 }}>
        {["Account", "Risk", "Save Motion", "Timeline", "Success Signals", ""].map((h) => (
          <div key={h} style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.08em", color: "var(--color-text-tertiary)" }}>
            {h}
          </div>
        ))}
      </div>

      {/* Rows */}
      {content.saveMotions.map((row, i) => (
        <div
          key={i}
          onMouseEnter={() => setHoveredRow(i)}
          onMouseLeave={() => setHoveredRow(null)}
          style={{ display: "grid", gridTemplateColumns: "1fr 100px 1fr 100px 1fr 32px", gap: 12, padding: "10px 0", borderBottom: "1px solid var(--color-rule-light)", maxWidth: 960, alignItems: "baseline" }}
        >
          <EditableText value={row.accountName} onChange={(v) => updateRow(i, { accountName: v })} multiline={false} style={{ fontFamily: "var(--font-sans)", fontSize: 14, fontWeight: 500, color: "var(--color-text-primary)" }} />
          <EditableText value={row.risk} onChange={(v) => updateRow(i, { risk: v })} multiline={false} style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--color-spice-terracotta)" }} />
          <EditableText value={row.saveMotion} onChange={(v) => updateRow(i, { saveMotion: v })} multiline={false} placeholder="Save motion..." style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-primary)" }} />
          <EditableText value={row.timeline} onChange={(v) => updateRow(i, { timeline: v })} multiline={false} style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--color-text-secondary)" }} />
          <EditableText value={row.successSignals} onChange={(v) => updateRow(i, { successSignals: v })} multiline={false} placeholder="Success signals..." style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-secondary)" }} />
          {content.saveMotions.length > 1 && (
            <button onClick={() => removeRow(i)} style={{ opacity: hoveredRow === i ? 0.6 : 0, transition: "opacity 0.15s", background: "none", border: "none", cursor: "pointer", fontSize: 14, color: "var(--color-text-tertiary)" }} aria-label="Remove">
              ✕
            </button>
          )}
        </div>
      ))}

      {content.saveMotions.length === 0 && (
        <div style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--color-text-tertiary)", padding: "20px 0" }}>
          No save motions defined.
        </div>
      )}

      <button onClick={addRow} style={{ fontFamily: "var(--font-mono)", fontSize: 11, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.06em", color: "var(--color-spice-turmeric)", background: "none", border: "none", cursor: "pointer", padding: "12px 0", textAlign: "left" }}>
        + Add Row
      </button>
    </section>
  );
}
