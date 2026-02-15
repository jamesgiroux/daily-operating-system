/**
 * TrajectoryChapter — Project-specific editorial chapter.
 * Replaces the shared StateOfPlay for projects. Shows trajectory confidence,
 * momentum/headwinds via StateBlock, velocity indicators, and remaining assessment prose.
 */
import type { ProjectDetail, EntityIntelligence } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { PullQuote } from "@/components/editorial/PullQuote";
import { StateBlock } from "@/components/editorial/StateBlock";

interface TrajectoryChapterProps {
  detail: ProjectDetail;
  intelligence: EntityIntelligence | null;
}

/* ── Velocity Strip ── */

function VelocityStrip({ detail }: { detail: ProjectDetail }) {
  const signals = detail.signals;
  if (!signals) return null;

  const items: string[] = [];

  if (signals.meetingFrequency30d != null) {
    const trendArrow =
      signals.meetingFrequency90d != null && signals.meetingFrequency90d > 0
        ? signals.meetingFrequency30d > signals.meetingFrequency90d / 3
          ? " \u2191"
          : signals.meetingFrequency30d < signals.meetingFrequency90d / 3
            ? " \u2193"
            : ""
        : "";
    items.push(`${signals.meetingFrequency30d} meetings / 30d${trendArrow}`);
  }

  if (signals.openActionCount != null) {
    items.push(`${signals.openActionCount} open actions`);
  }

  if (signals.daysUntilTarget != null) {
    items.push(`${signals.daysUntilTarget}d to target`);
  }

  if (items.length === 0) return null;

  return (
    <div
      style={{
        fontFamily: "var(--font-mono)",
        fontSize: 11,
        color: "var(--color-text-tertiary)",
        marginTop: 32,
        marginBottom: 32,
      }}
    >
      {items.map((item, i) => (
        <span key={i}>
          {i > 0 && (
            <span style={{ margin: "0 10px", opacity: 0.5 }}>{"\u00b7"}</span>
          )}
          <span
            style={{
              color: item.includes("\u2191")
                ? "var(--color-garden-olive)"
                : item.includes("\u2193")
                  ? "var(--color-spice-terracotta)"
                  : undefined,
            }}
          >
            {item}
          </span>
        </span>
      ))}
    </div>
  );
}

/* ── Main component ── */

export function TrajectoryChapter({ detail, intelligence }: TrajectoryChapterProps) {
  const working = intelligence?.currentState?.working ?? [];
  const notWorking = intelligence?.currentState?.notWorking ?? [];

  const paragraphs =
    intelligence?.executiveAssessment?.split("\n").filter((p) => p.trim()) ?? [];
  const pullQuote = paragraphs.length > 1 ? paragraphs[1] : null;
  const remainingParagraphs = paragraphs.slice(2);

  const hasContent = working.length > 0 || notWorking.length > 0 || paragraphs.length > 0;

  return (
    <section id="trajectory" style={{ scrollMarginTop: 60, paddingTop: 80 }}>
      <ChapterHeading title="Trajectory" />

      {hasContent ? (
        <>
          {pullQuote && <PullQuote text={pullQuote} />}

          <StateBlock
            label="Momentum"
            items={working}
            labelColor="var(--color-garden-olive)"
          />
          <StateBlock
            label="Headwinds"
            items={notWorking}
            labelColor="var(--color-spice-terracotta)"
          />

          <VelocityStrip detail={detail} />

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
                    margin: i < remainingParagraphs.length - 1 ? "0 0 12px" : 0,
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
          Build intelligence to reveal this project's trajectory.
        </p>
      )}
    </section>
  );
}
