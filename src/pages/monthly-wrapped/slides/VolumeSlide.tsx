import { useSlideActive, anim } from "../hooks";
import type { MonthlyWrappedContent } from "../types";
import { AnimatedNumber } from "@/components/monthly-wrapped/AnimatedNumber";
import styles from "../monthly-wrapped.module.css";

export function VolumeSlide({ content }: { content: MonthlyWrappedContent }) {
  const active = useSlideActive("volume");

  const a0 = anim(styles, active, "slideUp");
  const a1 = anim(styles, active, "fadeIn", "100ms");
  const a2 = anim(styles, active, "slideUp", "200ms");
  const a3 = anim(styles, active, "slideUp", "400ms");

  return (
    <section id="volume" className={`${styles.slide} ${styles.bgEucalyptus}`}>
      <div
        className={`${styles.overline} ${styles.volumeOverline} ${a0.className}`}
        style={a0.style}
      >
        You showed up.
      </div>

      <div
        className={`${styles.statNumber} ${styles.volumeStat} ${a1.className}`}
        style={a1.style}
      >
        <AnimatedNumber value={content.totalConversations} duration={1500} />
      </div>

      <div
        className={`${styles.statLabel} ${styles.volumeStatLabel} ${a2.className}`}
        style={a2.style}
      >
        conversations
      </div>

      <div
        className={`${styles.subStats} ${styles.volumeSubStats} ${a3.className}`}
        style={a3.style}
      >
        <AnimatedNumber value={content.totalEntitiesTouched} /> accounts
        &nbsp;&middot;&nbsp;
        <AnimatedNumber value={content.totalPeopleMet} /> people
        &nbsp;&middot;&nbsp;
        <AnimatedNumber value={content.signalsCaptured} /> updates
      </div>
    </section>
  );
}
