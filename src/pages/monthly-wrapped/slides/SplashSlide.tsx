import { useSlideActive, anim } from "../hooks";
import type { MonthlyWrappedContent } from "../types";
import styles from "../monthly-wrapped.module.css";

export function SplashSlide({ content }: { content: MonthlyWrappedContent }) {
  const active = useSlideActive("splash");

  const year = content.monthLabel
    ? content.monthLabel.split(" ").find((p) => /^\d{4}$/.test(p)) ?? ""
    : "";
  const month = content.monthLabel
    ? content.monthLabel.replace(year, "").trim()
    : content.monthLabel;

  const a0 = anim(styles, active, "slideUp");
  const a1 = anim(styles, active, "slideUpSlow", "200ms");
  const a2 = anim(styles, active, "slideUpSlow", "400ms");

  return (
    <section id="splash" className={`${styles.slide} ${styles.bgInk}`}>
      {/* Background texture — unique radial-gradient geometry, kept inline */}
      <div
        aria-hidden
        className={styles.bgTexture}
        style={{
          background:
            "radial-gradient(ellipse at 20% 80%, rgba(107, 168, 164, 0.08) 0%, transparent 60%), " +
            "radial-gradient(ellipse at 80% 20%, rgba(201, 162, 39, 0.06) 0%, transparent 50%)",
        }}
      />

      <div className={styles.contentLayer}>
        <div
          className={`${styles.overlineSmall} ${styles.splashOverline} ${a0.className}`}
          style={a0.style}
        >
          Monthly Wrapped
        </div>

        <h1
          className={`${styles.slideTitle} ${styles.splashMonth} ${a1.className}`}
          style={a1.style}
        >
          {month}
        </h1>

        <div
          className={`${styles.splashYear} ${a2.className}`}
          style={a2.style}
        >
          {year}
        </div>
      </div>
    </section>
  );
}
