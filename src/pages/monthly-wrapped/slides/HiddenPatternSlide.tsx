import { useSlideActive, anim } from "../hooks";
import type { MonthlyWrappedContent } from "../types";
import styles from "../monthly-wrapped.module.css";

export function HiddenPatternSlide({ content }: { content: MonthlyWrappedContent }) {
  const active = useSlideActive("hidden-pattern");

  const a0 = anim(styles, active, "slideUp");
  const a1 = anim(styles, active, "slideUpLong", "200ms");

  return (
    <section id="hidden-pattern" className={`${styles.slide} ${styles.bgTerracotta}`}>
      <div
        className={`${styles.overline} ${styles.hiddenPatternOverline} ${a0.className}`}
        style={a0.style}
      >
        Something you might have missed.
      </div>

      <p
        className={`${styles.serifQuote} ${styles.hiddenPatternQuote} ${a1.className}`}
        style={a1.style}
      >
        {content.hiddenPattern}
      </p>
    </section>
  );
}
