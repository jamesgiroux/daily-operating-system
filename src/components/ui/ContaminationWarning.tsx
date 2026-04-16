/**
 * DOS-83: Visual warning for cross-entity contaminated intelligence fields.
 *
 * Renders an inline warning banner when intelligence text is flagged as
 * potentially describing a different entity (cross-entity bleed).
 */
import { AlertTriangle } from "lucide-react";

interface ContaminationWarningProps {
  /** Compact inline badge mode vs. full banner. */
  variant?: "banner" | "badge";
  className?: string;
}

const bannerStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "flex-start",
  gap: "8px",
  padding: "8px 12px",
  borderRadius: "6px",
  background: "var(--color-spice-saffron, #e8a317)10",
  border: "1px solid var(--color-spice-saffron, #e8a317)30",
  fontSize: "12px",
  lineHeight: "1.4",
  color: "var(--color-text-secondary, #6b6b6b)",
  marginBottom: "8px",
};

const badgeStyle: React.CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: "4px",
  padding: "2px 8px",
  borderRadius: "4px",
  background: "var(--color-spice-saffron, #e8a317)18",
  fontSize: "11px",
  lineHeight: "1.3",
  color: "var(--color-spice-saffron, #b8860b)",
  fontWeight: 500,
};

const iconStyle: React.CSSProperties = {
  flexShrink: 0,
  color: "var(--color-spice-saffron, #e8a317)",
  marginTop: "1px",
};

export function ContaminationWarning({
  variant = "banner",
  className,
}: ContaminationWarningProps) {
  if (variant === "badge") {
    return (
      <span style={badgeStyle} className={className} title="This content may reference a different entity">
        <AlertTriangle size={11} strokeWidth={2} style={iconStyle} />
        Cross-entity suspect
      </span>
    );
  }

  return (
    <div style={bannerStyle} className={className}>
      <AlertTriangle size={14} strokeWidth={2} style={iconStyle} />
      <span>
        This intelligence may reference a different entity. It has been flagged
        for review and will be corrected on the next enrichment cycle.
      </span>
    </div>
  );
}
