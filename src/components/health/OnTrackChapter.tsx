/**
 * OnTrackChapter — fine-state chapter.
 *
 * Renders when `hasTriageContent() === false && hasDivergenceContent() === false`.
 * Mirrors `.docs/mockups/account-health-fine-globex.html`: "On track" heading,
 * serif italic reassurance body, peer-benchmark reassurance strip.
 *
 * Philosophy: empty states are features, density IS the verdict, "fine" must
 * feel prepared not abandoned.
 */
import type { EntityIntelligence } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { ChapterFreshness } from "@/components/editorial/ChapterFreshness";
import styles from "./health.module.css";

interface OnTrackChapterProps {
  intelligence: EntityIntelligence | null;
  accountSizeLabel?: string | null;
}

export function OnTrackChapter({ intelligence, accountSizeLabel }: OnTrackChapterProps) {
  return (
    <>
      <ChapterHeading
        title="On track"
        freshness={
          <ChapterFreshness
            enrichedAt={intelligence?.enrichedAt}
            fragments={["Nothing active needs your attention"]}
          />
        }
      />

      <p className={styles.ontrackBody}>
        No active friction. No divergences surfacing. No stakeholder changes.
        Support quiet, engagement stable. You're in a maintenance window — a good
        time to invest in the relationship rather than react to it.
      </p>

      <div className={styles.peerStrip}>
        <div className={styles.peerNumber}>—</div>
        <div className={styles.peerLabel}>Peer benchmark · coming in a later pass</div>
        <div className={styles.peerContext}>
          Peer cohort benchmarking isn't wired yet. When it lands, this strip will
          show how accounts similar to{" "}
          <strong>{accountSizeLabel ?? "this one"}</strong> have trended over the
          last twelve months. For now: steady is the story.
        </div>
      </div>
    </>
  );
}
