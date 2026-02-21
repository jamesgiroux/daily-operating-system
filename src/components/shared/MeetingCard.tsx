/**
 * MeetingCard — Shared meeting card with editorial treatment.
 *
 * Renders type-based accent bars, time/duration, entity bylines,
 * intelligence badges, temporal states. Used by BriefingMeetingCard
 * (composition) and WeekPage timeline (direct).
 */
import { Link } from "@tanstack/react-router";
import clsx from "clsx";
import { IntelligenceQualityBadge } from "@/components/entity/IntelligenceQualityBadge";
import s from "./MeetingCard.module.css";

export interface MeetingCardProps {
  id: string;
  title: string;
  displayTime?: string;
  duration?: string;
  meetingType: string;
  entityByline?: string;
  intelligenceQuality?: {
    level: string;
    hasNewSignals?: boolean;
    lastEnriched?: string;
  };
  hasPrep?: boolean;
  temporalState?: "upcoming" | "in-progress" | "past";
  onClick?: () => void;
  showNavigationHint?: boolean;
  titleExtra?: React.ReactNode;
  subtitleExtra?: React.ReactNode;
  children?: React.ReactNode;
  className?: string;
}

function getAccentClass(meetingType: string): string {
  switch (meetingType) {
    case "customer":
    case "qbr":
    case "partnership":
    case "external":
      return s.accentCustomer;
    case "personal":
      return s.accentPersonal;
    case "one_on_one":
      return s.accentLarkspur;
    case "internal":
    case "team_sync":
    case "all_hands":
      return s.accentInternal;
    default:
      return "";
  }
}

export function MeetingCard({
  id,
  title,
  displayTime,
  duration,
  meetingType,
  entityByline,
  intelligenceQuality,
  hasPrep,
  temporalState,
  onClick,
  showNavigationHint,
  titleExtra,
  subtitleExtra,
  children,
  className,
}: MeetingCardProps) {
  const isPast = temporalState === "past";
  const isActive = temporalState === "in-progress";
  const isClickable = !!onClick;
  const shouldNavigate = !onClick; // All meetings link to their briefing page

  const cardContent = (
    <div
      className={clsx(
        s.card,
        getAccentClass(meetingType),
        isActive && s.active,
        isPast && s.past,
        isClickable && s.clickable,
        shouldNavigate && s.navigable,
        isPast && showNavigationHint && s.pastNavigate,
        className,
      )}
      onClick={onClick}
    >
      {displayTime && (
        <div className={s.time}>
          {displayTime}
          {duration && <span className={s.duration}>{duration}</span>}
        </div>
      )}
      <div className={s.content}>
        <div className={s.titleRow}>
          <span className={s.title}>{title}</span>
          {isActive && <span className={s.nowPill}>NOW</span>}
          {titleExtra}
          {isPast && showNavigationHint && (
            <span className={s.pastArrow}>&rarr;</span>
          )}
        </div>
        <div className={s.subtitle}>
          {entityByline}
          {subtitleExtra}
          {intelligenceQuality ? (
            <IntelligenceQualityBadge quality={intelligenceQuality as { level: "sparse" | "developing" | "ready" | "fresh"; hasNewSignals: boolean; lastEnriched?: string }} />
          ) : hasPrep ? (
            <span className={s.prepDot} title="Prep available" />
          ) : null}
        </div>
        {children}
      </div>
    </div>
  );

  if (shouldNavigate) {
    return (
      <Link
        to="/meeting/$meetingId"
        params={{ meetingId: id }}
        style={{ textDecoration: "none", color: "inherit" }}
      >
        {cardContent}
      </Link>
    );
  }

  return cardContent;
}
