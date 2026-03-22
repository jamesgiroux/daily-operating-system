/**
 * CoverSlide — Full-viewport hero with title, vitals strip, executive summary.
 * Slide 1 of the Book of Business presentation.
 * All vitals are editable for internal presentation correction.
 */
import { useState, useCallback } from "react";
import { EditableText } from "@/components/ui/EditableText";
import { formatArr } from "@/lib/utils";
import type { BookOfBusinessContent } from "@/types/reports";

interface CoverSlideProps {
  content: BookOfBusinessContent;
  isStale?: boolean;
  onRegenerate?: () => void;
  generating?: boolean;
  onUpdate: (content: BookOfBusinessContent) => void;
}

export function CoverSlide({ content, isStale, onRegenerate, generating, onUpdate }: CoverSlideProps) {
  return (
    <div
      className="report-surface-slide"
    >
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "var(--color-spice-turmeric)",
          marginBottom: 24,
        }}
      >
        {content.periodLabel || "Book of Business"}
      </div>

      {/* Title */}
      <h1
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 52,
          fontWeight: 400,
          lineHeight: 1.1,
          letterSpacing: "-0.02em",
          color: "var(--color-text-primary)",
          margin: "0 0 32px",
          maxWidth: 700,
        }}
      >
        Book of Business
      </h1>

      {/* Staleness banner */}
      {isStale && (
        <div
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 13,
            color: "var(--color-spice-saffron)",
            background: "color-mix(in srgb, var(--color-spice-saffron) 8%, transparent)",
            border: "1px solid color-mix(in srgb, var(--color-spice-saffron) 20%, transparent)",
            borderRadius: 4,
            padding: "8px 16px",
            marginBottom: 24,
            display: "flex",
            alignItems: "center",
            gap: 16,
          }}
        >
          Account data has changed since this review was generated.
          <button
            onClick={onRegenerate}
            disabled={generating}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              letterSpacing: "0.06em",
              textTransform: "uppercase",
              color: "var(--color-spice-saffron)",
              background: "none",
              border: "1px solid var(--color-spice-saffron)",
              borderRadius: 4,
              padding: "2px 10px",
              cursor: "pointer",
              marginLeft: "auto",
            }}
          >
            Regenerate
          </button>
        </div>
      )}

      {/* Vitals strip — all editable */}
      <div
        style={{
          display: "flex",
          gap: 48,
          marginBottom: 40,
          flexWrap: "wrap",
        }}
      >
        <EditableVitalStat
          label="Accounts"
          value={String(content.totalAccounts)}
          onChange={(v) => {
            const n = parseInt(v.replace(/[^0-9]/g, ""), 10);
            if (!isNaN(n)) onUpdate({ ...content, totalAccounts: n });
          }}
        />
        <EditableVitalStat
          label="Total ARR"
          value={`$${formatArr(content.totalArr)}`}
          onChange={(v) => {
            const n = parseFloat(v.replace(/[^0-9.]/g, ""));
            if (!isNaN(n)) onUpdate({ ...content, totalArr: n });
          }}
        />
        <EditableVitalStat
          label="At-Risk ARR"
          value={`$${formatArr(content.atRiskArr)}`}
          danger={content.atRiskArr > 0}
          onChange={(v) => {
            const n = parseFloat(v.replace(/[^0-9.]/g, ""));
            if (!isNaN(n)) onUpdate({ ...content, atRiskArr: n });
          }}
        />
        <EditableVitalStat
          label="Projected Churn"
          value={`$${formatArr(content.projectedChurn)}`}
          danger={content.projectedChurn > 0}
          small
          onChange={(v) => {
            const n = parseFloat(v.replace(/[^0-9.]/g, ""));
            if (!isNaN(n)) onUpdate({ ...content, projectedChurn: n });
          }}
        />
      </div>

      {/* Biggest risk / upside callouts */}
      {(content.biggestRisk || content.biggestUpside) && (
        <div style={{ display: "flex", gap: 48, marginBottom: 32 }}>
          {content.biggestRisk && (
            <div>
              <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--color-spice-terracotta)", marginBottom: 4 }}>
                Biggest Risk
              </div>
              <div style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
                {content.biggestRisk.accountName} — ${formatArr(content.biggestRisk.arr)}
              </div>
            </div>
          )}
          {content.biggestUpside && (
            <div>
              <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--color-garden-sage)", marginBottom: 4 }}>
                Biggest Upside
              </div>
              <div style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
                {content.biggestUpside.accountName} — ${formatArr(content.biggestUpside.arr)}
              </div>
            </div>
          )}
          {content.eltHelpRequired && (
            <div style={{ fontFamily: "var(--font-mono)", fontSize: 11, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.08em", color: "var(--color-spice-saffron)", alignSelf: "center", background: "color-mix(in srgb, var(--color-spice-saffron) 10%, transparent)", padding: "4px 12px", borderRadius: 4 }}>
              ELT Help Required
            </div>
          )}
        </div>
      )}

      {/* Executive summary */}
      <EditableText
        as="p"
        value={content.executiveSummary}
        onChange={(v) => onUpdate({ ...content, executiveSummary: v })}
        multiline
        placeholder="Executive summary..."
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 22,
          fontWeight: 400,
          lineHeight: 1.5,
          letterSpacing: "-0.01em",
          color: "var(--color-text-primary)",
          maxWidth: 800,
          margin: 0,
        }}
      />
    </div>
  );
}

function EditableVitalStat({
  label,
  value,
  danger,
  small,
  onChange,
}: {
  label: string;
  value: string;
  danger?: boolean;
  small?: boolean;
  onChange: (value: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);

  const startEdit = useCallback(() => {
    setDraft(value);
    setEditing(true);
  }, [value]);

  const commitEdit = useCallback(() => {
    setEditing(false);
    if (draft.trim() !== value) {
      onChange(draft.trim());
    }
  }, [draft, value, onChange]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      commitEdit();
    } else if (e.key === "Escape") {
      setEditing(false);
      setDraft(value);
    }
  }, [commitEdit, value]);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      <span
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 10,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.1em",
          color: "var(--color-text-tertiary)",
        }}
      >
        {label}
      </span>
      {editing ? (
        <input
          autoFocus
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commitEdit}
          onKeyDown={handleKeyDown}
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: small ? 22 : 28,
            fontWeight: 400,
            color: danger ? "var(--color-spice-terracotta)" : "var(--color-text-primary)",
            letterSpacing: "-0.02em",
            background: "none",
            border: "none",
            borderBottom: "1px solid var(--color-spice-turmeric)",
            outline: "none",
            padding: 0,
            width: "100%",
            minWidth: 60,
          }}
        />
      ) : (
        <span
          onClick={startEdit}
          title="Click to edit"
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: small ? 22 : 28,
            fontWeight: 400,
            color: danger ? "var(--color-spice-terracotta)" : "var(--color-text-primary)",
            letterSpacing: "-0.02em",
            cursor: "text",
            borderBottom: "1px solid transparent",
            transition: "border-color 0.15s",
          }}
          onMouseEnter={(e) => { (e.target as HTMLElement).style.borderBottomColor = "var(--color-rule-light)"; }}
          onMouseLeave={(e) => { (e.target as HTMLElement).style.borderBottomColor = "transparent"; }}
        >
          {value}
        </span>
      )}
    </div>
  );
}
