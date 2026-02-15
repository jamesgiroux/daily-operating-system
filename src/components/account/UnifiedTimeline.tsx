/**
 * UnifiedTimeline — Chapter 5: The Record.
 * Merges meetings + emails + captures chronologically.
 * Shows 10 items by default, expandable. Vertical line timeline.
 */
import { useState } from "react";
import type { AccountDetail } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { TimelineEntry, TimelineContainer, type TimelineEntryType } from "@/components/editorial/TimelineEntry";
import { formatShortDate, formatMeetingType } from "@/lib/utils";

interface UnifiedTimelineProps {
  detail: AccountDetail;
}

interface TimelineItem {
  date: string;
  sortDate: string;
  type: TimelineEntryType;
  title: string;
  subtitle?: string;
  linkTo?: string;
  linkParams?: Record<string, string>;
}

export function UnifiedTimeline({ detail }: UnifiedTimelineProps) {
  const [expanded, setExpanded] = useState(false);

  // Build unified timeline
  const items: TimelineItem[] = [];

  // Recent meetings
  for (const m of detail.recentMeetings) {
    items.push({
      date: formatShortDate(m.startTime),
      sortDate: m.startTime,
      type: "meeting",
      title: m.title,
      subtitle: formatMeetingType(m.meetingType),
      linkTo: "/meeting/$meetingId",
      linkParams: { meetingId: m.id },
    });
  }

  // Email signals
  if (detail.recentEmailSignals) {
    for (const e of detail.recentEmailSignals) {
      items.push({
        date: e.detectedAt ? formatShortDate(e.detectedAt) : "",
        sortDate: e.detectedAt ?? "",
        type: "email",
        title: e.signalText,
        subtitle: e.senderEmail,
      });
    }
  }

  // Captures
  for (const c of detail.recentCaptures) {
    items.push({
      date: "",
      sortDate: "",
      type: "capture",
      title: c.content,
      subtitle: c.meetingTitle,
      linkTo: "/meeting/$meetingId",
      linkParams: { meetingId: c.meetingId },
    });
  }

  // Sort by date descending
  items.sort((a, b) => {
    if (!a.sortDate && !b.sortDate) return 0;
    if (!a.sortDate) return 1;
    if (!b.sortDate) return -1;
    return new Date(b.sortDate).getTime() - new Date(a.sortDate).getTime();
  });

  const visible = expanded ? items : items.slice(0, 10);
  const hasMore = items.length > 10;

  return (
    <section id="the-record" style={{ scrollMarginTop: 60, paddingTop: 80 }}>
      <ChapterHeading title="The Record" />

      {items.length > 0 ? (
        <>
          <TimelineContainer>
            {visible.map((item, i) => (
              <TimelineEntry
                key={`${item.type}-${i}`}
                date={item.date}
                type={item.type}
                title={item.title}
                subtitle={item.subtitle}
                linkTo={item.linkTo}
                linkParams={item.linkParams}
              />
            ))}
          </TimelineContainer>

          {hasMore && (
            <button
              onClick={() => setExpanded(!expanded)}
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 6,
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                fontWeight: 500,
                textTransform: "uppercase",
                letterSpacing: "0.06em",
                color: "var(--color-spice-turmeric)",
                cursor: "pointer",
                padding: "8px 0",
                marginTop: 12,
                border: "none",
                background: "none",
              }}
            >
              {expanded ? "Hide earlier history" : `Show full timeline (${items.length - 10} more)`}
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                style={{
                  width: 14,
                  height: 14,
                  transform: expanded ? "rotate(180deg)" : "none",
                  transition: "transform 0.3s ease",
                }}
              >
                <polyline points="6 9 12 15 18 9" />
              </svg>
            </button>
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
          No meetings, emails, or captures recorded yet.
        </p>
      )}
    </section>
  );
}
