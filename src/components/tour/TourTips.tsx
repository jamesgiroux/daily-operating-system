/**
 * TourTips.tsx
 *
 * Floating corner card with 4 post-wizard tips.
 * Appears after wizard completion on first real launch.
 * Not a spotlight overlay — just a corner card with navigation.
 */

import { useState } from "react";
import { useAppState } from "@/hooks/useAppState";
import styles from "./TourTips.module.css";

interface Tip {
  title: string;
  body: string;
}

const TIPS: Tip[] = [
  {
    title: "Your daily briefing",
    body: "This page updates every morning with your schedule, priorities, and what needs attention.",
  },
  {
    title: "Meeting briefings",
    body: "Click any meeting to see who\u2019s attending, what\u2019s relevant, and what to prepare.",
  },
  {
    title: "Your book of business",
    body: "Accounts and projects build context over time. The more you add, the better briefings get.",
  },
  {
    title: "The inbox",
    body: "Drop meeting notes, transcripts, or documents here. DailyOS reads them and connects insights to your accounts.",
  },
];

export function TourTips() {
  const { appState, completeTour } = useAppState();
  const [step, setStep] = useState(0);

  // Only show if wizard completed, tour not completed, and not in demo mode
  if (
    appState.demoModeActive ||
    appState.hasCompletedTour ||
    !appState.wizardCompletedAt
  ) {
    return null;
  }

  const tip = TIPS[step];
  const isFirst = step === 0;
  const isLast = step === TIPS.length - 1;

  function handleDismiss() {
    completeTour();
  }

  return (
    <div className={styles.card}>
      <div className={styles.label}>Getting started</div>
      <h3 className={styles.title}>{tip.title}</h3>
      <p className={styles.body}>{tip.body}</p>

      <div className={styles.footer}>
        {/* Progress dots */}
        <div className={styles.dots}>
          {TIPS.map((_, i) => (
            <div
              key={i}
              className={`${styles.dot} ${i === step ? styles.dotActive : ""}`}
            />
          ))}
        </div>

        {/* Navigation */}
        <div className={styles.actions}>
          {!isFirst && (
            <button
              className={styles.navButton}
              onClick={() => setStep(step - 1)}
            >
              Back
            </button>
          )}
          {isFirst && (
            <button className={styles.navButton} onClick={handleDismiss}>
              Skip tips
            </button>
          )}
          {isLast ? (
            <button className={styles.doneButton} onClick={handleDismiss}>
              Done
            </button>
          ) : (
            <button
              className={styles.navButton}
              onClick={() => setStep(step + 1)}
              style={{ color: "var(--color-text-primary)" }}
            >
              Next
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
