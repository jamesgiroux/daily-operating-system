/**
 * PartnershipSlide — relationship context and voice of the customer.
 * Slide 2: relationship summary, engagement cadence, optional customer quote.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { AccountHealthContent } from "./types";

interface PartnershipSlideProps {
  content: AccountHealthContent;
  onUpdate: (c: AccountHealthContent) => void;
}

export function PartnershipSlide({ content, onUpdate }: PartnershipSlideProps) {
  const [quoteHovered, setQuoteHovered] = useState(false);

  return (
    <section
      id="partnership"
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
          color: "var(--color-text-secondary)",
          marginBottom: 24,
        }}
      >
        The Partnership
      </div>

      {/* Relationship summary — large serif paragraph */}
      <EditableText
        as="p"
        value={content.relationshipSummary}
        onChange={(v) => onUpdate({ ...content, relationshipSummary: v })}
        multiline
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 22,
          lineHeight: 1.6,
          color: "var(--color-text-primary)",
          maxWidth: 800,
          margin: "0 0 28px",
        }}
      />

      {/* Engagement cadence — mono stat line */}
      <EditableText
        as="div"
        value={content.engagementCadence}
        onChange={(v) => onUpdate({ ...content, engagementCadence: v })}
        multiline={false}
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 14,
          fontWeight: 600,
          color: "var(--color-text-secondary)",
          letterSpacing: "0.04em",
          marginBottom: 36,
        }}
      />

      {/* Customer quote */}
      {content.customerQuote != null ? (
        <div
          onMouseEnter={() => setQuoteHovered(true)}
          onMouseLeave={() => setQuoteHovered(false)}
          style={{
            position: "relative",
            maxWidth: 720,
          }}
        >
          <div
            style={{
              borderLeft: "3px solid var(--color-spice-turmeric)",
              paddingLeft: 24,
            }}
          >
            <EditableText
              as="p"
              value={content.customerQuote}
              onChange={(v) => onUpdate({ ...content, customerQuote: v })}
              multiline
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 20,
                fontStyle: "italic",
                lineHeight: 1.6,
                color: "var(--color-text-primary)",
                margin: "0 0 8px",
              }}
            />
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                color: "var(--color-text-tertiary)",
                letterSpacing: "0.04em",
              }}
            >
              — Account Contact
            </div>
          </div>

          {/* Clear quote button */}
          <button
            onClick={() => onUpdate({ ...content, customerQuote: null })}
            style={{
              position: "absolute",
              top: 0,
              right: 0,
              opacity: quoteHovered ? 0.6 : 0,
              transition: "opacity 0.15s",
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: "4px 6px",
              fontSize: 14,
              color: "var(--color-text-tertiary)",
            }}
            aria-label="Remove quote"
          >
            ✕
          </button>
        </div>
      ) : (
        /* Add quote button */
        <button
          onClick={() => onUpdate({ ...content, customerQuote: "Add a customer quote here." })}
          style={{
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: 0,
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            color: "var(--color-text-tertiary)",
            letterSpacing: "0.06em",
            opacity: 0.5,
            textAlign: "left",
          }}
          onMouseEnter={(e) => (e.currentTarget.style.opacity = "0.8")}
          onMouseLeave={(e) => (e.currentTarget.style.opacity = "0.5")}
        >
          + Add quote
        </button>
      )}
    </section>
  );
}
