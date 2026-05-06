/**
 * AccountExecutiveSummary — italic serif narrative, extracted from AccountHero.
 *
 * Rendered at the top of the Context view. Matches vitals strip width
 * (no max-width cap, flows to page container width).
 */
import { useState } from "react";
import type { EntityIntelligence } from "@/types";
import { ClaimTextRenderer } from "@/components/ui/ClaimTextRenderer";
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

  if (!narrative) return null;

  return (
    <div className={`editorial-reveal ${styles.summary}`}>
      {intelligence?.executiveAssessmentRenderPolicy ? (
        <p className={styles.paragraph}>
          <ClaimTextRenderer
            value={{
              text: narrative,
              policy: intelligence.executiveAssessmentRenderPolicy,
            }}
            surface="tauri_entity_detail"
          />
        </p>
      ) : (
        narrative.split("\n\n").map((p, i) => (
          <p key={i} className={i === 0 ? styles.paragraph : styles.paragraphSpaced}>{p}</p>
        ))
      )}
      {narrativeTruncated && (
        <button onClick={() => setShowFullLede(true)} className={styles.readMore}>
          Read more
        </button>
      )}
    </div>
  );
}
