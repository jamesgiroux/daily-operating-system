/**
 * RiskTableSlide — Risk & retention concerns table.
 * Slide 3: at-risk accounts with ARR, timing, risk level, primary driver.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import { formatArr } from "@/lib/utils";
import type { BookOfBusinessContent, RiskAccountRow } from "@/types/reports";

interface RiskTableSlideProps {
  content: BookOfBusinessContent;
  onUpdate: (content: BookOfBusinessContent) => void;
}

export function RiskTableSlide({ content, onUpdate }: RiskTableSlideProps) {
  const [hoveredRow, setHoveredRow] = useState<number | null>(null);

  const updateRow = (index: number, patch: Partial<RiskAccountRow>) => {
    const next = [...content.riskAccounts];
    next[index] = { ...next[index], ...patch };
    onUpdate({ ...content, riskAccounts: next });
  };

  const removeRow = (index: number) => {
    onUpdate({ ...content, riskAccounts: content.riskAccounts.filter((_, i) => i !== index) });
  };

  const addRow = () => {
    onUpdate({
      ...content,
      riskAccounts: [
        ...content.riskAccounts,
        { accountName: "New Account", arr: 0, renewalTiming: "", riskLevel: "watch", primaryRiskDriver: "" },
      ],
    });
  };

  return (
    <section
      id="risk-table"
      className="report-surface-slide"
      style={{ scrollMarginTop: 60 }}
    >
      <div style={{ fontFamily: "var(--font-mono)", fontSize: 12, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.12em", color: "var(--color-spice-terracotta)", marginBottom: 24 }}>
        Risk & Retention Concerns
      </div>

      {/* Table header */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 100px 120px 100px 1fr 32px", gap: 12, paddingBottom: 8, borderBottom: "2px solid var(--color-rule-heavy)", maxWidth: 900 }}>
        {["Account", "ARR", "Renewal", "Risk", "Primary Driver", ""].map((h) => (
          <div key={h} style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.08em", color: "var(--color-text-tertiary)" }}>
            {h}
          </div>
        ))}
      </div>

      {/* Rows */}
      {content.riskAccounts.map((row, i) => (
        <div
          key={i}
          onMouseEnter={() => setHoveredRow(i)}
          onMouseLeave={() => setHoveredRow(null)}
          style={{ display: "grid", gridTemplateColumns: "1fr 100px 120px 100px 1fr 32px", gap: 12, padding: "10px 0", borderBottom: "1px solid var(--color-rule-light)", maxWidth: 900, alignItems: "baseline" }}
        >
          <EditableText value={row.accountName} onChange={(v) => updateRow(i, { accountName: v })} multiline={false} style={{ fontFamily: "var(--font-sans)", fontSize: 14, fontWeight: 500, color: "var(--color-text-primary)" }} />
          <EditableText value={`$${formatArr(row.arr)}`} onChange={(v) => { const n = parseFloat(v.replace(/[^0-9.]/g, "")); if (!isNaN(n)) updateRow(i, { arr: n }); }} multiline={false} style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-text-primary)" }} />
          <EditableText value={row.renewalTiming} onChange={(v) => updateRow(i, { renewalTiming: v })} multiline={false} style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--color-text-secondary)" }} />
          <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, fontWeight: 600, textTransform: "uppercase", color: row.riskLevel === "at-risk" ? "var(--color-spice-terracotta)" : "var(--color-spice-saffron)" }}>
            {row.riskLevel}
          </span>
          <EditableText value={row.primaryRiskDriver} onChange={(v) => updateRow(i, { primaryRiskDriver: v })} multiline={false} placeholder="Risk driver..." style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-secondary)" }} />
          {content.riskAccounts.length > 1 && (
            <button onClick={() => removeRow(i)} style={{ opacity: hoveredRow === i ? 0.6 : 0, transition: "opacity 0.15s", background: "none", border: "none", cursor: "pointer", fontSize: 14, color: "var(--color-text-tertiary)" }} aria-label="Remove">
              ✕
            </button>
          )}
        </div>
      ))}

      {content.riskAccounts.length === 0 && (
        <div style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--color-text-tertiary)", padding: "20px 0" }}>
          No at-risk accounts identified.
        </div>
      )}

      <button onClick={addRow} style={{ fontFamily: "var(--font-mono)", fontSize: 11, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.06em", color: "var(--color-spice-turmeric)", background: "none", border: "none", cursor: "pointer", padding: "12px 0", textAlign: "left" }}>
        + Add Row
      </button>
    </section>
  );
}
