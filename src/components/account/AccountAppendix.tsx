/**
 * AccountAppendix — Appendix section.
 * Lifecycle events, notes, files, BUs.
 * Styled to match the editorial appendix mockup: double rule, mono labels, grid rows.
 */
import type {
  AccountDetail,
  AccountEvent,
  ContentFile,
} from "@/types";
import { formatShortDate } from "@/lib/utils";
import { FileListSection } from "@/components/entity/FileListSection";

interface AccountAppendixProps {
  detail: AccountDetail;
  events: AccountEvent[];
  files: ContentFile[];
  // Lifecycle events
  onRecordEvent?: () => void;
  // File indexing
  onIndexFiles?: () => void;
  indexing?: boolean;
  indexFeedback?: string | null;
  // Account merge
  onMerge?: () => void;
}

function formatCurrency(amount: number): string {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: 0,
  }).format(amount);
}

/* ── Shared style tokens ─────────────────────────────────────────────── */

const appendixLabelStyle: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 10,
  fontWeight: 500,
  textTransform: "uppercase",
  letterSpacing: "0.1em",
  color: "var(--color-text-tertiary)",
  marginBottom: 28,
};

const sectionTitleStyle: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 10,
  fontWeight: 500,
  textTransform: "uppercase",
  letterSpacing: "0.1em",
  color: "var(--color-text-tertiary)",
  marginBottom: 12,
};

const monoActionButtonStyle: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 10,
  fontWeight: 500,
  color: "var(--color-text-tertiary)",
  background: "none",
  border: "none",
  cursor: "pointer",
  textTransform: "uppercase",
  letterSpacing: "0.06em",
  padding: 0,
};

/* ── Sub-components ──────────────────────────────────────────────────── */

/* ── Component ───────────────────────────────────────────────────────── */

export function AccountAppendix({
  detail: _detail,
  events,
  files,
  onRecordEvent,
  onIndexFiles,
  indexing,
  indexFeedback,
  onMerge,
}: AccountAppendixProps) {
  return (
    <section
      id="appendix"
      style={{ scrollMarginTop: 60, paddingTop: 48, paddingBottom: 160 }}
    >
      {/* Double rule */}
      <hr
        style={{
          border: "none",
          borderTop: "3px double var(--color-desk-charcoal)",
          marginBottom: 32,
        }}
      />

      {/* "Reference" label */}
      <div style={appendixLabelStyle}>Reference</div>

      {/* ── Lifecycle Events ──────────────────────────────────────── */}
      <div style={{ marginBottom: 40 }}>
        <div
          style={{
            display: "flex",
            alignItems: "baseline",
            justifyContent: "space-between",
          }}
        >
          <div style={sectionTitleStyle}>
            Lifecycle{events.length > 0 ? ` \u00B7 ${events.length}` : ""}
          </div>
          {onRecordEvent && (
            <button onClick={onRecordEvent} style={monoActionButtonStyle}>
              + Record
            </button>
          )}
          {onMerge && (
            <button onClick={onMerge} style={{ ...monoActionButtonStyle, marginLeft: 12 }}>
              Merge Into...
            </button>
          )}
        </div>
        {events.length > 0 ? (
          <div>
            {events.map((event, idx) => (
              <div
                key={event.id}
                style={{
                  display: "grid",
                  gridTemplateColumns: "80px auto 1fr auto",
                  gap: 12,
                  padding: "8px 0",
                  borderBottom:
                    idx === events.length - 1
                      ? "none"
                      : "1px solid var(--color-rule-light)",
                  alignItems: "baseline",
                  fontSize: 13,
                }}
              >
                <span
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    color: "var(--color-text-tertiary)",
                  }}
                >
                  {formatShortDate(event.eventDate)}
                </span>
                <span
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 9,
                    fontWeight: 500,
                    textTransform: "uppercase",
                    letterSpacing: "0.06em",
                    padding: "2px 6px",
                    borderRadius: 2,
                    background: "var(--color-spice-saffron-10)",
                    color: "var(--color-spice-saffron)",
                  }}
                >
                  {event.eventType}
                </span>
                <span
                  style={{
                    fontFamily: "var(--font-sans)",
                    color: "var(--color-text-primary)",
                  }}
                >
                  {event.notes || event.eventType}
                </span>
                {event.arrImpact != null && (
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 11,
                      color: "var(--color-text-tertiary)",
                      textAlign: "right",
                    }}
                  >
                    {event.arrImpact >= 0 ? "+" : ""}
                    {formatCurrency(event.arrImpact)}
                  </span>
                )}
              </div>
            ))}
          </div>
        ) : (
          <p
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              color: "var(--color-text-tertiary)",
              fontStyle: "italic",
              margin: 0,
            }}
          >
            No lifecycle events recorded.
          </p>
        )}
      </div>

      {/* ── Files ─────────────────────────────────────────────────── */}
      <FileListSection
        files={files}
        onIndexFiles={onIndexFiles}
        indexing={indexing}
        indexFeedback={indexFeedback}
      />

    </section>
  );
}
