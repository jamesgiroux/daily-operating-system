import { useSlideActive, anim } from "../hooks";
import type { MonthlyWrappedContent } from "../types";
import styles from "../monthly-wrapped.module.css";

export function PersonalitySlide({ content }: { content: MonthlyWrappedContent }) {
  const active = useSlideActive("personality");

  const a0 = anim(styles, active, "fadeIn");
  const a1 = anim(styles, active, "scaleReveal", "150ms");
  const a2 = anim(styles, active, "slideUp", "400ms");
  const a3 = anim(styles, active, "slideUp", "550ms");
  const a4 = anim(styles, active, "slideUp", "700ms");

  return (
    <section id="personality" className={`${styles.slide} ${styles.bgRosemary}`}>
      {/* Subtle radial glow behind the type name — unique geometry, kept inline */}
      <div
        aria-hidden
        className={styles.bgTexture}
        style={{
          top: "40%",
          left: "50%",
          transform: "translate(-50%, -50%)",
          width: 600,
          height: 400,
          inset: "auto",
          background:
            "radial-gradient(ellipse at center, rgba(222, 184, 65, 0.12) 0%, transparent 70%)",
        }}
      />

      <div className={styles.contentLayer}>
        <div
          className={`${styles.overlineSmall} ${styles.personalityOverline} ${a0.className}`}
          style={a0.style}
        >
          Your type this month
        </div>

        <h2
          className={`${styles.slideTitle} ${styles.personalityTypeName} ${a1.className}`}
          style={a1.style}
        >
          {content.personality.typeName}
        </h2>

        <p
          className={`${styles.slideSubtitle} ${styles.personalityDescription} ${a2.className}`}
          style={a2.style}
        >
          {content.personality.description}
        </p>

        {/* Key defining trait */}
        <div
          className={`${styles.personalityKeyTrait} ${a3.className}`}
          style={a3.style}
        >
          {content.personality.keySignal}
        </div>

        <div
          className={`${styles.rarityDivider} ${styles.overlineSmall} ${styles.personalityRarity} ${a4.className}`}
          style={a4.style}
        >
          {content.personality.rarityLabel}
        </div>
      </div>
    </section>
  );
}
