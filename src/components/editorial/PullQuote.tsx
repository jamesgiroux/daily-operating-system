/**
 * PullQuote — centered italic serif with thin centered rules above/below.
 * Mockup: 72px vertical padding, 120px wide centered rules, serif 28px italic.
 */
interface PullQuoteProps {
  text: string;
  attribution?: string;
}

export function PullQuote({ text, attribution }: PullQuoteProps) {
  return (
    <blockquote
      style={{
        margin: 0,
        padding: "72px 0",
        textAlign: "center",
        position: "relative",
      }}
    >
      {/* Top rule — centered 120px */}
      <div
        aria-hidden="true"
        style={{
          width: 120,
          height: 1,
          background: "var(--color-rule-heavy)",
          margin: "0 auto 48px",
        }}
      />
      <p
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 28,
          fontWeight: 400,
          fontStyle: "italic",
          lineHeight: 1.45,
          color: "var(--color-text-primary)",
          maxWidth: 580,
          margin: "0 auto",
        }}
      >
        {text}
      </p>
      {attribution && (
        <cite
          style={{
            display: "block",
            marginTop: 16,
            fontFamily: "var(--font-sans)",
            fontSize: 12,
            fontStyle: "normal",
            color: "var(--color-text-tertiary)",
          }}
        >
          — {attribution}
        </cite>
      )}
      {/* Bottom rule — centered 120px */}
      <div
        aria-hidden="true"
        style={{
          width: 120,
          height: 1,
          background: "var(--color-rule-heavy)",
          margin: "48px auto 0",
        }}
      />
    </blockquote>
  );
}
