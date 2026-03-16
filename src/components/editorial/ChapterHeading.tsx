/**
 * ChapterHeading — heavy rule + serif title.
 * Used at the top of each editorial chapter section.
 * No chapter number shown — just rule + title.
 *
 * I529: Optional feedbackSlot renders inline feedback controls next to the title.
 */
import type { ReactNode } from "react";

interface ChapterHeadingProps {
  title: string;
  epigraph?: string;
  /** I529: Optional inline feedback controls rendered after the title */
  feedbackSlot?: ReactNode;
}

export function ChapterHeading({ title, epigraph, feedbackSlot }: ChapterHeadingProps) {
  return (
    <div style={{ marginBottom: 32 }}>
      <hr
        style={{
          border: "none",
          borderTop: "1px solid var(--color-rule-heavy)",
          marginBottom: 16,
        }}
      />
      <h2
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 28,
          fontWeight: 400,
          lineHeight: 1.2,
          letterSpacing: "-0.01em",
          color: "var(--color-text-primary)",
          margin: 0,
          display: "flex",
          alignItems: "center",
          gap: 8,
        }}
      >
        {title}
        {feedbackSlot}
      </h2>
      {epigraph && (
        <p
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 17,
            fontStyle: "italic",
            fontWeight: 300,
            lineHeight: 1.55,
            color: "var(--color-text-tertiary)",
            marginTop: 16,
            marginBottom: 0,
            maxWidth: 540,
          }}
        >
          {epigraph}
        </p>
      )}
    </div>
  );
}
