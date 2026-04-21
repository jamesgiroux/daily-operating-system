/**
 * AccountExecutiveSummary — italic serif narrative, extracted from AccountHero.
 *
 * Rendered at the top of the Context view. Matches vitals strip width
 * (no max-width cap, flows to page container width).
 */
import { useState } from "react";
import type { EntityIntelligence } from "@/types";
import { hasBleedFlag } from "@/lib/contamination-guard";
import { ContaminationWarning } from "@/components/ui/ContaminationWarning";
import styles from "./AccountExecutiveSummary.module.css";

interface Props {
  intelligence: EntityIntelligence | null;
}

const LEDE_LIMIT = 500;

export function AccountExecutiveSummary({ intelligence }: Props) {
  const paragraphs = intelligence?.executiveAssessment?.split("\n").filter((p) => p.trim()) ?? [];
  const fullNarrative = paragraphs.join("\n\n");
  const [showFullLede, setShowFullLede] = useState(false);
  const narrativeTruncated = fullNarrative.length > LEDE_LIMIT && !showFullLede;
  const narrative = narrativeTruncated ? fullNarrative.slice(0, LEDE_LIMIT) + "\u2026" : fullNarrative;

  // DOS-83: Check if executive assessment is flagged as cross-entity contamination.
  const assessmentBleed = hasBleedFlag(intelligence?.consistencyFindings, "executiveAssessment");

  if (!narrative) return null;

  return (
    <div className={`editorial-reveal ${styles.summary}`}>
      {assessmentBleed && <ContaminationWarning />}
      {!assessmentBleed && narrative.split("\n\n").map((p, i) => (
        <p key={i} className={i === 0 ? styles.paragraph : styles.paragraphSpaced}>{p}</p>
      ))}
      {!assessmentBleed && narrativeTruncated && (
        <button onClick={() => setShowFullLede(true)} className={styles.readMore}>
          Read more
        </button>
      )}
    </div>
  );
}
