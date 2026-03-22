/**
 * RoadmapSlide — Slide 6: What's Ahead.
 * One big serif paragraph — intentionally spacious.
 */
import { EditableText } from "@/components/ui/EditableText";
import type { EbrQbrContent } from "@/types/reports";

interface RoadmapSlideProps {
  content: EbrQbrContent;
  onUpdate: (c: EbrQbrContent) => void;
}

export function RoadmapSlide({ content, onUpdate }: RoadmapSlideProps) {
  return (
    <section
      id="whats-ahead"
      className="report-surface-slide"
      style={{ scrollMarginTop: 60 }}
    >
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "var(--color-garden-larkspur)",
          marginBottom: 48,
        }}
      >
        {"What's Ahead"}
      </div>

      {/* Strategic roadmap — big serif paragraph, let it breathe */}
      <EditableText
        as="p"
        value={content.strategicRoadmap}
        onChange={(v) => onUpdate({ ...content, strategicRoadmap: v })}
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 24,
          fontWeight: 400,
          lineHeight: 1.7,
          color: "var(--color-text-primary)",
          maxWidth: 700,
          margin: 0,
        }}
      />
    </section>
  );
}
