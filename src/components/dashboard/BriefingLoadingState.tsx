/**
 * BriefingLoadingState — editorial holding state while the briefing assembles.
 *
 * Centered single-column stage. Eyebrow + serif headline + optional pulsing
 * dot. Copy is always passed in by the consuming surface.
 *
 * Spec: .docs/design/patterns/BriefingLoadingState.md
 */

import clsx from "clsx";
import styles from "./BriefingLoadingState.module.css";

interface BriefingLoadingStateProps {
  headline: string;
  eyebrow: string;
  withPulse?: boolean;
}

export function BriefingLoadingState({
  headline,
  eyebrow,
  withPulse = true,
}: BriefingLoadingStateProps): JSX.Element {
  return (
    <section
      className={styles.root}
      role="status"
      aria-live="polite"
      data-ds-name="BriefingLoadingState"
      data-ds-tier="pattern"
      data-ds-spec="patterns/BriefingLoadingState.md"
    >
      <p className={styles.headline}>{headline}</p>
      {withPulse && (
        <span
          className={clsx(styles.pulse)}
          aria-hidden="true"
          data-ds-name="BriefingLoadingState.pulse"
        />
      )}
      <p className={styles.eyebrow}>{eyebrow}</p>
    </section>
  );
}
