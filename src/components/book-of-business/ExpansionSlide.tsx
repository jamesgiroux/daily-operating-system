/**
 * ExpansionSlide — Expansion potential + expansion readiness tables.
 * Slides 6+7 combined: two tables on one full-viewport slide.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import { formatArr } from "@/lib/utils";
import type { BookOfBusinessContent, ExpansionRow, ExpansionReadiness } from "@/types/reports";

interface ExpansionSlideProps {
  content: BookOfBusinessContent;
  onUpdate: (content: BookOfBusinessContent) => void;
}

export function ExpansionSlide({ content, onUpdate }: ExpansionSlideProps) {
  const [hoveredExpRow, setHoveredExpRow] = useState<number | null>(null);
  const [hoveredReadyRow, setHoveredReadyRow] = useState<number | null>(null);

  /* Expansion accounts helpers */
  const updateExpRow = (index: number, patch: Partial<ExpansionRow>) => {
    const next = [...content.expansionAccounts];
    next[index] = { ...next[index], ...patch };
    onUpdate({ ...content, expansionAccounts: next });
  };

  const removeExpRow = (index: number) => {
    onUpdate({ ...content, expansionAccounts: content.expansionAccounts.filter((_, i) => i !== index) });
  };

  const addExpRow = () => {
    onUpdate({
      ...content,
      expansionAccounts: [
        ...content.expansionAccounts,
        { accountName: "New Account", arr: 0, readiness: "", expansionType: "", estimatedValue: "", timing: "" },
      ],
    });
  };

  /* Expansion readiness helpers */
  const updateReadyRow = (index: number, patch: Partial<ExpansionReadiness>) => {
    const next = [...content.expansionReadiness];
    next[index] = { ...next[index], ...patch };
    onUpdate({ ...content, expansionReadiness: next });
  };

  const removeReadyRow = (index: number) => {
    onUpdate({ ...content, expansionReadiness: content.expansionReadiness.filter((_, i) => i !== index) });
  };

  const addReadyRow = () => {
    onUpdate({
      ...content,
      expansionReadiness: [
        ...content.expansionReadiness,
        { accountName: "New Account", readiness: "", primaryRisk: "", nextAction: "" },
      ],
    });
  };

  return (
    <section
      id="expansion"
      className="report-surface-slide"
      style={{ scrollMarginTop: 60 }}
    >
      {/* — Expansion Potential — */}
      <div style={{ fontFamily: "var(--font-mono)", fontSize: 12, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.12em", color: "var(--color-spice-turmeric)", marginBottom: 24 }}>
        Expansion Potential
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 90px 90px 110px 100px 90px 32px", gap: 12, paddingBottom: 8, borderBottom: "2px solid var(--color-rule-heavy)", maxWidth: 960 }}>
        {["Account", "ARR", "Readiness", "Type", "Est. Value", "Timing", ""].map((h) => (
          <div key={h} style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.08em", color: "var(--color-text-tertiary)" }}>
            {h}
          </div>
        ))}
      </div>

      {content.expansionAccounts.map((row, i) => (
        <div
          key={i}
          onMouseEnter={() => setHoveredExpRow(i)}
          onMouseLeave={() => setHoveredExpRow(null)}
          style={{ display: "grid", gridTemplateColumns: "1fr 90px 90px 110px 100px 90px 32px", gap: 12, padding: "10px 0", borderBottom: "1px solid var(--color-rule-light)", maxWidth: 960, alignItems: "baseline" }}
        >
          <EditableText value={row.accountName} onChange={(v) => updateExpRow(i, { accountName: v })} multiline={false} style={{ fontFamily: "var(--font-sans)", fontSize: 14, fontWeight: 500, color: "var(--color-text-primary)" }} />
          <EditableText value={`$${formatArr(row.arr)}`} onChange={(v) => { const n = parseFloat(v.replace(/[^0-9.]/g, "")); if (!isNaN(n)) updateExpRow(i, { arr: n }); }} multiline={false} style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-text-primary)" }} />
          <EditableText value={row.readiness} onChange={(v) => updateExpRow(i, { readiness: v })} multiline={false} style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--color-text-secondary)" }} />
          <EditableText value={row.expansionType} onChange={(v) => updateExpRow(i, { expansionType: v })} multiline={false} style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-secondary)" }} />
          <EditableText value={row.estimatedValue} onChange={(v) => updateExpRow(i, { estimatedValue: v })} multiline={false} style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-spice-turmeric)" }} />
          <EditableText value={row.timing} onChange={(v) => updateExpRow(i, { timing: v })} multiline={false} style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--color-text-secondary)" }} />
          {content.expansionAccounts.length > 1 && (
            <button onClick={() => removeExpRow(i)} style={{ opacity: hoveredExpRow === i ? 0.6 : 0, transition: "opacity 0.15s", background: "none", border: "none", cursor: "pointer", fontSize: 14, color: "var(--color-text-tertiary)" }} aria-label="Remove">
              ✕
            </button>
          )}
        </div>
      ))}

      {content.expansionAccounts.length === 0 && (
        <div style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--color-text-tertiary)", padding: "20px 0" }}>
          No expansion accounts identified.
        </div>
      )}

      <button onClick={addExpRow} style={{ fontFamily: "var(--font-mono)", fontSize: 11, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.06em", color: "var(--color-spice-turmeric)", background: "none", border: "none", cursor: "pointer", padding: "12px 0", textAlign: "left" }}>
        + Add Row
      </button>

      {/* — Expansion Readiness — */}
      <div style={{ fontFamily: "var(--font-mono)", fontSize: 12, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.12em", color: "var(--color-text-secondary)", marginBottom: 24, marginTop: 56 }}>
        Expansion Readiness
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 100px 1fr 1fr 32px", gap: 12, paddingBottom: 8, borderBottom: "2px solid var(--color-rule-heavy)", maxWidth: 960 }}>
        {["Account", "Readiness", "Primary Risk", "Next Action", ""].map((h) => (
          <div key={h} style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.08em", color: "var(--color-text-tertiary)" }}>
            {h}
          </div>
        ))}
      </div>

      {content.expansionReadiness.map((row, i) => (
        <div
          key={i}
          onMouseEnter={() => setHoveredReadyRow(i)}
          onMouseLeave={() => setHoveredReadyRow(null)}
          style={{ display: "grid", gridTemplateColumns: "1fr 100px 1fr 1fr 32px", gap: 12, padding: "10px 0", borderBottom: "1px solid var(--color-rule-light)", maxWidth: 960, alignItems: "baseline" }}
        >
          <EditableText value={row.accountName} onChange={(v) => updateReadyRow(i, { accountName: v })} multiline={false} style={{ fontFamily: "var(--font-sans)", fontSize: 14, fontWeight: 500, color: "var(--color-text-primary)" }} />
          <EditableText value={row.readiness} onChange={(v) => updateReadyRow(i, { readiness: v })} multiline={false} style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--color-text-secondary)" }} />
          <EditableText value={row.primaryRisk} onChange={(v) => updateReadyRow(i, { primaryRisk: v })} multiline={false} placeholder="Primary risk..." style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-secondary)" }} />
          <EditableText value={row.nextAction} onChange={(v) => updateReadyRow(i, { nextAction: v })} multiline={false} placeholder="Next action..." style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-primary)" }} />
          {content.expansionReadiness.length > 1 && (
            <button onClick={() => removeReadyRow(i)} style={{ opacity: hoveredReadyRow === i ? 0.6 : 0, transition: "opacity 0.15s", background: "none", border: "none", cursor: "pointer", fontSize: 14, color: "var(--color-text-tertiary)" }} aria-label="Remove">
              ✕
            </button>
          )}
        </div>
      ))}

      {content.expansionReadiness.length === 0 && (
        <div style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--color-text-tertiary)", padding: "20px 0" }}>
          No expansion readiness data.
        </div>
      )}

      <button onClick={addReadyRow} style={{ fontFamily: "var(--font-mono)", fontSize: 11, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.06em", color: "var(--color-spice-turmeric)", background: "none", border: "none", cursor: "pointer", padding: "12px 0", textAlign: "left" }}>
        + Add Row
      </button>
    </section>
  );
}
