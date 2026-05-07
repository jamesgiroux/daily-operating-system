/**
 * DayStrip.tsx - Daily Briefing day-to-day navigation
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
      className={styles.side}
      href={prev.href}
      title={prev.label}
      data-iso-date={prev.isoDate}
    >
      <span className={styles.direction} aria-hidden="true">
        &larr;
      </span>
      <span className={styles.label}>{prev.label}</span>
      <span className={styles.preview}>{prev.preview}</span>
    </a>
  );
}

function renderNextNeighbor(next: DayStripNeighbor): JSX.Element {
  return (
    <a
      className={clsx(styles.side, styles.sideRight)}
      href={next.href}
      title={next.label}
      data-iso-date={next.isoDate}
    >
      <span className={styles.preview}>{next.preview}</span>
      <span className={styles.label}>{next.label}</span>
      <span className={styles.direction} aria-hidden="true">
        &rarr;
      </span>
    </a>
  );
}

export function DayStrip({ prev, current, next }: DayStripViewModel): JSX.Element {
  return (
    <nav
      className={styles.root}
      aria-label="Briefing days"
      data-ds-name="DayStrip"
      data-ds-tier="pattern"
      data-ds-spec="patterns/DayStrip.md"
    >
      {renderPrevNeighbor(prev)}
      <time
        className={styles.current}
        dateTime={current.isoDate}
        data-iso-date={current.isoDate}
        aria-current="date"
        aria-label={current.ariaLabel}
      >
        <span className={styles.mark} aria-hidden="true" />
        {current.label}
      </time>
      {renderNextNeighbor(next)}
    </nav>
  );
}
