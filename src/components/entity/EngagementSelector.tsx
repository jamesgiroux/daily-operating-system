/**
 * EngagementSelector — Clickable badge that opens a dropdown to change
 * stakeholder engagement level. Replaces the static engagement badge in
 * StakeholderGallery.
 *
 * Display labels are activity-based for clarity (ADR-0076 color system).
 * Stored values stay backward-compatible with existing intelligence.json.
 */
import { useState, useRef, useEffect } from "react";

interface EngagementSelectorProps {
  value: string;
  onChange: (value: string) => void;
}

interface EngagementOption {
  stored: string;
  label: string;
  background: string;
  color: string;
}

const ENGAGEMENT_OPTIONS: EngagementOption[] = [
  { stored: "champion", label: "Champion", background: "rgba(201, 162, 39, 0.12)", color: "var(--color-spice-turmeric)" },
  { stored: "executive_sponsor", label: "Exec Sponsor", background: "rgba(74, 103, 65, 0.14)", color: "var(--color-garden-rosemary)" },
  { stored: "decision_maker", label: "Decision Maker", background: "rgba(74, 103, 65, 0.14)", color: "var(--color-garden-rosemary)" },
  { stored: "primary_contact", label: "Primary Contact", background: "rgba(143, 163, 196, 0.14)", color: "var(--color-garden-larkspur)" },
  { stored: "technical_contact", label: "Technical Contact", background: "rgba(143, 163, 196, 0.14)", color: "var(--color-garden-larkspur)" },
  { stored: "power_user", label: "Power User", background: "rgba(143, 163, 196, 0.14)", color: "var(--color-garden-larkspur)" },
  { stored: "end_user", label: "End User", background: "rgba(143, 163, 196, 0.08)", color: "var(--color-text-tertiary)" },
];

/** Map a stored engagement value to its display configuration. */
export function getEngagementDisplay(stored: string): EngagementOption {
  const lower = stored.toLowerCase();
  return ENGAGEMENT_OPTIONS.find((o) => o.stored === lower) ?? {
    stored: lower,
    label: stored,
    background: "rgba(143, 163, 196, 0.14)",
    color: "var(--color-text-tertiary)",
  };
}

export function EngagementSelector({ value, onChange }: EngagementSelectorProps) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const display = getEngagementDisplay(value);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    function handleClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [open]);

  return (
    <div ref={ref} style={{ position: "relative", display: "inline-block" }}>
      <button
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          setOpen(!open);
        }}
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 9,
          fontWeight: 500,
          textTransform: "uppercase",
          letterSpacing: "0.08em",
          padding: "2px 7px",
          borderRadius: 3,
          border: "none",
          cursor: "pointer",
          background: display.background,
          color: display.color,
        }}
      >
        {display.label}
      </button>

      {open && (
        <div
          style={{
            position: "absolute",
            top: "calc(100% + 4px)",
            left: 0,
            zIndex: 50,
            background: "var(--color-paper-cream)",
            border: "1px solid var(--color-rule-light)",
            borderRadius: 6,
            boxShadow: "0 4px 12px rgba(0,0,0,0.08)",
            padding: "4px 0",
            minWidth: 140,
          }}
        >
          {ENGAGEMENT_OPTIONS.map((opt) => (
            <button
              key={opt.stored}
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                onChange(opt.stored);
                setOpen(false);
              }}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                width: "100%",
                padding: "6px 12px",
                border: "none",
                background: opt.stored === value.toLowerCase() ? "rgba(0,0,0,0.04)" : "none",
                cursor: "pointer",
                textAlign: "left",
              }}
            >
              <span
                style={{
                  width: 8,
                  height: 8,
                  borderRadius: "50%",
                  background: opt.color,
                  flexShrink: 0,
                }}
              />
              <span
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 12,
                  color: "var(--color-text-primary)",
                }}
              >
                {opt.label}
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
