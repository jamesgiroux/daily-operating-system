import { useSlideActive, anim } from "../hooks";
import type { MonthlyWrappedContent } from "../types";
import styles from "../monthly-wrapped.module.css";

export function MomentsSlide({ content }: { content: MonthlyWrappedContent }) {
  const active = useSlideActive("moments");

  const a0 = anim(styles, active, "slideUp");

  return (
    <section id="moments" className={`${styles.slide} ${styles.bgWarmWhite}`}>
      <div
        className={`${styles.overline} ${styles.momentsOverline} ${a0.className}`}
        style={a0.style}
      >
        The moments.
      </div>

      <div className={styles.momentsList}>
        {content.moments.map((moment, i) => (
          <div
            key={i}
            className={`${i < content.moments.length - 1 ? styles.momentItem : ""} ${anim(styles, active, "slideUp", `${i * 200}ms`).className}`}
            style={anim(styles, active, "slideUp", `${i * 200}ms`).style}
          >
            <div className={styles.momentLabel}>
              {moment.label}
            </div>
            <div className={styles.momentHeadline}>
              {moment.headline}
            </div>
            {moment.subtext && (
              <div className={styles.momentSubtext}>
                {moment.subtext}
              </div>
            )}
          </div>
        ))}
      </div>
    </section>
  );
}
