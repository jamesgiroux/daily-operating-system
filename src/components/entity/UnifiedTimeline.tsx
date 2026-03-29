/**
 * UnifiedTimeline — The Record chapter.
 * Merges meetings + emails + captures chronologically.
 * Shows 10 items by default, expandable. Vertical line timeline.
 * Generalized: accepts TimelineSource instead of AccountDetail.
 */
import { useState } from "react";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { TimelineEntry, TimelineContainer, type TimelineEntryType } from "@/components/editorial/TimelineEntry";
import { formatShortDate, formatMeetingType } from "@/lib/utils";
import type { TimelineSource } from "@/lib/entity-types";
import s from "./UnifiedTimeline.module.css";

interface UnifiedTimelineProps {
  data: TimelineSource;
  sectionId?: string;
  chapterTitle?: string;
  emptyMessage?: string;
  /** Slot rendered between heading and timeline list (e.g., AddToRecord). */
  actionSlot?: React.ReactNode;
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

export function UnifiedTimeline({
  data,
  sectionId = "the-record",
  chapterTitle = "The Record",
  emptyMessage = "No meetings, emails, or captures recorded yet.",
  actionSlot,
}: UnifiedTimelineProps) {
  const [expanded, setExpanded] = useState(false);

  const items: TimelineItem[] = [];

  for (const m of data.recentMeetings) {
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

  if (data.recentEmailSignals) {
    for (const e of data.recentEmailSignals) {
      items.push({
        date: e.detectedAt ? formatShortDate(e.detectedAt) : "",
        sortDate: e.detectedAt ?? "",
        type: "email",
        title: e.signalText,
        subtitle: e.senderEmail,
      });
    }
  }

  if (data.recentCaptures) {
    for (const c of data.recentCaptures) {
      items.push({
        date: "",
        sortDate: "",
        type: "capture",
        title: c.content,
        subtitle: c.meetingTitle,
        ...(c.meetingId
          ? { linkTo: "/meeting/$meetingId", linkParams: { meetingId: c.meetingId } }
          : {}),
      });
    }
  }

  if (data.accountEvents) {
    for (const ev of data.accountEvents) {
      const label = ev.eventType
        .replace(/_/g, " ")
        .replace(/\b\w/g, (c) => c.toUpperCase());
      items.push({
        date: formatShortDate(ev.eventDate),
        sortDate: ev.eventDate,
        type: "event" as TimelineEntryType,
        title: `${label}${ev.arrImpact != null ? ` ($${(ev.arrImpact / 1000).toFixed(0)}k)` : ""}`,
        subtitle: ev.notes || undefined,
      });
    }
  }

  if (data.lifecycleChanges) {
    for (const change of data.lifecycleChanges) {
      const transition = change.previousLifecycle
        ? `${change.previousLifecycle} → ${change.newLifecycle}`
        : change.newLifecycle;
      const subtitle = [
        change.newStage ? `Stage: ${change.newStage.replace(/_/g, " ")}` : null,
        change.evidence ?? null,
        `Source: ${change.source}`,
      ]
        .filter(Boolean)
        .join(" · ");
      items.push({
        date: formatShortDate(change.createdAt),
        sortDate: change.createdAt,
        type: "event" as TimelineEntryType,
        title: `Lifecycle: ${transition}`,
        subtitle,
      });
    }
  }

  if (data.contextEntries) {
    for (const entry of data.contextEntries) {
      items.push({
        date: formatShortDate(entry.createdAt),
        sortDate: entry.createdAt,
        type: "context",
        title: entry.title,
        subtitle: entry.content.length > 140 ? `${entry.content.slice(0, 140)}… · Added by you` : `${entry.content} · Added by you`,
      });
    }
  }

  if (data.autoCompletedMilestones) {
    for (const ms of data.autoCompletedMilestones) {
      const triggerLabel = ms.completionTrigger
        ? ms.completionTrigger.replace(/_/g, " ")
        : "lifecycle transition";
      items.push({
        date: ms.completedAt ? formatShortDate(ms.completedAt) : "",
        sortDate: ms.completedAt ?? "",
        type: "value" as TimelineEntryType,
        title: `Milestone completed: ${ms.title}`,
        subtitle: `Auto-completed by ${triggerLabel}`,
      });
    }
  }

  items.sort((a, b) => {
    if (!a.sortDate && !b.sortDate) return 0;
    if (!a.sortDate) return 1;
    if (!b.sortDate) return -1;
    return new Date(b.sortDate).getTime() - new Date(a.sortDate).getTime();
  });

  const visible = expanded ? items : items.slice(0, 10);
  const hasMore = items.length > 10;

  return (
    <section id={sectionId || undefined} style={{ scrollMarginTop: sectionId ? 60 : undefined }}>
      <ChapterHeading title={chapterTitle} />
      {actionSlot}

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
              className={s.toggleButton}
            >
              {expanded ? "Hide earlier history" : `Show full timeline (${items.length - 10} more)`}
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                className={`${s.chevron} ${expanded ? s.chevronExpanded : ""}`}
              >
                <polyline points="6 9 12 15 18 9" />
              </svg>
            </button>
          )}
        </>
      ) : (
        <p className={s.emptyMessage}>
          {emptyMessage}
        </p>
      )}
    </section>
  );
}
