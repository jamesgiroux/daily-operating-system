import type React from "react";
import { useSlideActive, anim } from "../hooks";
import type { MonthlyWrappedContent } from "../types";
import { AnimatedNumber } from "@/components/monthly-wrapped/AnimatedNumber";
import styles from "../monthly-wrapped.module.css";

export function PrioritySlide({
  content,
  onNavigateToMe,
}: {
  content: MonthlyWrappedContent;
  onNavigateToMe: () => void;
}) {
  const active = useSlideActive("priority");
  const hasPriority = content.priorityAlignmentPct !== null;

  // Badge style based on alignment label — dynamic data-dependent colors, kept inline
  const labelStyle = (): React.CSSProperties => {
    const label = (content.priorityAlignmentLabel ?? "").toLowerCase();
    if (label.includes("on track") || label.includes("strong")) {
      return {
        background: "rgba(126, 170, 123, 0.25)",
        color: "#c8e6c5",
        border: "1px solid rgba(126, 170, 123, 0.4)",
      };
    }
    if (label.includes("worth") || label.includes("look")) {
      return {
        background: "rgba(201, 162, 39, 0.25)",
        color: "#ffe082",
        border: "1px solid rgba(201, 162, 39, 0.4)",
      };
    }
    return {
      background: "rgba(255,255,255,0.1)",
      color: "rgba(255,255,255,0.75)",
      border: "1px solid rgba(255,255,255,0.2)",
    };
  };

  const a0 = anim(styles, active, "slideUp");
  const a1 = anim(styles, active, "fadeIn", "100ms");
  const a2 = anim(styles, active, "slideUp", "300ms");
  const a3 = anim(styles, active, "slideUp", "500ms");
  const a4 = anim(styles, active, "slideUp", "100ms");

  return (
    <section id="priority" className={`${styles.slide} ${styles.bgSage}`}>
      <div
        className={`${styles.overline} ${styles.priorityOverline} ${a0.className}`}
        style={a0.style}
      >
        Were you spending time where it matters?
      </div>

      {hasPriority ? (
        <>
          <div
            className={`${styles.statNumber} ${styles.priorityStat} ${a1.className}`}
            style={a1.style}
          >
            <AnimatedNumber value={content.priorityAlignmentPct!} duration={1200} />%
          </div>

          <div
            className={`${styles.statLabel} ${styles.priorityStatLabel} ${a2.className}`}
            style={a2.style}
          >
            of relationship energy on priority accounts
          </div>

          {content.priorityAlignmentLabel && (
            <div
              className={`${styles.alignmentBadge} ${a3.className}`}
              style={{ ...a3.style, ...labelStyle() }}
            >
              {content.priorityAlignmentLabel}
            </div>
          )}
        </>
      ) : (
        <div className={a4.className} style={a4.style}>
          <p className={`${styles.slideSubtitle} ${styles.priorityEmptyText}`}>
            Set priorities on your profile to track alignment month over month.
          </p>
          <button
            onClick={onNavigateToMe}
            className={`${styles.ctaButton} ${styles.priorityEmptyCta}`}
          >
            Go to /me
          </button>
        </div>
      )}
    </section>
  );
}
