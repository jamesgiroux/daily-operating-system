/**
 * ChapterHeading — heavy rule + serif title.
 * Used at the top of each editorial chapter section.
 * No chapter number shown — just rule + title.
 *
 * Optional feedbackSlot renders inline feedback controls next to the title.
 */
import type { ReactNode } from "react";
import styles from "./ChapterHeading.module.css";

interface ChapterHeadingProps {
  title: string;
  epigraph?: string;
  /** Optional inline feedback controls rendered after the title */
  feedbackSlot?: ReactNode;
  /** Freshness strip rendered between title and epigraph. */
  freshness?: ReactNode;
  /** Render title in compact monospace uppercase ("reference weight") per mockup. */
  variant?: "primary" | "reference";
  /** Suppress the chapter-break HR rule — use on the first chapter of a view. */
  noRule?: boolean;
}

export function ChapterHeading({ title, epigraph, feedbackSlot, freshness, variant = "primary", noRule = false }: ChapterHeadingProps) {
  return (
    <div className={styles.heading}>
      {!noRule && <hr className={styles.rule} />}
      <div className={styles.titleRow}>
        {variant === "reference" ? (
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.14em",
              color: "var(--color-text-secondary)",
              margin: 0,
            }}
          >
            {title}
          </div>
        ) : (
          <h2 className={styles.title}>{title}</h2>
        )}
        {feedbackSlot ? <span className={styles.feedback}>{feedbackSlot}</span> : null}
      </div>
      {freshness}
      {epigraph ? <p className={styles.epigraph}>{epigraph}</p> : null}
    </div>
  );
}
