/**
 * DayStrip.tsx - Daily Briefing day-to-day navigation (DOS-420, W1)
 *
 * Fixed briefing-day strip below FolioBar. It renders the contract verbatim:
 * previous day, current day, next day.
 *
 * Spec: .docs/design/patterns/DayStrip.md
 * Contract: src/types/briefing.ts -> DayStripViewModel + DayStripNeighbor
 */

import clsx from "clsx";
import type { DayStripNeighbor, DayStripViewModel } from "@/types/briefing";
import styles from "./DayStrip.module.css";

function renderPrevNeighbor(prev: DayStripNeighbor): JSX.Element {
  return (
    <a
      className={styles.DayStrip_side}
      href={prev.href}
      title={prev.label}
      data-iso-date={prev.isoDate}
    >
      <span className={styles.DayStrip_direction} aria-hidden="true">
        &larr;
      </span>
      <span className={styles.DayStrip_label}>{prev.label}</span>
      <span className={styles.DayStrip_preview}>{prev.preview}</span>
    </a>
  );
}

function renderNextNeighbor(next: DayStripNeighbor): JSX.Element {
  return (
    <a
      className={clsx(styles.DayStrip_side, styles.DayStrip_sideRight)}
      href={next.href}
      title={next.label}
      data-iso-date={next.isoDate}
    >
      <span className={styles.DayStrip_preview}>{next.preview}</span>
      <span className={styles.DayStrip_label}>{next.label}</span>
      <span className={styles.DayStrip_direction} aria-hidden="true">
        &rarr;
      </span>
    </a>
  );
}

export function DayStrip({ prev, current, next }: DayStripViewModel): JSX.Element {
  return (
    <nav
      className={styles.DayStrip_strip}
      aria-label="Briefing days"
      data-ds-name="DayStrip"
      data-ds-tier="pattern"
      data-ds-spec="patterns/DayStrip.md"
    >
      {renderPrevNeighbor(prev)}
      <time
        className={styles.DayStrip_current}
        dateTime={current.isoDate}
        data-iso-date={current.isoDate}
        aria-current="date"
        aria-label={current.ariaLabel}
      >
        <span className={styles.DayStrip_mark} aria-hidden="true" />
        {current.label}
      </time>
      {renderNextNeighbor(next)}
    </nav>
  );
}
