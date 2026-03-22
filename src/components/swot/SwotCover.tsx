/**
 * SwotCover — full-viewport hero slide for SWOT Analysis.
 * Slide 1: overline, account name (non-editable), optional summary pull quote,
 * date + prepared-by (local state only, not persisted to SwotContent).
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { SwotContent } from "@/types/reports";

interface SwotCoverProps {
  accountName: string;
  content: SwotContent;
  onUpdate: (c: SwotContent) => void;
  generatedAt?: string;
}

export function SwotCover({ accountName, content, onUpdate, generatedAt }: SwotCoverProps) {
  // tamName is a presentation annotation — not persisted to SwotContent
  const [tamName, setTamName] = useState("");

  const dateLabel = generatedAt
    ? new Date(generatedAt).toLocaleDateString("en-US", {
        year: "numeric",
        month: "long",
        day: "numeric",
      })
    : new Date().toLocaleDateString("en-US", {
        year: "numeric",
        month: "long",
        day: "numeric",
      });

  return (
    <div
      className="report-surface-slide"
    >
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "var(--color-garden-sage)",
          marginBottom: 24,
        }}
      >
        SWOT Analysis
      </div>

      {/* Account name — not editable */}
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
        {accountName || "\u00A0"}
      </h1>

      {/* Summary pull quote — shown only when present */}
      {content.summary && (
        <>
          <div
            style={{
              width: 40,
              height: 1,
              background: "var(--color-garden-sage)",
              marginBottom: 16,
            }}
          />
          <EditableText
            as="p"
            value={content.summary}
            onChange={(v) => onUpdate({ ...content, summary: v || null })}
            multiline
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 20,
              fontStyle: "italic",
              lineHeight: 1.7,
              color: "var(--color-text-primary)",
              maxWidth: 700,
              margin: "0 0 40px",
            }}
          />
        </>
      )}

      {/* Date + Prepared by */}
      <div
        style={{
          display: "flex",
          gap: 24,
          alignItems: "baseline",
          marginTop: content.summary ? 0 : 8,
        }}
      >
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            color: "var(--color-text-tertiary)",
            letterSpacing: "0.04em",
          }}
        >
          {dateLabel}
        </span>
        <span
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            color: "var(--color-text-secondary)",
            display: "inline-flex",
            alignItems: "baseline",
            gap: 4,
          }}
        >
          Prepared by{" "}
          <EditableText
            value={tamName}
            onChange={(v) => setTamName(v)}
            multiline={false}
            placeholder="Add name"
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              color: "var(--color-text-secondary)",
            }}
          />
        </span>
      </div>
    </div>
  );
}
