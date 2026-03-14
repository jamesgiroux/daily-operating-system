/**
 * AttentionSlide — Portfolio risks and opportunities.
 * Slide 2: what needs attention, presented as editorial callouts.
 */
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
              onUpdate={(updated) => {
                const next = [...content.topRisks];
                next[i] = updated;
                onUpdate({ ...content, topRisks: next });
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
              onUpdate={(updated) => {
                const next = [...content.topOpportunities];
                next[i] = updated;
                onUpdate({ ...content, topOpportunities: next });
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
  onUpdate,
}: {
  index: number;
  item: BookRiskItem;
  onUpdate: (item: BookRiskItem) => void;
}) {
  return (
    <div
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
    </div>
  );
}

function OppItem({
  index,
  item,
  onUpdate,
}: {
  index: number;
  item: BookOpportunityItem;
  onUpdate: (item: BookOpportunityItem) => void;
}) {
  return (
    <div
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
    </div>
  );
}
