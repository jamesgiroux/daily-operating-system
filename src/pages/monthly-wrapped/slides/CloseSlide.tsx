import { useSlideActive, anim } from "../hooks";
import { nextMonthName } from "../constants";
import type { MonthlyWrappedContent } from "../types";
import styles from "../monthly-wrapped.module.css";

export function CloseSlide({ content }: { content: MonthlyWrappedContent }) {
  const active = useSlideActive("close");
  const next = nextMonthName(content.monthLabel);

  const a0 = anim(styles, active, "slideUpLong");
  const a1 = anim(styles, active, "fadeInSlow", "300ms");
  const a2 = anim(styles, active, "fadeInSlow", "600ms");

  return (
    <section id="close" className={`${styles.slide} ${styles.bgInk}`}>
      {/* Subtle background glow — unique radial-gradient geometry, kept inline */}
      <div
        aria-hidden
        className={styles.bgTexture}
        style={{
          background:
            "radial-gradient(ellipse at 80% 30%, rgba(107, 168, 164, 0.06) 0%, transparent 60%)",
        }}
      />

      <div className={styles.contentLayer}>
        <h2
          className={`${styles.slideTitle} ${styles.closeHeadline} ${a0.className}`}
          style={a0.style}
        >
          See you in {next}.
        </h2>

        <div
          className={`${styles.closeSubLabel} ${a1.className}`}
          style={a1.style}
        >
          Your {content.monthLabel}.
        </div>

        <button
          onClick={() => window.print()}
          className={`${styles.ctaButton} ${styles.closeExportCta} ${a2.className}`}
          style={a2.style}
        >
          Save as PDF
        </button>
      </div>
    </section>
  );
}
