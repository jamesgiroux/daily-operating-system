/**
 * StateOfPlay — Two StateBlocks (working/struggling) + PullQuote.
 * Renders current state data from intelligence.
 * Generalized: already entity-generic (only uses EntityIntelligence).
 *
 * I261: Optional onUpdateField prop enables click-to-edit on state items.
 * I261: List truncation (5 per section) + empty section collapse.
 */
import { useState } from "react";
import type { EntityIntelligence } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { StateBlock } from "@/components/editorial/StateBlock";
import { PullQuote } from "@/components/editorial/PullQuote";

interface StateOfPlayProps {
  intelligence: EntityIntelligence | null;
  sectionId?: string;
  chapterTitle?: string;
  /** When provided, items become editable. Called with (fieldPath, newValue). */
  onUpdateField?: (fieldPath: string, value: string) => void;
}

export function StateOfPlay({
  intelligence,
  sectionId = "state-of-play",
  chapterTitle = "State of Play",
  onUpdateField,
}: StateOfPlayProps) {
  const working = intelligence?.currentState?.working ?? [];
  const struggling = intelligence?.currentState?.notWorking ?? [];

  const paragraphs = intelligence?.executiveAssessment?.split("\n").filter((p) => p.trim()) ?? [];
  const pullQuote = paragraphs.length > 1 ? paragraphs[1] : null;

  const hasContent = working.length > 0 || struggling.length > 0;

  const [expandedWorking, setExpandedWorking] = useState(false);
  const [expandedStruggling, setExpandedStruggling] = useState(false);

  // Empty section collapse
  if (!hasContent) {
    return null;
  }

  const STATE_LIMIT = 5;
  const visibleWorking = expandedWorking ? working : working.slice(0, STATE_LIMIT);
  const hasMoreWorking = working.length > STATE_LIMIT && !expandedWorking;
  const visibleStruggling = expandedStruggling ? struggling : struggling.slice(0, STATE_LIMIT);
  const hasMoreStruggling = struggling.length > STATE_LIMIT && !expandedStruggling;

  const showMoreButtonStyle: React.CSSProperties = {
    fontFamily: "var(--font-mono)",
    fontSize: 11,
    color: "var(--color-text-tertiary)",
    background: "none",
    border: "none",
    cursor: "pointer",
    padding: "8px 0 0",
    textTransform: "uppercase",
    letterSpacing: "0.06em",
  };

  return (
    <section id={sectionId} style={{ scrollMarginTop: 60, paddingTop: 80 }}>
      <ChapterHeading title={chapterTitle} />

      <StateBlock
        label="What's Working"
        items={visibleWorking}
        labelColor="var(--color-garden-sage)"
        onItemChange={
          onUpdateField
            ? (index, value) => onUpdateField(`currentState.working[${index}]`, value)
            : undefined
        }
      />
      {hasMoreWorking && (
        <button onClick={() => setExpandedWorking(true)} style={showMoreButtonStyle}>
          Show {working.length - STATE_LIMIT} more
        </button>
      )}
      <StateBlock
        label="Where It's Struggling"
        items={visibleStruggling}
        labelColor="var(--color-spice-terracotta)"
        onItemChange={
          onUpdateField
            ? (index, value) => onUpdateField(`currentState.notWorking[${index}]`, value)
            : undefined
        }
      />
      {hasMoreStruggling && (
        <button onClick={() => setExpandedStruggling(true)} style={showMoreButtonStyle}>
          Show {struggling.length - STATE_LIMIT} more
        </button>
      )}
      {pullQuote && <PullQuote text={pullQuote} />}
    </section>
  );
}
