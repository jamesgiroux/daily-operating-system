/**
 * ChapterHeading — heavy rule + serif title.
 * Used at the top of each editorial chapter section.
 * No chapter number shown — just rule + title.
 */
interface ChapterHeadingProps {
  title: string;
  epigraph?: string;
}

export function ChapterHeading({ title, epigraph }: ChapterHeadingProps) {
  return (
    <div style={{ marginBottom: 32 }}>
      <hr
        style={{
          border: "none",
          borderTop: "2px solid var(--color-desk-charcoal)",
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
        }}
      >
        {title}
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
