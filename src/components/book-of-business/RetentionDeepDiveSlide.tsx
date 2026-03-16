/**
 * RetentionDeepDiveSlide — Top 2-3 at-risk account deep dives.
 * Slide 4: per-account risk narrative, save confidence, tactics, signals, help needed.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import { formatArr } from "@/lib/utils";
import type { BookOfBusinessContent, RetentionRiskDeepDive } from "@/types/reports";

interface RetentionDeepDiveSlideProps {
  content: BookOfBusinessContent;
  onUpdate: (content: BookOfBusinessContent) => void;
}

const CONFIDENCE_COLORS: Record<string, string> = {
  High: "var(--color-botanical-sage)",
  Medium: "var(--color-spice-saffron)",
  Low: "var(--color-spice-terracotta)",
};

export function RetentionDeepDiveSlide({ content, onUpdate }: RetentionDeepDiveSlideProps) {
  const [hoveredDive, setHoveredDive] = useState<number | null>(null);
  const [hoveredBullet, setHoveredBullet] = useState<{ dive: number; list: string; idx: number } | null>(null);

  const updateDive = (index: number, patch: Partial<RetentionRiskDeepDive>) => {
    const next = [...content.retentionRiskDeepDives];
    next[index] = { ...next[index], ...patch };
    onUpdate({ ...content, retentionRiskDeepDives: next });
  };

  const updateBulletList = (diveIndex: number, field: "keyTactics" | "successSignals" | "helpNeeded", bulletIndex: number, value: string) => {
    const next = [...content.retentionRiskDeepDives];
    const list = [...next[diveIndex][field]];
    list[bulletIndex] = value;
    next[diveIndex] = { ...next[diveIndex], [field]: list };
    onUpdate({ ...content, retentionRiskDeepDives: next });
  };

  const removeBullet = (diveIndex: number, field: "keyTactics" | "successSignals" | "helpNeeded", bulletIndex: number) => {
    const next = [...content.retentionRiskDeepDives];
    next[diveIndex] = { ...next[diveIndex], [field]: next[diveIndex][field].filter((_, j) => j !== bulletIndex) };
    onUpdate({ ...content, retentionRiskDeepDives: next });
  };

  const addBullet = (diveIndex: number, field: "keyTactics" | "successSignals" | "helpNeeded") => {
    const next = [...content.retentionRiskDeepDives];
    next[diveIndex] = { ...next[diveIndex], [field]: [...next[diveIndex][field], ""] };
    onUpdate({ ...content, retentionRiskDeepDives: next });
  };

  const removeDive = (index: number) => {
    onUpdate({ ...content, retentionRiskDeepDives: content.retentionRiskDeepDives.filter((_, i) => i !== index) });
  };

  const addDive = () => {
    onUpdate({
      ...content,
      retentionRiskDeepDives: [
        ...content.retentionRiskDeepDives,
        { accountName: "New Account", arr: 0, whyAtRisk: "", saveConfidence: "Medium", next90Days: "", keyTactics: [""], successSignals: [""], helpNeeded: [""] },
      ],
    });
  };

  const renderBulletList = (diveIndex: number, field: "keyTactics" | "successSignals" | "helpNeeded", label: string, items: string[]) => (
    <div style={{ flex: 1 }}>
      <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--color-text-tertiary)", marginBottom: 10 }}>
        {label}
      </div>
      {items.map((item, bi) => (
        <div
          key={bi}
          onMouseEnter={() => setHoveredBullet({ dive: diveIndex, list: field, idx: bi })}
          onMouseLeave={() => setHoveredBullet(null)}
          style={{ display: "flex", alignItems: "baseline", gap: 10, paddingBottom: 6 }}
        >
          <span style={{ width: 4, height: 4, borderRadius: "50%", background: "var(--color-spice-turmeric)", flexShrink: 0, marginTop: 8 }} />
          <EditableText
            value={item}
            onChange={(v) => updateBulletList(diveIndex, field, bi, v)}
            multiline={false}
            placeholder="Add detail..."
            style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)", flex: 1 }}
          />
          {items.length > 1 && (
            <button
              onClick={(e) => { e.stopPropagation(); removeBullet(diveIndex, field, bi); }}
              style={{ opacity: hoveredBullet?.dive === diveIndex && hoveredBullet?.list === field && hoveredBullet?.idx === bi ? 0.6 : 0, transition: "opacity 0.15s", background: "none", border: "none", cursor: "pointer", padding: "2px 4px", fontSize: 12, color: "var(--color-text-tertiary)", flexShrink: 0 }}
              aria-label="Remove"
            >
              ✕
            </button>
          )}
        </div>
      ))}
      <button
        onClick={() => addBullet(diveIndex, field)}
        style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.06em", color: "var(--color-spice-turmeric)", background: "none", border: "none", cursor: "pointer", padding: "6px 0", textAlign: "left" }}
      >
        + Add
      </button>
    </div>
  );

  return (
    <section
      id="retention-deep-dive"
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
      <div style={{ fontFamily: "var(--font-mono)", fontSize: 12, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.12em", color: "var(--color-spice-turmeric)", marginBottom: 32 }}>
        Retention Risk Deep Dives
      </div>

      {content.retentionRiskDeepDives.map((dive, di) => (
        <div
          key={di}
          onMouseEnter={() => setHoveredDive(di)}
          onMouseLeave={() => setHoveredDive(null)}
          style={{ marginBottom: 48, paddingBottom: 48, borderBottom: di < content.retentionRiskDeepDives.length - 1 ? "1px solid var(--color-rule-light)" : "none", maxWidth: 900 }}
        >
          {/* Header: Account name + ARR + remove */}
          <div style={{ display: "flex", alignItems: "baseline", gap: 16, marginBottom: 16 }}>
            <EditableText
              value={dive.accountName}
              onChange={(v) => updateDive(di, { accountName: v })}
              multiline={false}
              style={{ fontFamily: "var(--font-serif)", fontSize: 28, fontWeight: 400, color: "var(--color-text-primary)" }}
            />
            <span style={{ fontFamily: "var(--font-mono)", fontSize: 18, fontWeight: 600, color: "var(--color-spice-turmeric)" }}>
              ${formatArr(dive.arr)}
            </span>
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                fontWeight: 600,
                textTransform: "uppercase",
                letterSpacing: "0.06em",
                color: CONFIDENCE_COLORS[dive.saveConfidence] || "var(--color-text-tertiary)",
                background: "var(--color-rule-light)",
                padding: "2px 8px",
                borderRadius: 3,
              }}
            >
              {dive.saveConfidence} confidence
            </span>
            {content.retentionRiskDeepDives.length > 1 && (
              <button
                onClick={(e) => { e.stopPropagation(); removeDive(di); }}
                style={{ opacity: hoveredDive === di ? 0.6 : 0, transition: "opacity 0.15s", background: "none", border: "none", cursor: "pointer", padding: "4px 6px", fontSize: 14, color: "var(--color-text-tertiary)", flexShrink: 0, marginLeft: "auto" }}
                aria-label="Remove"
              >
                ✕
              </button>
            )}
          </div>

          {/* Why at risk */}
          <EditableText
            as="p"
            value={dive.whyAtRisk}
            onChange={(v) => updateDive(di, { whyAtRisk: v })}
            multiline
            placeholder="Why is this account at risk..."
            style={{ fontFamily: "var(--font-serif)", fontSize: 17, lineHeight: 1.5, color: "var(--color-text-primary)", maxWidth: 800, margin: "0 0 20px" }}
          />

          {/* Next 90 days */}
          <div style={{ borderLeft: "3px solid var(--color-spice-turmeric)", paddingLeft: 16, marginBottom: 24 }}>
            <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--color-text-tertiary)", marginBottom: 6 }}>
              Next 90 Days
            </div>
            <EditableText
              value={dive.next90Days}
              onChange={(v) => updateDive(di, { next90Days: v })}
              multiline={false}
              placeholder="90-day plan..."
              style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--color-text-primary)" }}
            />
          </div>

          {/* Bullet lists */}
          <div style={{ display: "flex", gap: 48 }}>
            {renderBulletList(di, "keyTactics", "Key Tactics", dive.keyTactics)}
            {renderBulletList(di, "successSignals", "Success Signals", dive.successSignals)}
            {renderBulletList(di, "helpNeeded", "Help Needed", dive.helpNeeded)}
          </div>
        </div>
      ))}

      {content.retentionRiskDeepDives.length === 0 && (
        <div style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--color-text-tertiary)", padding: "20px 0" }}>
          No retention risk deep dives.
        </div>
      )}

      <button
        onClick={addDive}
        style={{ fontFamily: "var(--font-mono)", fontSize: 11, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.06em", color: "var(--color-spice-turmeric)", background: "none", border: "none", cursor: "pointer", padding: "12px 0", textAlign: "left" }}
      >
        + Add Account
      </button>
    </section>
  );
}
