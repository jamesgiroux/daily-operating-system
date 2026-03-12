import { useSlideActive, anim } from "../hooks";
import type { MonthlyWrappedContent } from "../types";
import { AnimatedNumber } from "@/components/monthly-wrapped/AnimatedNumber";
import styles from "../monthly-wrapped.module.css";

export function TopAccountsSlide({ content }: { content: MonthlyWrappedContent }) {
  const active = useSlideActive("top-account");

  const a0 = anim(styles, active, "slideUp");
  const a1 = anim(styles, active, "scaleReveal", "150ms");
  const a2 = anim(styles, active, "slideUp", "350ms");

  return (
    <section id="top-account" className={`${styles.slide} ${styles.bgTurmeric}`}>
      <div
        className={`${styles.overline} ${styles.topAccountOverline} ${a0.className}`}
        style={a0.style}
      >
        Your most active account.
      </div>

      <h2
        className={`${styles.slideTitle} ${styles.topAccountName} ${a1.className}`}
        style={a1.style}
      >
        {content.topEntityName}
      </h2>

      <div
        className={`${styles.topAccountTouches} ${a2.className}`}
        style={a2.style}
      >
        <AnimatedNumber value={content.topEntityTouches} /> touchpoints this month
      </div>
    </section>
  );
}
