import { useSlideActive, anim } from "../hooks";
import type { MonthlyWrappedContent } from "../types";
import styles from "../monthly-wrapped.module.css";

const PILL_COLORS = [
  { border: "var(--color-spice-turmeric)", color: "var(--color-text-primary)" },
  { border: "var(--color-garden-larkspur)", color: "var(--color-text-primary)" },
  { border: "var(--color-garden-eucalyptus)", color: "var(--color-text-primary)" },
];

export function CarryForwardSlide({ content }: { content: MonthlyWrappedContent }) {
  const active = useSlideActive("carry-forward");
  const words = [content.wordOne, content.wordTwo, content.wordThree].filter(Boolean);

  const a0 = anim(styles, active, "slideUp");
  const a1 = anim(styles, active, "slideUpSlow", "200ms");
  const a2 = anim(styles, active, "slideUp", "400ms");

  return (
    <section id="carry-forward" className={`${styles.slide} ${styles.bgLinen}`}>
      <div
        className={`${styles.overline} ${styles.carryForwardOverline} ${a0.className}`}
        style={a0.style}
      >
        Into next month.
      </div>

      <p
        className={`${styles.slideSubtitle} ${styles.carryForwardText} ${a1.className}`}
        style={a1.style}
      >
        {content.carryForward}
      </p>

      {words.length > 0 && (
        <div className={a2.className} style={a2.style}>
          <div className={`${styles.momentLabel} ${styles.carryForwardWordsLabel}`}>
            {content.monthLabel} in three words
          </div>
          <div className={styles.carryForwardWordRow}>
            {words.map((word, i) => {
              const pillAnim = anim(styles, active, "slideUp", `${400 + i * 120}ms`);
              return (
                <span
                  key={i}
                  className={`${styles.wordPill} ${pillAnim.className}`}
                  style={{
                    ...pillAnim.style,
                    color: PILL_COLORS[i % PILL_COLORS.length].color,
                    border: `1px solid ${PILL_COLORS[i % PILL_COLORS.length].border}`,
                  }}
                >
                  {word}
                </span>
              );
            })}
          </div>
        </div>
      )}
    </section>
  );
}
