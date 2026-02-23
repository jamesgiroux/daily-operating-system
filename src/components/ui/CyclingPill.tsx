/**
 * CyclingPill — Click to cycle through a set of options.
 * Editorial pill style: mono text, border, colored dot indicator.
 */

interface CyclingPillProps {
  options: string[];
  value: string;
  onChange: (value: string) => void;
  /** Map option value to a CSS color for the dot indicator */
  colorMap?: Record<string, string>;
  /** Placeholder text when value is empty */
  placeholder?: string;
}

export function CyclingPill({
  options,
  value,
  onChange,
  colorMap,
  placeholder = "Not set",
}: CyclingPillProps) {
  const handleClick = () => {
    const currentIdx = options.indexOf(value);
    const nextIdx = (currentIdx + 1) % options.length;
    onChange(options[nextIdx]);
  };

  const display = value
    ? value.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase())
    : placeholder;

  const dotColor = value && colorMap ? colorMap[value] : undefined;

  return (
    <button
      type="button"
      onClick={handleClick}
      title="Click to cycle"
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 6,
        fontFamily: "var(--font-mono)",
        fontSize: 10,
        fontWeight: 600,
        textTransform: "uppercase",
        letterSpacing: "0.06em",
        color: value ? "var(--color-text-secondary)" : "var(--color-text-tertiary)",
        background: "none",
        border: "1px solid var(--color-rule-light)",
        borderRadius: 3,
        padding: "3px 10px",
        cursor: "pointer",
        whiteSpace: "nowrap",
      }}
    >
      {dotColor && (
        <span
          style={{
            width: 6,
            height: 6,
            borderRadius: "50%",
            background: dotColor,
            flexShrink: 0,
          }}
        />
      )}
      {display}
    </button>
  );
}
