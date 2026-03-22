/**
 * ChapterHeading — heavy rule + serif title.
 * Used at the top of each editorial chapter section.
 * No chapter number shown — just rule + title.
 *
 * I529: Optional feedbackSlot renders inline feedback controls next to the title.
 */
import type { ReactNode } from "react";
import styles from "./ChapterHeading.module.css";

interface ChapterHeadingProps {
  title: string;
  epigraph?: string;
  /** I529: Optional inline feedback controls rendered after the title */
  feedbackSlot?: ReactNode;
}

export function ChapterHeading({ title, epigraph, feedbackSlot }: ChapterHeadingProps) {
  return (
    <div className={styles.heading}>
      <hr className={styles.rule} />
      <div className={styles.titleRow}>
        <h2 className={styles.title}>{title}</h2>
        {feedbackSlot ? <span className={styles.feedback}>{feedbackSlot}</span> : null}
      </div>
      {epigraph ? <p className={styles.epigraph}>{epigraph}</p> : null}
    </div>
  );
}
