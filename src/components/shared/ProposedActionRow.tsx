/**
 * Shared ProposedActionRow — renders a suggested action with accept/reject.
 *
 * Consolidates duplicate implementations from:
 * - ActionsPage.tsx (full: "Suggested" label, priority, context)
 * - DailyBriefing.tsx (compact: smaller, no label/priority)
 *
 * ADR-0084 C2.
 */

interface ProposedActionRowProps {
  action: {
    id: string;
    title: string;
    priority?: string;
    sourceLabel?: string | null;
    accountName?: string | null;
    accountId?: string | null;
  };
  onAccept: () => void;
  onReject: () => void;
  showBorder?: boolean;
  compact?: boolean;
  stripMarkdown?: (s: string) => string;
}

function defaultStripMarkdown(s: string): string {
  return s.replace(/\*\*/g, "").replace(/\[([^\]]+)\]\([^)]+\)/g, "$1");
}

export function ProposedActionRow({
  action,
  onAccept,
  onReject,
  showBorder = true,
  compact = false,
  stripMarkdown = defaultStripMarkdown,
}: ProposedActionRowProps) {
  const btnSize = compact ? 24 : 28;
  const svgSize = compact ? 12 : 14;

  const contextParts: string[] = [];
  if (!compact) {
    if (action.sourceLabel) contextParts.push(action.sourceLabel);
    if (action.accountName || action.accountId) {
      contextParts.push((action.accountName || action.accountId)!);
    }
  }

  return (
    <div
      style={{
        display: "flex",
        alignItems: compact ? "center" : "flex-start",
        gap: compact ? 10 : 12,
        padding: compact ? "8px 0" : "14px 0",
        borderBottom: showBorder ? "1px solid var(--color-rule-light)" : "none",
        borderLeft: "2px dashed var(--color-spice-turmeric)",
        paddingLeft: compact ? 12 : 16,
      }}
    >
      <div style={{ flex: 1, minWidth: 0 }}>
        {!compact && (
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 2 }}>
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                fontWeight: 600,
                letterSpacing: "0.06em",
                textTransform: "uppercase",
                color: "var(--color-spice-turmeric)",
              }}
            >
              Suggested
            </span>
            {action.priority && (
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  fontWeight: 600,
                  letterSpacing: "0.04em",
                  color:
                    action.priority === "P1"
                      ? "var(--color-spice-terracotta)"
                      : action.priority === "P2"
                        ? "var(--color-spice-turmeric)"
                        : "var(--color-text-tertiary)",
                }}
              >
                {action.priority}
              </span>
            )}
          </div>
        )}
        <div
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: compact ? 15 : 17,
            fontWeight: 400,
            color: "var(--color-text-primary)",
            lineHeight: 1.4,
          }}
        >
          {stripMarkdown(action.title)}
        </div>
        {compact && action.sourceLabel && (
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              color: "var(--color-text-tertiary)",
              marginTop: 1,
            }}
          >
            {action.sourceLabel}
          </div>
        )}
        {!compact && contextParts.length > 0 && (
          <div
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              fontWeight: 300,
              color: "var(--color-text-tertiary)",
              marginTop: 2,
            }}
          >
            {contextParts.join(" \u00B7 ")}
          </div>
        )}
      </div>

      <div style={{ display: "flex", gap: compact ? 4 : 6, flexShrink: 0, marginTop: compact ? 0 : 4 }}>
        <button
          onClick={onAccept}
          title="Accept"
          style={{
            width: btnSize,
            height: btnSize,
            borderRadius: 4,
            border: "1px solid var(--color-garden-sage)",
            background: "transparent",
            cursor: "pointer",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            padding: 0,
          }}
        >
          <svg width={svgSize} height={svgSize} viewBox={`0 0 ${svgSize} ${svgSize}`} fill="none">
            <path
              d={compact ? "M2.5 6L5 8.5L9.5 4" : "M3 7L6 10L11 4"}
              stroke="var(--color-garden-sage)"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </button>
        <button
          onClick={onReject}
          title={compact ? "Dismiss" : "Reject"}
          style={{
            width: btnSize,
            height: btnSize,
            borderRadius: 4,
            border: "1px solid var(--color-spice-terracotta)",
            background: "transparent",
            cursor: "pointer",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            padding: 0,
          }}
        >
          <svg width={svgSize} height={svgSize} viewBox={`0 0 ${svgSize} ${svgSize}`} fill="none">
            <path
              d={compact ? "M3 3L9 9M9 3L3 9" : "M4 4L10 10M10 4L4 10"}
              stroke="var(--color-spice-terracotta)"
              strokeWidth="2"
              strokeLinecap="round"
            />
          </svg>
        </button>
      </div>
    </div>
  );
}
