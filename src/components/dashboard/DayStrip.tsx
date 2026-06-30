import clsx from "clsx";
import styles from "./DayStrip.module.css";

export interface DayStripProps {
  selectedDate: Date;
  today: Date;
  onSelectDate: (date: Date) => void;
  windowDays?: number;
}

function startOfLocalDay(date: Date): Date {
  const next = new Date(date);
  next.setHours(0, 0, 0, 0);
  return next;
}

function addDays(date: Date, days: number): Date {
  const next = startOfLocalDay(date);
  next.setDate(next.getDate() + days);
  return next;
}

function dayDelta(left: Date, right: Date): number {
  const msPerDay = 24 * 60 * 60 * 1000;
  return Math.round((startOfLocalDay(left).getTime() - startOfLocalDay(right).getTime()) / msPerDay);
}

function formatFullDate(date: Date): string {
  return date.toLocaleDateString("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
  });
}

function formatShortDate(date: Date): string {
  return date.toLocaleDateString("en-US", {
    weekday: "short",
    month: "short",
    day: "numeric",
  });
}

function sideLabel(date: Date, today: Date): string {
  const delta = dayDelta(date, today);
  if (delta === -1) return "Yesterday";
  if (delta === 0) return "Today";
  if (delta === 1) return "Tomorrow";
  return formatShortDate(date);
}

function DayStripSide({
  date,
  disabled,
  direction,
  label,
  onSelectDate,
}: {
  date: Date;
  disabled: boolean;
  direction: "previous" | "next";
  label: string;
  onSelectDate: (date: Date) => void;
}) {
  const content = (
    <>
      {direction === "previous" ? (
        <span className={styles.DayStrip_direction} aria-hidden="true">
          &larr;
        </span>
      ) : null}
      <span className={styles.DayStrip_label}>{label}</span>
      {direction === "next" ? (
        <span className={styles.DayStrip_direction} aria-hidden="true">
          &rarr;
        </span>
      ) : null}
    </>
  );
  const className = clsx(
    styles.DayStrip_side,
    direction === "next" && styles.DayStrip_sideRight,
    disabled && styles.DayStrip_sideDisabled,
  );

  if (disabled) {
    return (
      <span className={className} aria-disabled="true" title={label}>
        {content}
      </span>
    );
  }

  return (
    <a
      className={className}
      href="#"
      title={label}
      onClick={(event) => {
        event.preventDefault();
        onSelectDate(date);
      }}
    >
      {content}
    </a>
  );
}

export function DayStrip({
  selectedDate,
  today,
  onSelectDate,
  windowDays = 7,
}: DayStripProps) {
  const current = startOfLocalDay(selectedDate);
  const liveDay = startOfLocalDay(today);
  const delta = dayDelta(current, liveDay);
  const isToday = delta === 0;
  const previousDate = addDays(current, -1);
  const nextDate = addDays(current, 1);
  const previousDisabled = delta <= -windowDays;
  const nextDisabled = delta >= windowDays;
  const currentLabel = isToday ? "Today" : formatFullDate(current);

  return (
    <nav
      className={styles.DayStrip_strip}
      aria-label="Briefing days"
      data-ds-tier="pattern"
      data-ds-name="DayStrip"
      data-ds-spec="patterns/DayStrip.md"
    >
      <DayStripSide
        date={previousDate}
        disabled={previousDisabled}
        direction="previous"
        label={sideLabel(previousDate, liveDay)}
        onSelectDate={onSelectDate}
      />
      <div
        className={styles.DayStrip_current}
        aria-current="date"
        aria-label={isToday ? `Today, ${formatFullDate(current)}` : formatFullDate(current)}
      >
        {isToday ? <span className={styles.DayStrip_mark} aria-hidden="true" /> : null}
        {currentLabel}
      </div>
      <DayStripSide
        date={nextDate}
        disabled={nextDisabled}
        direction="next"
        label={sideLabel(nextDate, liveDay)}
        onSelectDate={onSelectDate}
      />
    </nav>
  );
}
