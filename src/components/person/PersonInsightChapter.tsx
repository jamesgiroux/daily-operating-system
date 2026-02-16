/**
 * PersonInsightChapter — Adaptive chapter that changes editorial framing
 * based on person relationship type (internal vs external).
 *
 * External/Unknown → "The Dynamic" (relationship health framing)
 * Internal → "The Rhythm" (collaboration patterns framing)
 */
import type { PersonDetail, EntityIntelligence } from "@/types";
import { formatShortDate } from "@/lib/utils";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { PullQuote } from "@/components/editorial/PullQuote";
import { StateBlock } from "@/components/editorial/StateBlock";

interface PersonInsightChapterProps {
  detail: PersonDetail;
  intelligence: EntityIntelligence | null;
  /** When provided, state items become editable. Called with (fieldPath, newValue). */
  onUpdateField?: (fieldPath: string, value: string) => void;
}

const ADAPTATION = {
  external: {
    title: "The Dynamic",
    sectionId: "the-dynamic",
    workingLabel: "Relationship Strengths",
    notWorkingLabel: "Relationship Gaps",
    emptyMessage: "Build intelligence to understand this relationship.",
  },
  internal: {
    title: "The Rhythm",
    sectionId: "the-rhythm",
    workingLabel: "Collaboration Strengths",
    notWorkingLabel: "Alignment Gaps",
    emptyMessage: "Build intelligence to understand how you collaborate.",
  },
} as const;

function getAdaptation(relationship: string) {
  return relationship === "internal" ? ADAPTATION.internal : ADAPTATION.external;
}

/* ── CadenceStrip sub-component ── */

function CadenceStrip({ detail }: { detail: PersonDetail }) {
  const sig = detail.signals;
  if (!sig) return null;

  const parts: string[] = [];

  const trendArrow =
    sig.trend === "increasing" ? " \u2191" : sig.trend === "decreasing" ? " \u2193" : "";
  const trendColor =
    sig.trend === "increasing"
      ? "var(--color-garden-olive)"
      : sig.trend === "decreasing"
        ? "var(--color-spice-terracotta)"
        : undefined;

  parts.push(`${sig.meetingFrequency30d} meetings / 30d${trendArrow}`);

  if (sig.temperature) {
    parts.push(sig.temperature);
  }

  if (sig.lastMeeting) {
    parts.push(`last met ${formatShortDate(sig.lastMeeting)}`);
  }

  return (
    <div
      style={{
        fontFamily: "var(--font-mono)",
        fontSize: 11,
        color: "var(--color-text-tertiary)",
        marginTop: 24,
        marginBottom: 8,
      }}
    >
      {parts.map((part, i) => (
        <span key={i}>
          {i > 0 && (
            <span style={{ margin: "0 8px", opacity: 0.4 }}>&middot;</span>
          )}
          <span style={i === 0 && trendColor ? { color: trendColor } : undefined}>
            {part}
          </span>
        </span>
      ))}
    </div>
  );
}

/* ── Main component ── */

export function PersonInsightChapter({ detail, intelligence, onUpdateField }: PersonInsightChapterProps) {
  const adapt = getAdaptation(detail.relationship);

  const working = intelligence?.currentState?.working ?? [];
  const notWorking = intelligence?.currentState?.notWorking ?? [];
  const paragraphs = intelligence?.executiveAssessment?.split("\n").filter((p) => p.trim()) ?? [];
  const pullQuote = paragraphs.length > 1 ? paragraphs[1] : null;
  const remainingParagraphs = paragraphs.slice(2);

  const hasContent = working.length > 0 || notWorking.length > 0;

  return (
    <section id={adapt.sectionId} style={{ scrollMarginTop: 60, paddingTop: 80 }}>
      <ChapterHeading title={adapt.title} />

      {hasContent ? (
        <>
          <StateBlock
            label={adapt.workingLabel}
            items={working}
            labelColor="var(--color-garden-sage)"
            onItemChange={
              onUpdateField
                ? (index, value) => onUpdateField(`currentState.working[${index}]`, value)
                : undefined
            }
          />
          <StateBlock
            label={adapt.notWorkingLabel}
            items={notWorking}
            labelColor="var(--color-spice-terracotta)"
            onItemChange={
              onUpdateField
                ? (index, value) => onUpdateField(`currentState.notWorking[${index}]`, value)
                : undefined
            }
          />
          {pullQuote && <PullQuote text={pullQuote} />}
          <CadenceStrip detail={detail} />
          {remainingParagraphs.length > 0 && (
            <div style={{ marginTop: 24 }}>
              {remainingParagraphs.map((p, i) => (
                <p
                  key={i}
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 15,
                    lineHeight: 1.65,
                    color: "var(--color-text-primary)",
                    maxWidth: 620,
                    margin: i < remainingParagraphs.length - 1 ? "0 0 16px" : 0,
                  }}
                >
                  {p}
                </p>
              ))}
            </div>
          )}
        </>
      ) : (
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            color: "var(--color-text-tertiary)",
            fontStyle: "italic",
          }}
        >
          {adapt.emptyMessage}
        </p>
      )}
    </section>
  );
}
