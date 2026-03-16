import { ArrowRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { BrandMark } from "@/components/ui/BrandMark";
import styles from "../onboarding.module.css";

interface WelcomeProps {
  onNext: () => void;
  onDemoMode?: () => void;
  onSkipSetup?: () => void;
}

export function Welcome({ onNext, onDemoMode, onSkipSetup }: WelcomeProps) {
  return (
    <div className={`${styles.flexCol} ${styles.gap32}`}>
      {/* Brand mark */}
      <div className={styles.brandMark}>
        <BrandMark size={48} />
      </div>

      {/* Hero headline — serif, left-aligned */}
      <div className={`${styles.flexCol} ${styles.gap16}`}>
        <h1 className={styles.heroHeadline}>
          Open the app. Your day is ready.
        </h1>
        <p className={styles.bodyTextConstrained}>
          DailyOS prepares your day while you sleep — meeting prep,
          email triage, actions due, and a morning summary. You open it,
          read, and get to work.
        </p>
      </div>

      {/* Timeline block — editorial rule-separated */}
      <div className={styles.ruleSection}>
        <div className={styles.sectionLabel}>
          What it looks like
        </div>
        <div className={`${styles.flexCol} ${styles.gap8}`}>
          <div className={styles.flexRowBaseline}>
            <span className={styles.timestamp}>
              6:00 AM
            </span>
            <span className={styles.timelineEntry}>
              Your briefing generates automatically
            </span>
          </div>
          <div className={styles.flexRowBaseline}>
            <span className={styles.timestamp}>
              8:00 AM
            </span>
            <span className={styles.timelineEntry}>
              You open the app. Everything's there.
            </span>
          </div>
          <div className={styles.flexRowBaseline}>
            <span className={styles.timestampAccent}>
              8:15 AM
            </span>
            <span className={styles.timelineHighlight}>
              You're prepared. Close the app. Do your work.
            </span>
          </div>
        </div>
      </div>

      {/* Tagline */}
      <p className={styles.tagline}>
        No setup to maintain. No inbox to clear.
        Skip a day, skip a week — it picks up where you are.
      </p>

      {/* Primary CTA */}
      <div className={`${styles.flexCol} ${styles.gap12}`}>
        <div>
          <Button size="lg" onClick={onNext}>
            Get started
            <ArrowRight className="ml-2 size-4" />
          </Button>
        </div>

        {/* Secondary: Explore with demo data */}
        {onDemoMode && (
          <button
            onClick={onDemoMode}
            className={styles.textLink}
          >
            Explore with demo data
          </button>
        )}
      </div>

      {/* Footer: Skip setup */}
      {onSkipSetup && (
        <div className={styles.ruleSection}>
          <button
            onClick={onSkipSetup}
            className={styles.skipButton}
          >
            Skip setup
          </button>
        </div>
      )}
    </div>
  );
}
