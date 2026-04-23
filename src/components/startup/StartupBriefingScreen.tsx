import { useEffect, useRef, useState } from "react";
import styles from "./StartupBriefingScreen.module.css";

export interface StartupProgressPhase {
  key: string;
  label: string;
  detail: string;
}

const STARTUP_BRIEFING_COPY = {
  title: "Preparing your daily briefing",
  navigateHint: "This runs in the background - feel free to navigate away",
};

const DAILY_BRIEFING_PHASES: StartupProgressPhase[] = [
  { key: "preparing", label: "Gathering your day", detail: "Pulling calendar, emails, and account context" },
  { key: "enriching", label: "Building context", detail: "Assembling meeting briefings, priorities, and action items" },
  { key: "delivering", label: "Composing the briefing", detail: "Writing your daily briefing" },
];

const DAILY_BRIEFING_QUOTES = [
  "Your day is coming into focus.",
  "Checking the calendar against the work that matters.",
  "Separating signal from noise.",
  "Assembling the context before the first meeting starts.",
  "Turning the day into a briefing you can use.",
];

interface StartupBriefingScreenProps {
  mode?: "splash" | "progress";
  fading?: boolean;
  currentPhaseKey?: string;
  phases?: StartupProgressPhase[];
  elapsed?: number;
  quotes?: string[];
  showNavigateHint?: boolean;
}

function formatElapsed(secs: number) {
  const minutes = Math.floor(secs / 60);
  const seconds = secs % 60;
  return minutes > 0 ? `${minutes}m ${seconds}s` : `${seconds}s`;
}

function BrandMark() {
  return (
    <svg className={styles.brandMark} xmlns="http://www.w3.org/2000/svg" viewBox="0 0 433 407" aria-hidden="true">
      <path d="M159 407 161 292 57 355 0 259 102 204 0 148 57 52 161 115 159 0H273L271 115L375 52L433 148L331 204L433 259L375 355L271 292L273 407Z" fill="currentColor" />
    </svg>
  );
}

export function StartupBriefingScreen({
  mode = "splash",
  fading = false,
  currentPhaseKey = "preparing",
  phases = DAILY_BRIEFING_PHASES,
  elapsed: elapsedProp,
  quotes = DAILY_BRIEFING_QUOTES,
  showNavigateHint = true,
}: StartupBriefingScreenProps) {
  const [quoteIndex, setQuoteIndex] = useState(0);
  const [localElapsed, setLocalElapsed] = useState(0);
  const startTime = useRef(Date.now());
  const shuffledQuotes = useRef([...quotes].sort(() => Math.random() - 0.5));
  const elapsed = elapsedProp ?? localElapsed;
  const currentPhaseIndex = Math.max(0, phases.findIndex((phase) => phase.key === currentPhaseKey));

  useEffect(() => {
    if (mode !== "progress") return;
    const interval = setInterval(() => {
      setQuoteIndex((index) => (index + 1) % shuffledQuotes.current.length);
    }, 8000);
    return () => clearInterval(interval);
  }, [mode]);

  useEffect(() => {
    if (mode !== "progress" || elapsedProp != null) return;
    const interval = setInterval(() => {
      setLocalElapsed(Math.floor((Date.now() - startTime.current) / 1000));
    }, 1000);
    return () => clearInterval(interval);
  }, [elapsedProp, mode]);

  if (mode === "progress") {
    return (
      <section className={`${styles.screen} ${styles.progress}`} aria-label={STARTUP_BRIEFING_COPY.title}>
        <div className={styles.elapsed}>{formatElapsed(elapsed)}</div>
        <div className={styles.progressSpacer} />
        <div className={styles.progressContent}>
          <div className={styles.sectionRule} />
          <div className={styles.progressTitle}>{STARTUP_BRIEFING_COPY.title}</div>
          <div className={styles.phaseList}>
            {phases.map((phase, index) => {
              const state = index < currentPhaseIndex
                ? "complete"
                : index === currentPhaseIndex
                  ? "current"
                  : "pending";

              return (
                <div key={phase.key} className={styles.phaseRow} data-state={state}>
                  <div className={styles.phaseDot}>
                    {state === "complete" && (
                      <svg width="10" height="10" viewBox="0 0 12 12" fill="none" aria-hidden="true">
                        <path d="M2 6l3 3 5-5" stroke="white" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                      </svg>
                    )}
                    {state === "current" && <div className={styles.phasePulse} />}
                  </div>
                  <div className={styles.phaseText}>
                    <div className={styles.phaseLabel}>{phase.label}</div>
                    {state === "current" && <div className={styles.phaseDetail}>{phase.detail}</div>}
                  </div>
                </div>
              );
            })}
          </div>
          <div className={styles.quoteBox}>
            <p className={styles.quote} key={quoteIndex}>{shuffledQuotes.current[quoteIndex]}</p>
          </div>
          {showNavigateHint && <div className={styles.navigateHint}>{STARTUP_BRIEFING_COPY.navigateHint}</div>}
        </div>
      </section>
    );
  }

  return (
    <div className={`${styles.screen} ${styles.overlay} ${fading ? styles.fading : ""}`} aria-label={STARTUP_BRIEFING_COPY.title}>
      <div className={styles.splashStack}>
        <div className={`${styles.rule} ${styles.ruleTop}`} />
        <BrandMark />
        <div className={styles.brandName}>DailyOS</div>
        <div className={styles.splashTitle}>{STARTUP_BRIEFING_COPY.title}</div>
        <div className={`${styles.rule} ${styles.ruleBottom}`} />
      </div>
    </div>
  );
}
