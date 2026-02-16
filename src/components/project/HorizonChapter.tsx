/**
 * HorizonChapter — Forward-looking editorial chapter for projects.
 * Next milestone, target date reality, decisions pending, meeting readiness.
 * No account equivalent — project-specific.
 */
import type { ProjectDetail, EntityIntelligence } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { StateBlock } from "@/components/editorial/StateBlock";
import { formatShortDate } from "@/lib/utils";

interface HorizonChapterProps {
  detail: ProjectDetail;
  intelligence: EntityIntelligence | null;
  /** When provided, items become editable. Called with (fieldPath, newValue). */
  onUpdateField?: (fieldPath: string, value: string) => void;
}

/** Find a risk mentioning timeline/deadline/schedule/target/delay. */
function findTimelineRisk(intelligence: EntityIntelligence | null): string | null {
  if (!intelligence?.risks?.length) return null;
  const pattern = /timeline|deadline|schedule|target|delay/i;
  const match = intelligence.risks.find((r) => pattern.test(r.text));
  return match?.text ?? null;
}

/** Days between now and a date string. Negative = overdue. */
function daysUntil(dateStr: string): number | null {
  const target = new Date(dateStr);
  if (isNaN(target.getTime())) return null;
  const now = new Date();
  const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const startOfTarget = new Date(target.getFullYear(), target.getMonth(), target.getDate());
  return Math.round((startOfTarget.getTime() - startOfToday.getTime()) / (1000 * 60 * 60 * 24));
}

export function HorizonChapter({ detail, intelligence, onUpdateField }: HorizonChapterProps) {
  const nextMilestone = detail.milestones.find(
    (m) => m.status.toLowerCase() !== "completed" && m.status.toLowerCase() !== "done",
  );
  const daysToTarget = detail.signals?.daysUntilTarget;
  const unknowns = intelligence?.currentState?.unknowns ?? [];
  const readiness = intelligence?.nextMeetingReadiness;
  const timelineRisk = findTimelineRisk(intelligence);

  const hasContent =
    nextMilestone != null ||
    daysToTarget != null ||
    unknowns.length > 0 ||
    (readiness && readiness.prepItems.length > 0);

  return (
    <section id="the-horizon" style={{ scrollMarginTop: 60, paddingTop: 80 }}>
      <ChapterHeading title="The Horizon" />

      {hasContent ? (
        <>
          {/* Next Milestone */}
          {nextMilestone && (
            <div
              style={{
                background: "var(--color-paper-linen)",
                borderLeft: "3px solid var(--color-garden-olive)",
                borderRadius: "0 8px 8px 0",
                padding: "24px 28px",
                marginBottom: 40,
              }}
            >
              <div
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  fontWeight: 500,
                  textTransform: "uppercase",
                  letterSpacing: "0.1em",
                  color: "var(--color-garden-olive)",
                  marginBottom: 10,
                }}
              >
                Next Milestone
              </div>
              <div
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 15,
                  fontWeight: 500,
                  color: "var(--color-text-primary)",
                  marginBottom: 6,
                }}
              >
                {nextMilestone.name}
              </div>
              {nextMilestone.targetDate && (
                <div style={{ display: "flex", gap: 16, alignItems: "baseline" }}>
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      color: "var(--color-text-tertiary)",
                    }}
                  >
                    {formatShortDate(nextMilestone.targetDate)}
                  </span>
                  {(() => {
                    const days = daysUntil(nextMilestone.targetDate);
                    if (days == null) return null;
                    return (
                      <span
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 10,
                          color:
                            days < 0
                              ? "var(--color-spice-terracotta)"
                              : "var(--color-text-tertiary)",
                        }}
                      >
                        {days < 0 ? `${Math.abs(days)}d overdue` : `${days}d away`}
                      </span>
                    );
                  })()}
                </div>
              )}
            </div>
          )}

          {/* Target Date Reality */}
          {daysToTarget != null && (
            <div style={{ marginBottom: 40 }}>
              <div
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 24,
                  fontWeight: 500,
                  color:
                    daysToTarget <= 0
                      ? "var(--color-spice-terracotta)"
                      : "var(--color-garden-olive)",
                  marginBottom: 12,
                }}
              >
                {daysToTarget <= 0
                  ? `${Math.abs(daysToTarget)} days overdue`
                  : `${daysToTarget} days to target`}
              </div>
              {timelineRisk && (
                <p
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 15,
                    lineHeight: 1.65,
                    color: "var(--color-text-primary)",
                    maxWidth: 620,
                    margin: 0,
                  }}
                >
                  {timelineRisk}
                </p>
              )}
            </div>
          )}

          {/* Decisions Pending */}
          <StateBlock
            label="Decisions Pending"
            items={unknowns}
            labelColor="var(--color-garden-larkspur)"
            onItemChange={
              onUpdateField
                ? (index, value) => onUpdateField(`currentState.unknowns[${index}]`, value)
                : undefined
            }
          />

          {/* Meeting Readiness */}
          {readiness && readiness.prepItems.length > 0 && (
            <div
              style={{
                background: "var(--color-paper-linen)",
                borderLeft: "3px solid var(--color-garden-olive)",
                borderRadius: "0 8px 8px 0",
                padding: "24px 28px",
                marginTop: 32,
              }}
            >
              <div
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  fontWeight: 500,
                  textTransform: "uppercase",
                  letterSpacing: "0.1em",
                  color: "var(--color-garden-olive)",
                  marginBottom: 10,
                }}
              >
                Meeting Readiness
                {readiness.meetingTitle && (
                  <span
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 10,
                      fontWeight: 500,
                      textTransform: "none",
                      letterSpacing: "normal",
                      color: "var(--color-text-primary)",
                      marginLeft: 8,
                    }}
                  >
                    {readiness.meetingTitle}
                  </span>
                )}
                {readiness.meetingDate && (
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      fontWeight: 500,
                      textTransform: "uppercase",
                      letterSpacing: "0.1em",
                      color: "var(--color-text-tertiary)",
                      marginLeft: 8,
                    }}
                  >
                    {formatShortDate(readiness.meetingDate)}
                  </span>
                )}
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
                {readiness.prepItems.map((item, i) => (
                  <p
                    key={i}
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 15,
                      lineHeight: 1.65,
                      color: "var(--color-text-primary)",
                      margin: 0,
                    }}
                  >
                    {item}
                  </p>
                ))}
              </div>
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
          Build intelligence and set milestones to populate The Horizon.
        </p>
      )}
    </section>
  );
}
