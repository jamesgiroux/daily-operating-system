/**
 * CoverSlide — Weekly Impact cover.
 * Slide 1: week label, headline, meeting + actions closed stat.
 */
import { EditableText } from "@/components/ui/EditableText";
import type { WeeklyImpactContent } from "@/types/reports";

interface CoverSlideProps {
  content: WeeklyImpactContent;
  onUpdate: (updated: WeeklyImpactContent) => void;
}

export function CoverSlide({ content, onUpdate }: CoverSlideProps) {
  return (
    <section
      id="cover"
      className="report-surface-slide"
      style={{ scrollMarginTop: 60 }}
    >
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "var(--color-garden-eucalyptus)",
          marginBottom: 24,
        }}
      >
        Weekly Impact
      </div>

      {/* Week label — not editable, it's the date range */}
      <h1
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 48,
          fontWeight: 400,
          lineHeight: 1.1,
          letterSpacing: "-0.02em",
          color: "var(--color-text-primary)",
          margin: "0 0 16px",
        }}
      >
        {content.weekLabel}
      </h1>

      {/* Headline — editable */}
      <EditableText
        as="p"
        value={content.headline}
        onChange={(v) => onUpdate({ ...content, headline: v })}
        multiline
        placeholder="Add a headline for this week..."
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 22,
          fontStyle: "italic",
          fontWeight: 400,
          lineHeight: 1.5,
          color: "var(--color-text-secondary)",
          maxWidth: 700,
          margin: "0 0 32px",
        }}
      />

      {/* Stats row */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 14,
          color: "var(--color-text-tertiary)",
          letterSpacing: "0.04em",
        }}
      >
        {content.totalMeetings} meetings · {content.totalActionsClosed} actions closed
      </div>
    </section>
  );
}
