import { useSlideActive, anim } from "../hooks";
import type { MonthlyWrappedContent } from "../types";
import styles from "../monthly-wrapped.module.css";

export function TopWinSlide({ content }: { content: MonthlyWrappedContent }) {
  const active = useSlideActive("top-win");

  const a0 = anim(styles, active, "slideUp");
  const a1 = anim(styles, active, "slideUpSlow", "200ms");
  const a2 = anim(styles, active, "fadeInSlow", "600ms");

  return (
    <section id="top-win" className={`${styles.slide} ${styles.bgSaffron}`}>
      <div
        className={`${styles.overline} ${styles.topWinOverline} ${a0.className}`}
        style={a0.style}
      >
        Your biggest win.
      </div>

      <p
        className={`${styles.serifQuote} ${styles.topWinQuote} ${a1.className}`}
        style={a1.style}
      >
        {content.topWin}
      </p>

      <div
        className={`${styles.flourish} ${styles.topWinFlourish} ${a2.className}`}
        style={a2.style}
      >
        &#10022;
      </div>
    </section>
  );
}
