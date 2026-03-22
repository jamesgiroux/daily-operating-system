/**
 * AccountHealthCover — full-viewport hero slide for Account Health Review.
 * Slide 1: atmosphere turmeric, account name (non-editable), overall assessment subtitle.
 */
import { EditableText } from "@/components/ui/EditableText";
import type { AccountHealthContent } from "./types";

interface AccountHealthCoverProps {
  accountName: string;
  content: AccountHealthContent;
  onUpdate: (c: AccountHealthContent) => void;
}

export function AccountHealthCover({ accountName, content, onUpdate }: AccountHealthCoverProps) {
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
        Account Review
      </div>

      {/* Account name — not editable, comes from prop */}
      <h1
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 52,
          fontWeight: 400,
          lineHeight: 1.1,
          letterSpacing: "-0.02em",
          color: "var(--color-text-primary)",
          margin: "0 0 24px",
          maxWidth: 700,
        }}
      >
        {accountName || "\u00A0"}
      </h1>

      {/* Overall assessment — editable subtitle */}
      <EditableText
        as="p"
        value={content.overallAssessment}
        onChange={(v) => onUpdate({ ...content, overallAssessment: v })}
        multiline
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 40,
          fontWeight: 400,
          lineHeight: 1.25,
          letterSpacing: "-0.01em",
          color: "var(--color-text-primary)",
          maxWidth: 800,
          margin: "0 0 24px",
          opacity: 0.85,
        }}
      />

      {/* Health score narrative */}
      {content.healthScoreNarrative && (
        <EditableText
          as="p"
          value={content.healthScoreNarrative}
          onChange={(v) => onUpdate({ ...content, healthScoreNarrative: v })}
          multiline
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 18,
            lineHeight: 1.6,
            color: "var(--color-text-secondary)",
            maxWidth: 700,
            margin: "0 0 32px",
            opacity: 0.8,
          }}
        />
      )}

      {/* Date + CSM name */}
      <div
        style={{
          display: "flex",
          gap: 24,
          alignItems: "baseline",
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
          {new Date().toLocaleDateString("en-US", {
            year: "numeric",
            month: "long",
            day: "numeric",
          })}
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
            value={content.csmName ?? ""}
            onChange={(v) => onUpdate({ ...content, csmName: v })}
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
