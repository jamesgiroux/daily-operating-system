/**
 * BriefingEmptyState — editorial cold-start frame when no data is connected.
 *
 * Left-aligned 640px column. Eyebrow + serif headline + italic lede +
 * optional checklist + optional primary CTA. Frames "what DailyOS needs"
 * rather than "the surface failed."
 *
 * Spec: .docs/design/patterns/BriefingEmptyState.md
 */

import clsx from "clsx";
import styles from "./BriefingEmptyState.module.css";

interface ChecklistItem {
  label: string;
  status?: "todo" | "done";
}

interface BriefingEmptyStateProps {
  eyebrow: string;
  headline: string;
  lede: string;
  checklistItems?: ChecklistItem[];
  cta?: { label: string; onClick: () => void };
}

export function BriefingEmptyState({
  eyebrow,
  headline,
  lede,
  checklistItems,
  cta,
}: BriefingEmptyStateProps): JSX.Element {
  return (
    <section
      className={styles.root}
      data-ds-name="BriefingEmptyState"
      data-ds-tier="pattern"
      data-ds-spec="patterns/BriefingEmptyState.md"
    >
      <p className={styles.eyebrow}>{eyebrow}</p>
      <h1 className={styles.headline}>{headline}</h1>
      <p className={styles.lede}>{lede}</p>
      {checklistItems && checklistItems.length > 0 && (
        <ul
          className={styles.checklist}
          data-ds-name="BriefingEmptyState.checklist"
        >
          {checklistItems.map((item, i) => (
            <li
              key={i}
              className={clsx(
                styles.checklistItem,
                item.status === "done" && styles.checklistItemDone,
              )}
            >
              <span className={styles.checklistGlyph} aria-hidden="true">
                {item.status === "done" ? "●" : "○"}
              </span>
              <span className={styles.checklistLabel}>{item.label}</span>
            </li>
          ))}
        </ul>
      )}
      {cta && (
        <button
          type="button"
          className={styles.cta}
          onClick={cta.onClick}
          data-ds-name="BriefingEmptyState.cta"
        >
          {cta.label}
        </button>
      )}
    </section>
  );
}
