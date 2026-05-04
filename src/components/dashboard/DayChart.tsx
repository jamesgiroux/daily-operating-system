import clsx from "clsx";
import type { ComponentPropsWithoutRef, CSSProperties } from "react";
import styles from "./DayChart.module.css";

export type DayChartMeetingType = "customer" | "customer.warn" | "internal" | "oo" | "cancel";
export type DayChartMeetingState = "past" | "now" | "upcoming" | "cancelled";

export interface DayChartMeeting {
  id: string;
  type: DayChartMeetingType;
  state?: DayChartMeetingState;
  startPct: number;
  durationPct: number;
  title: string;
  time: string;
  warn?: boolean;
  ariaLabel?: string;
}

export interface DayChartLegendItem {
  label: string;
  tone: "customer" | "warn" | "oo" | "internal";
}

export interface DayChartProps extends ComponentPropsWithoutRef<"section"> {
  hours?: string[];
  meetings: DayChartMeeting[];
  nowPosition?: number | null;
  nowLabel?: string;
  showLegend?: boolean;
  legendItems?: DayChartLegendItem[];
  chartHeight?: number;
  mutedHours?: string[];
  onMeetingClick?: (meeting: DayChartMeeting) => void;
}

const DEFAULT_HOURS = ["7 AM", "8", "9", "10", "11", "12 PM", "1", "2", "3", "4", "5"];

const DEFAULT_LEGEND: DayChartLegendItem[] = [
  { label: "Customer", tone: "customer" },
  { label: "At risk", tone: "warn" },
  { label: "1:1", tone: "oo" },
  { label: "Internal", tone: "internal" },
];

const LEGEND_SWATCH_CLASS: Record<DayChartLegendItem["tone"], string> = {
  customer: styles.swatchCustomer,
  warn: styles.swatchWarn,
  oo: styles.swatchOo,
  internal: styles.swatchInternal,
};

const MEETING_TYPE_CLASS: Record<DayChartMeetingType, string> = {
  customer: styles.customer,
  "customer.warn": styles.customer,
  internal: styles.internal,
  oo: styles.oo,
  cancel: styles.cancel,
};

function clampPct(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.min(100, Math.max(0, value));
}

function clampHeight(value: number): number {
  if (!Number.isFinite(value)) return 110;
  return Math.min(160, Math.max(70, value));
}

function barStyle(meeting: DayChartMeeting): CSSProperties {
  const left = clampPct(meeting.startPct);
  const width = Math.min(100 - left, Math.max(0, meeting.durationPct));
  return {
    left: `${left}%`,
    width: `${width}%`,
  };
}

export function DayChart({
  hours = DEFAULT_HOURS,
  meetings,
  nowPosition = null,
  nowLabel = "NOW",
  showLegend = true,
  legendItems = DEFAULT_LEGEND,
  chartHeight = 110,
  mutedHours = ["12 PM"],
  onMeetingClick,
  className,
  "aria-label": ariaLabel = "Shape of the day",
  ...rest
}: DayChartProps) {
  const chartStyle = {
    "--day-chart-height": `${clampHeight(chartHeight)}px`,
    "--day-chart-hour-count": hours.length,
    "--day-chart-grid-column-count": Math.max(1, hours.length - 1),
  } as CSSProperties;
  const hasNow = typeof nowPosition === "number" && Number.isFinite(nowPosition);

  return (
    <section
      className={clsx(styles.dayChart, className)}
      style={chartStyle}
      aria-label={ariaLabel}
      data-ds-name="DayChart"
      data-ds-spec="patterns/DayChart.md"
      {...rest}
    >
      {showLegend ? (
        <div className={styles.legend} aria-label="Meeting type legend">
          {legendItems.map((item) => (
            <span className={styles.legendItem} key={`${item.tone}-${item.label}`}>
              <span className={clsx(styles.swatch, LEGEND_SWATCH_CLASS[item.tone])} aria-hidden="true" />
              {item.label}
            </span>
          ))}
        </div>
      ) : null}

      <div className={styles.hourTicks} aria-hidden="true">
        {hours.map((hour) => (
          <span
            className={clsx(styles.hourTick, mutedHours.includes(hour) && styles.hourTickMuted)}
            key={hour}
          >
            {hour}
          </span>
        ))}
      </div>

      <div className={styles.bars} role="list" aria-label="Meetings plotted across the workday">
        {meetings.map((meeting) => {
          const isNow = meeting.state === "now";
          const isPast = meeting.state === "past";
          const isCancelled = meeting.state === "cancelled" || meeting.type === "cancel";
          const isWarn = meeting.warn || meeting.type === "customer.warn";
          const classes = clsx(
            styles.bar,
            MEETING_TYPE_CLASS[meeting.type],
            isWarn && styles.warn,
            isPast && styles.past,
            isNow && styles.nowBar,
            isCancelled && styles.cancel,
            onMeetingClick && styles.barInteractive,
          );

          const content = (
            <>
              <span className={styles.barTitle}>{meeting.title}</span>
              <span className={styles.barTime}>{meeting.time}</span>
            </>
          );

          if (onMeetingClick) {
            return (
              <button
                type="button"
                className={classes}
                style={barStyle(meeting)}
                key={meeting.id}
                role="listitem"
                aria-label={meeting.ariaLabel ?? `${meeting.title}, ${meeting.time}`}
                onClick={() => onMeetingClick(meeting)}
              >
                {content}
              </button>
            );
          }

          return (
            <div
              className={classes}
              style={barStyle(meeting)}
              key={meeting.id}
              role="listitem"
              aria-label={meeting.ariaLabel ?? `${meeting.title}, ${meeting.time}`}
            >
              {content}
            </div>
          );
        })}

        {hasNow ? (
          <span
            className={styles.nowLine}
            style={{ left: `${clampPct(nowPosition)}%` }}
            data-now-label={nowLabel}
            aria-hidden="true"
          />
        ) : null}
      </div>
    </section>
  );
}
