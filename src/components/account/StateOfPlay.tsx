/**
 * StateOfPlay — Chapter 2: Two StateBlocks (working/struggling) + PullQuote.
 * Renders current state data from intelligence.
 */
import type { EntityIntelligence } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { StateBlock } from "@/components/editorial/StateBlock";
import { PullQuote } from "@/components/editorial/PullQuote";

interface StateOfPlayProps {
  intelligence: EntityIntelligence | null;
}

export function StateOfPlay({ intelligence }: StateOfPlayProps) {
  const working = intelligence?.currentState?.working ?? [];
  const struggling = intelligence?.currentState?.notWorking ?? [];

  // Extract a pull quote from the executive assessment (second paragraph if exists)
  const paragraphs = intelligence?.executiveAssessment?.split("\n").filter((p) => p.trim()) ?? [];
  const pullQuote = paragraphs.length > 1 ? paragraphs[1] : null;

  const hasContent = working.length > 0 || struggling.length > 0;

  return (
    <section id="state-of-play" style={{ scrollMarginTop: 60, paddingTop: 80 }}>
      <ChapterHeading number={2} title="State of Play" />

      {hasContent ? (
        <>
          <StateBlock
            label="What's Working"
            items={working}
            labelColor="var(--color-garden-sage)"
          />
          <StateBlock
            label="Where It's Struggling"
            items={struggling}
            labelColor="var(--color-spice-terracotta)"
          />
          {pullQuote && <PullQuote text={pullQuote} />}
        </>
      ) : (
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            color: "var(--color-text-tertiary)",
            fontStyle: "italic",
          }}
        >
          Build intelligence to populate this section.
        </p>
      )}
    </section>
  );
}
