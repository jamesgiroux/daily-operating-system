/**
 * StateBlock — colored label (uppercase mono) + items as prose paragraphs.
 * Used in State of Play chapter for "Working" and "Struggling" sections.
 * Mockup: no dots, prose paragraphs, label 10px with 0.1em spacing, sage/terracotta label color.
 */
interface StateBlockProps {
  label: string;
  items: string[];
  labelColor?: string;
}

export function StateBlock({ label, items, labelColor = "var(--color-text-tertiary)" }: StateBlockProps) {
  if (items.length === 0) return null;

  return (
    <div style={{ marginBottom: 32 }}>
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 10,
          fontWeight: 500,
          textTransform: "uppercase",
          letterSpacing: "0.1em",
          color: labelColor,
          marginBottom: 10,
        }}
      >
        {label}
      </div>
      {items.map((item, i) => (
        <p
          key={i}
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 15,
            lineHeight: 1.65,
            color: "var(--color-text-primary)",
            maxWidth: 620,
            margin: i < items.length - 1 ? "0 0 12px" : 0,
          }}
        >
          {item}
        </p>
      ))}
    </div>
  );
}
