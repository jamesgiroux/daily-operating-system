/**
 * EngagementSelector — Clickable badge that opens a dropdown to change
 * stakeholder engagement level. I652: options are now actual engagement
 * levels (strong_advocate, engaged, neutral, disengaged, unknown), not
 * roles. Roles are handled separately via multi-role badges.
 *
 * Display labels are activity-based for clarity (ADR-0076 color system).
 * Stored values stay backward-compatible with existing DB columns.
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
  { stored: "strong_advocate", label: "Strong Advocate", background: "var(--color-spice-turmeric-12)", color: "var(--color-spice-turmeric)" },
  { stored: "engaged", label: "Engaged", background: "var(--color-garden-rosemary-14)", color: "var(--color-garden-rosemary)" },
  { stored: "neutral", label: "Neutral", background: "var(--color-garden-larkspur-14)", color: "var(--color-garden-larkspur)" },
  { stored: "disengaged", label: "Disengaged", background: "var(--color-text-tertiary-8)", color: "var(--color-text-tertiary)" },
  { stored: "unknown", label: "Unknown", background: "var(--color-text-tertiary-8)", color: "var(--color-text-tertiary)" },
];

/** Map a stored engagement value to its display configuration. */
export function getEngagementDisplay(stored: string): EngagementOption {
  const lower = stored.toLowerCase();
  return ENGAGEMENT_OPTIONS.find((o) => o.stored === lower) ?? {
    stored: lower,
    label: stored,
    background: "var(--color-garden-larkspur-14)",
    color: "var(--color-text-tertiary)",
  };
}

/** Get a human-readable label for an engagement value. */
export function getEngagementLabel(stored: string): string {
  return getEngagementDisplay(stored).label;
}

/** Get a CSS class suffix for an engagement value (used by StakeholderGallery). */
export function getEngagementClass(stored: string): string {
  const lower = stored.toLowerCase();
  if (lower === "strong_advocate") return "engagementStrongAdvocate";
  if (lower === "engaged") return "engagementEngaged";
  if (lower === "neutral") return "engagementNeutral";
  if (lower === "disengaged") return "engagementDisengaged";
  return "engagementUnknown";
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
            boxShadow: "var(--shadow-lg)",
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
                background: opt.stored === value.toLowerCase() ? "var(--color-black-4)" : "none",
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
