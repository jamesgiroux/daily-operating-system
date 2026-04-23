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
          Your day, ready as it changes.
        </h1>
        <p className={styles.bodyTextConstrained}>
          DailyOS keeps your day in view: meeting prep, email triage,
          actions due, and context that updates as work changes. Open it
          whenever you need the thread.
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
              Before work
            </span>
            <span className={styles.timelineEntry}>
              DailyOS prepares the first pass automatically
            </span>
          </div>
          <div className={styles.flexRowBaseline}>
            <span className={styles.timestamp}>
              All day
            </span>
            <span className={styles.timelineEntry}>
              Meetings, email, and actions stay connected
            </span>
          </div>
          <div className={styles.flexRowBaseline}>
            <span className={styles.timestampAccent}>
              Whenever
            </span>
            <span className={styles.timelineHighlight}>
              You know what changed and what needs attention
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
