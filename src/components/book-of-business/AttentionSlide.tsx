/**
 * AttentionSlide — Portfolio risks and opportunities.
 * Slide 2: what needs attention, presented as editorial callouts.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import { formatArr } from "@/lib/utils";
import type { BookOfBusinessContent, BookRiskItem, BookOpportunityItem } from "@/types/reports";

interface AttentionSlideProps {
  content: BookOfBusinessContent;
  onUpdate: (content: BookOfBusinessContent) => void;
}

export function AttentionSlide({ content, onUpdate }: AttentionSlideProps) {
  return (
    <section
      id="attention"
      style={{
        scrollMarginTop: 60,
        minHeight: "100vh",
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "120px 120px 80px",
        scrollSnapAlign: "start",
      }}
    >
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "var(--color-spice-turmeric)",
          marginBottom: 24,
        }}
      >
        What Needs Attention
      </div>

      <div style={{ display: "flex", gap: 80, maxWidth: 900 }}>
        {/* Risks */}
        <div style={{ flex: 1 }}>
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.1em",
              color: "var(--color-spice-terracotta)",
              marginBottom: 16,
            }}
          >
            Risks
          </div>
          {content.topRisks.map((item, i) => (
            <RiskItem
              key={i}
              index={i + 1}
              item={item}
              canRemove={content.topRisks.length > 1}
              onUpdate={(updated) => {
                const next = [...content.topRisks];
                next[i] = updated;
                onUpdate({ ...content, topRisks: next });
              }}
              onRemove={() => {
                onUpdate({ ...content, topRisks: content.topRisks.filter((_, j) => j !== i) });
              }}
            />
          ))}
          {content.topRisks.length === 0 && (
            <div style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--color-text-tertiary)" }}>
              No risks identified.
            </div>
          )}
        </div>

        {/* Opportunities */}
        <div style={{ flex: 1 }}>
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.1em",
              color: "var(--color-garden-sage)",
              marginBottom: 16,
            }}
          >
            Opportunities
          </div>
          {content.topOpportunities.map((item, i) => (
            <OppItem
              key={i}
              index={i + 1}
              item={item}
              canRemove={content.topOpportunities.length > 1}
              onUpdate={(updated) => {
                const next = [...content.topOpportunities];
                next[i] = updated;
                onUpdate({ ...content, topOpportunities: next });
              }}
              onRemove={() => {
                onUpdate({ ...content, topOpportunities: content.topOpportunities.filter((_, j) => j !== i) });
              }}
            />
          ))}
          {content.topOpportunities.length === 0 && (
            <div style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--color-text-tertiary)" }}>
              No opportunities identified.
            </div>
          )}
        </div>
      </div>
    </section>
  );
}

function RiskItem({
  index,
  item,
  canRemove,
  onUpdate,
  onRemove,
}: {
  index: number;
  item: BookRiskItem;
  canRemove: boolean;
  onUpdate: (item: BookRiskItem) => void;
  onRemove: () => void;
}) {
  const [hovered, setHovered] = useState(false);

  return (
    <div
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        display: "flex",
        alignItems: "baseline",
        gap: 12,
        padding: "12px 0",
        borderBottom: "1px solid var(--color-rule-light)",
      }}
    >
      <span
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 18,
          fontWeight: 600,
          color: "var(--color-spice-terracotta)",
          minWidth: 20,
          flexShrink: 0,
        }}
      >
        {index}
      </span>
      <div style={{ flex: 1 }}>
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            fontWeight: 600,
            textTransform: "uppercase",
            letterSpacing: "0.08em",
            color: "var(--color-text-tertiary)",
            marginBottom: 4,
          }}
        >
          {item.accountName}
          {item.arr != null && (
            <span style={{ marginLeft: 8, fontWeight: 400 }}>${formatArr(item.arr)}</span>
          )}
        </div>
        <EditableText
          value={item.risk}
          onChange={(v) => onUpdate({ ...item, risk: v })}
          multiline={false}
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 16,
            color: "var(--color-text-primary)",
          }}
        />
      </div>
      {canRemove && (
        <button
          onClick={(e) => { e.stopPropagation(); onRemove(); }}
          style={{
            opacity: hovered ? 0.6 : 0,
            transition: "opacity 0.15s",
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: "4px 6px",
            fontSize: 14,
            color: "var(--color-text-tertiary)",
            flexShrink: 0,
          }}
          aria-label="Remove"
        >
          ✕
        </button>
      )}
    </div>
  );
}

function OppItem({
  index,
  item,
  canRemove,
  onUpdate,
  onRemove,
}: {
  index: number;
  item: BookOpportunityItem;
  canRemove: boolean;
  onUpdate: (item: BookOpportunityItem) => void;
  onRemove: () => void;
}) {
  const [hovered, setHovered] = useState(false);

  return (
    <div
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        display: "flex",
        alignItems: "baseline",
        gap: 12,
        padding: "12px 0",
        borderBottom: "1px solid var(--color-rule-light)",
      }}
    >
      <span
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 18,
          fontWeight: 600,
          color: "var(--color-garden-sage)",
          minWidth: 20,
          flexShrink: 0,
        }}
      >
        {index}
      </span>
      <div style={{ flex: 1 }}>
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            fontWeight: 600,
            textTransform: "uppercase",
            letterSpacing: "0.08em",
            color: "var(--color-text-tertiary)",
            marginBottom: 4,
          }}
        >
          {item.accountName}
          {item.estimatedValue && (
            <span style={{ marginLeft: 8, fontWeight: 400 }}>{item.estimatedValue}</span>
          )}
        </div>
        <EditableText
          value={item.opportunity}
          onChange={(v) => onUpdate({ ...item, opportunity: v })}
          multiline={false}
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 16,
            color: "var(--color-text-primary)",
          }}
        />
      </div>
      {canRemove && (
        <button
          onClick={(e) => { e.stopPropagation(); onRemove(); }}
          style={{
            opacity: hovered ? 0.6 : 0,
            transition: "opacity 0.15s",
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: "4px 6px",
            fontSize: 14,
            color: "var(--color-text-tertiary)",
            flexShrink: 0,
          }}
          aria-label="Remove"
        >
          ✕
        </button>
      )}
    </div>
  );
}
