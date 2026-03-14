/**
 * StateOfPlay — Two StateBlocks (working/struggling) + PullQuote.
 * Renders current state data from intelligence.
 * Generalized: already entity-generic (only uses EntityIntelligence).
 *
 * I261: Optional onUpdateField prop enables click-to-edit on state items.
 * I261: List truncation (5 per section) + empty section collapse.
 * I550: Per-item dismiss and feedback controls.
 */
import { useState } from "react";
import type { ReactNode } from "react";
import type { EntityIntelligence } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { StateBlock } from "@/components/editorial/StateBlock";
import styles from "./StateOfPlay.module.css";

interface StateOfPlayProps {
  intelligence: EntityIntelligence | null;
  sectionId?: string;
  chapterTitle?: string;
  /** When provided, items become editable. Called with (fieldPath, newValue). */
  onUpdateField?: (fieldPath: string, value: string) => void;
  /** I529: Optional feedback controls for chapter heading */
  feedbackSlot?: ReactNode;
  /** Per-item feedback value getter. Field path like "currentState.working[0]". */
  getItemFeedback?: (fieldPath: string) => "positive" | "negative" | null;
  /** Per-item feedback submit. */
  onItemFeedback?: (fieldPath: string, type: "positive" | "negative") => void;
}

export function StateOfPlay({
  intelligence,
  sectionId = "state-of-play",
  chapterTitle = "State of Play",
  onUpdateField,
  feedbackSlot,
  getItemFeedback,
  onItemFeedback,
}: StateOfPlayProps) {
  const working = (intelligence?.currentState?.working ?? []).filter((w) => w?.trim());
  const struggling = (intelligence?.currentState?.notWorking ?? []).filter((s) => s?.trim());

  const hasContent = working.length > 0 || struggling.length > 0;

  const [expandedWorking, setExpandedWorking] = useState(false);
  const [expandedStruggling, setExpandedStruggling] = useState(false);

  // Empty section collapse
  if (!hasContent) {
    return null;
  }

  const STATE_LIMIT = 3;
  const visibleWorking = expandedWorking ? working : working.slice(0, STATE_LIMIT);
  const hasMoreWorking = working.length > STATE_LIMIT && !expandedWorking;
  const visibleStruggling = expandedStruggling ? struggling : struggling.slice(0, STATE_LIMIT);
  const hasMoreStruggling = struggling.length > STATE_LIMIT && !expandedStruggling;

  return (
    <section
      id={sectionId || undefined}
      className={sectionId ? styles.sectionWithScrollMargin : undefined}
    >
      <ChapterHeading title={chapterTitle} feedbackSlot={feedbackSlot} />

      <div className={styles.stateColumns}>
        <div className={styles.stateColumn}>
          <StateBlock
            label="What's Working"
            items={visibleWorking}
            labelColor="var(--color-garden-sage)"
            onItemChange={
              onUpdateField
                ? (index, value) => onUpdateField(`currentState.working[${index}]`, value)
                : undefined
            }
            onItemDismiss={
              onUpdateField
                ? (index) => onUpdateField(`currentState.working[${index}]`, "")
                : undefined
            }
            getItemFeedback={
              getItemFeedback
                ? (index) => getItemFeedback(`currentState.working[${index}]`)
                : undefined
            }
            onItemFeedback={
              onItemFeedback
                ? (index, type) => onItemFeedback(`currentState.working[${index}]`, type)
                : undefined
            }
          />
          {hasMoreWorking && (
            <button onClick={() => setExpandedWorking(true)} className={styles.showMoreButton}>
              Show {working.length - STATE_LIMIT} more
            </button>
          )}
        </div>
        <div className={styles.stateColumn}>
          <StateBlock
            label="Where It's Struggling"
            items={visibleStruggling}
            labelColor="var(--color-spice-terracotta)"
            onItemChange={
              onUpdateField
                ? (index, value) => onUpdateField(`currentState.notWorking[${index}]`, value)
                : undefined
            }
            onItemDismiss={
              onUpdateField
                ? (index) => onUpdateField(`currentState.notWorking[${index}]`, "")
                : undefined
            }
            getItemFeedback={
              getItemFeedback
                ? (index) => getItemFeedback(`currentState.notWorking[${index}]`)
                : undefined
            }
            onItemFeedback={
              onItemFeedback
                ? (index, type) => onItemFeedback(`currentState.notWorking[${index}]`, type)
                : undefined
            }
          />
          {hasMoreStruggling && (
            <button onClick={() => setExpandedStruggling(true)} className={styles.showMoreButton}>
              Show {struggling.length - STATE_LIMIT} more
            </button>
          )}
        </div>
      </div>
    </section>
  );
}
