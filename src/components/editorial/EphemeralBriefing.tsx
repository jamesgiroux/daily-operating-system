/**
 * EphemeralBriefing — editorial rendering of a one-shot account briefing.
 *
 * I495: Renders the result of an ephemeral Glean query in the app's
 * magazine aesthetic. Newsreader serif heading, DM Sans body, section
 * rules between sections. No system vocabulary in the UI (ADR-0083).
 */
import type { EphemeralBriefing as EphemeralBriefingType } from "@/types";

interface EphemeralBriefingProps {
  briefing: EphemeralBriefingType;
  /** Called when user wants to add this account to their book. */
  onAdd?: () => void;
  /** Called when user wants to navigate to the existing account. */
  onNavigate?: (entityId: string) => void;
}

export function EphemeralBriefing({
  briefing,
  onAdd,
  onNavigate,
}: EphemeralBriefingProps) {
  return (
    <article
      style={{
        padding: "32px 0",
      }}
    >
      {/* Heading */}
      <h3
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 24,
          fontWeight: 400,
          lineHeight: 1.2,
          letterSpacing: "-0.01em",
          color: "var(--color-text-primary)",
          margin: "0 0 8px 0",
        }}
      >
        {briefing.name}
      </h3>

      {/* Source count */}
      {briefing.sourceCount > 0 && (
        <p
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            fontWeight: 500,
            letterSpacing: "0.06em",
            textTransform: "uppercase",
            color: "var(--color-text-tertiary)",
            margin: "0 0 20px 0",
          }}
        >
          {briefing.sourceCount} source{briefing.sourceCount !== 1 ? "s" : ""}
        </p>
      )}

      {/* Already exists banner */}
      {briefing.alreadyExists && (
        <div
          style={{
            padding: "12px 16px",
            background: "var(--color-spice-turmeric-8)",
            borderLeft: "3px solid var(--color-spice-turmeric)",
            borderRadius: 2,
            marginBottom: 20,
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <span
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              color: "var(--color-text-secondary)",
            }}
          >
            This account is already in your book.
          </span>
          {onNavigate && (
            <button
              onClick={() => onNavigate(briefing.alreadyExists!)}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                fontWeight: 600,
                letterSpacing: "0.06em",
                textTransform: "uppercase",
                color: "var(--color-spice-turmeric)",
                background: "none",
                border: "none",
                cursor: "pointer",
                padding: 0,
              }}
            >
              View account
            </button>
          )}
        </div>
      )}

      {/* Summary */}
      {briefing.summary && (
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 15,
            lineHeight: 1.65,
            color: "var(--color-text-secondary)",
            margin: "0 0 24px 0",
            whiteSpace: "pre-line",
          }}
        >
          {briefing.summary}
        </p>
      )}

      {/* Sections */}
      {briefing.sections.map((section, i) => (
        <div key={i}>
          {/* Section rule */}
          <hr
            style={{
              border: "none",
              borderTop: "1px solid var(--color-rule-light)",
              margin: "20px 0 16px 0",
            }}
          />
          <div style={{ display: "flex", alignItems: "baseline", gap: 12, marginBottom: 8 }}>
            <h4
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 17,
                fontWeight: 400,
                lineHeight: 1.3,
                color: "var(--color-text-primary)",
                margin: 0,
              }}
            >
              {section.title}
            </h4>
            {section.source && (
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  fontWeight: 500,
                  letterSpacing: "0.06em",
                  textTransform: "uppercase",
                  color: "var(--color-text-tertiary)",
                }}
              >
                {section.source}
              </span>
            )}
          </div>
          <p
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              lineHeight: 1.65,
              color: "var(--color-text-secondary)",
              margin: 0,
              whiteSpace: "pre-line",
            }}
          >
            {section.content}
          </p>
        </div>
      ))}

      {/* Add to book button — only show if not already in DailyOS */}
      {!briefing.alreadyExists && onAdd && (
        <>
          <hr
            style={{
              border: "none",
              borderTop: "1px solid var(--color-rule-light)",
              margin: "24px 0 20px 0",
            }}
          />
          <button
            onClick={onAdd}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              letterSpacing: "0.06em",
              textTransform: "uppercase",
              color: "var(--color-spice-turmeric)",
              background: "none",
              border: "1px solid var(--color-spice-turmeric)",
              borderRadius: 4,
              padding: "6px 16px",
              cursor: "pointer",
            }}
          >
            Add to my accounts
          </button>
        </>
      )}
    </article>
  );
}
