/**
 * SentimentHero — elevated journal entry at the top of Health & Outlook.
 *
 * User sentiment is authoritative, per account-detail-content-design.md. When
 * unset: single-line editorial prompt. When set: value chip + meta + pull-quote
 * note + divergence acknowledgment if computed health disagrees.
 *
 * Consumes DOS-27's data path — `userHealthSentiment` and `sentimentSetAt` are
 * existing fields on the account detail. Sparkline and note editing land with
 * the full DOS-27 round-trip; this component is read + update entry point.
 *
 * DOS-203
 */
import type { EntityIntelligence } from "@/types";
import { formatRelativeDate } from "@/lib/utils";

interface SentimentHeroProps {
  /** User's most recent sentiment assessment (Strong / On Track / Concerning / At Risk / Critical). */
  userHealthSentiment?: string | null;
  /** When the sentiment was last set. */
  sentimentSetAt?: string | null;
  /** Computed health block for divergence acknowledgment. */
  intelligenceHealth?: EntityIntelligence["health"];
  /** Called when the user clicks "Update" or the unset prompt. */
  onUpdate?: () => void;
}

function labelFor(value: string | null | undefined): { text: string; color: string } {
  switch ((value ?? "").toLowerCase()) {
    case "strong":
      return { text: "Strong", color: "var(--color-garden-sage)" };
    case "on_track":
    case "ontrack":
    case "on track":
      return { text: "On Track", color: "var(--color-garden-sage)" };
    case "concerning":
      return { text: "Concerning", color: "var(--color-spice-saffron)" };
    case "at_risk":
    case "atrisk":
    case "at risk":
      return { text: "At Risk", color: "var(--color-spice-terracotta)" };
    case "critical":
      return { text: "Critical", color: "var(--color-spice-terracotta)" };
    default:
      return { text: value ?? "", color: "var(--color-text-primary)" };
  }
}

function bandLabel(band: string): string {
  if (band === "green") return "Healthy";
  if (band === "red") return "At Risk";
  return "Monitor";
}

/** Does user sentiment diverge materially from computed band? */
function computeDivergence(userSentiment: string | null | undefined, band: string | undefined): boolean {
  if (!userSentiment || !band) return false;
  const user = userSentiment.toLowerCase();
  const positiveUser = /strong|on.?track/.test(user);
  const negativeUser = /concerning|risk|critical/.test(user);
  if (positiveUser && band === "red") return true;
  if (negativeUser && band === "green") return true;
  return false;
}

export function SentimentHero({
  userHealthSentiment,
  sentimentSetAt,
  intelligenceHealth,
  onUpdate,
}: SentimentHeroProps) {
  const hasSentiment = !!userHealthSentiment;
  const label = labelFor(userHealthSentiment);
  const divergence = computeDivergence(userHealthSentiment, intelligenceHealth?.band);

  return (
    <section
      className="editorial-reveal"
      style={{
        padding: "48px 0 40px",
        borderBottom: "1px solid var(--color-rule-light)",
        marginBottom: 32,
      }}
    >
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 10,
          textTransform: "uppercase",
          letterSpacing: "0.14em",
          color: "var(--color-text-tertiary)",
          marginBottom: 20,
        }}
      >
        Your Assessment
      </div>

      {!hasSentiment ? (
        <button
          type="button"
          onClick={onUpdate}
          style={{
            background: "none",
            border: "none",
            padding: 0,
            cursor: onUpdate ? "pointer" : "default",
            fontFamily: "var(--font-serif)",
            fontSize: 24,
            fontStyle: "italic",
            lineHeight: 1.4,
            color: "var(--color-text-tertiary)",
            textAlign: "left",
          }}
        >
          How are you reading this account? <span style={{ color: "var(--color-spice-turmeric)", borderBottom: "1px solid var(--color-spice-turmeric)" }}>Set your take &rarr;</span>
        </button>
      ) : (
        <>
          <div style={{ display: "flex", alignItems: "center", gap: 16, flexWrap: "wrap" }}>
            <span
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 32,
                fontWeight: 400,
                color: label.color,
                letterSpacing: "-0.01em",
              }}
            >
              {label.text}
            </span>
            {onUpdate && (
              <button
                type="button"
                onClick={onUpdate}
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  fontWeight: 600,
                  textTransform: "uppercase",
                  letterSpacing: "0.08em",
                  color: "var(--color-text-tertiary)",
                  background: "none",
                  border: "1px solid var(--color-rule-heavy)",
                  borderRadius: 4,
                  padding: "3px 10px",
                  cursor: "pointer",
                }}
              >
                Update
              </button>
            )}
          </div>
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              color: "var(--color-text-tertiary)",
              marginTop: 10,
            }}
          >
            {sentimentSetAt ? `Set ${formatRelativeDate(sentimentSetAt)}` : "Set recently"}
          </div>

          {divergence && intelligenceHealth && (
            <div
              style={{
                marginTop: 24,
                padding: "14px 18px",
                background: "var(--color-spice-saffron-8, rgba(196,147,53,0.08))",
                borderLeft: "3px solid var(--color-spice-saffron)",
                fontFamily: "var(--font-sans)",
                fontSize: 13,
                lineHeight: 1.55,
                color: "var(--color-text-secondary)",
              }}
            >
              <strong style={{ color: "var(--color-text-primary)" }}>Updates currently disagree.</strong>
              {" "}
              Computed health is{" "}
              <strong style={{ color: intelligenceHealth.band === "green" ? "var(--color-garden-sage)" : intelligenceHealth.band === "red" ? "var(--color-spice-terracotta)" : "var(--color-spice-saffron)" }}>
                {bandLabel(intelligenceHealth.band)} {Math.round(intelligenceHealth.score)}
              </strong>
              {intelligenceHealth.trend?.direction ? ` — trending ${intelligenceHealth.trend.direction}. ` : ". "}
              Your read differs from the machine's. Noting it here trains the loop.
            </div>
          )}
        </>
      )}
    </section>
  );
}
