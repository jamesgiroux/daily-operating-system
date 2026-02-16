/**
 * WatchItem — type-colored dot + source badge + description.
 * Used in the Watch List chapter for risks, wins, and unknowns.
 */
interface WatchItemProps {
  type: "risk" | "win" | "unknown";
  text: string;
  source?: string;
  detail?: string;
}

const dotColors: Record<WatchItemProps["type"], string> = {
  risk: "var(--color-spice-terracotta)",
  win: "var(--color-garden-sage)",
  unknown: "var(--color-spice-turmeric)",
};

const typeLabels: Record<WatchItemProps["type"], string> = {
  risk: "Risk",
  win: "Win",
  unknown: "Unknown",
};

export function WatchItem({ type, text, source, detail }: WatchItemProps) {
  return (
    <div
      style={{
        display: "flex",
        gap: 12,
        alignItems: "flex-start",
        paddingBottom: 16,
        marginBottom: 16,
        borderBottom: "1px solid var(--color-rule-light)",
      }}
    >
      <span
        style={{
          width: 8,
          height: 8,
          borderRadius: "50%",
          background: dotColors[type] ?? "var(--color-text-tertiary)",
          flexShrink: 0,
          marginTop: 6,
        }}
      />
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.06em",
              color: dotColors[type],
            }}
          >
            {typeLabels[type]}
          </span>
          {source && (
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                color: "var(--color-text-tertiary)",
              }}
            >
              {source}
            </span>
          )}
        </div>
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            lineHeight: 1.5,
            color: "var(--color-text-primary)",
            margin: "4px 0 0",
          }}
        >
          {text}
        </p>
        {detail && (
          <p
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              lineHeight: 1.5,
              color: "var(--color-text-secondary)",
              margin: "4px 0 0",
            }}
          >
            {detail}
          </p>
        )}
      </div>
    </div>
  );
}
