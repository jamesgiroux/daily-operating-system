/**
 * AccountAppendix — Appendix section.
 * Lifecycle events, notes, files, BUs.
 * Styled to match the editorial appendix mockup: double rule, mono labels, grid rows.
 */
import { Link } from "@tanstack/react-router";
import type {
  AccountDetail,
  AccountEvent,
  ContentFile,
} from "@/types";
import { formatArr, formatShortDate } from "@/lib/utils";
import { FileListSection } from "@/components/entity/FileListSection";

interface AccountAppendixProps {
  detail: AccountDetail;
  events: AccountEvent[];
  files: ContentFile[];
  // Notes editing
  editNotes?: string;
  setEditNotes?: (v: string) => void;
  onSaveNotes?: () => void;
  notesDirty?: boolean;
  // Lifecycle events
  onRecordEvent?: () => void;
  // File indexing
  onIndexFiles?: () => void;
  indexing?: boolean;
  indexFeedback?: string | null;
  // Child account creation
  onCreateChild?: () => void;
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
  detail,
  events,
  files,
  editNotes,
  setEditNotes,
  onSaveNotes,
  notesDirty,
  onRecordEvent,
  onIndexFiles,
  indexing,
  indexFeedback,
  onCreateChild,
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
                    background: "rgba(222, 184, 65, 0.10)",
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

      {/* ── Notes ─────────────────────────────────────────────────── */}
      <div style={{ marginBottom: 40 }}>
        <div style={sectionTitleStyle}>Notes</div>
        {setEditNotes ? (
          <div style={{ maxWidth: 580 }}>
            <textarea
              value={editNotes ?? detail.notes ?? ""}
              onChange={(e) => setEditNotes(e.target.value)}
              placeholder="Add notes about this account..."
              style={{
                width: "100%",
                minHeight: 80,
                fontFamily: "var(--font-sans)",
                fontSize: 14,
                lineHeight: 1.6,
                color: "var(--color-text-secondary)",
                background: "none",
                border: "none",
                borderBottom: "1px solid var(--color-rule-light)",
                outline: "none",
                resize: "vertical",
                padding: "4px 0",
                maxWidth: 580,
              }}
            />
            {notesDirty && onSaveNotes && (
              <button
                onClick={onSaveNotes}
                style={{
                  ...monoActionButtonStyle,
                  padding: "4px 0",
                  marginTop: 4,
                }}
              >
                Save Notes
              </button>
            )}
          </div>
        ) : detail.notes ? (
          <p
            className="appendix-notes"
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              lineHeight: 1.6,
              color: "var(--color-text-secondary)",
              maxWidth: 580,
              margin: 0,
              whiteSpace: "pre-wrap",
            }}
          >
            {detail.notes}
          </p>
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
            No notes.
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

      {/* ── Business Units ────────────────────────────────────────── */}
      {detail.children.length > 0 && (
        <div style={{ marginBottom: 40 }}>
          <div style={sectionTitleStyle}>
            Business Units {"\u00B7"} {detail.children.length}
          </div>
          <div>
            {detail.children.map((child, idx) => (
              <Link
                key={child.id}
                to="/accounts/$accountId"
                params={{ accountId: child.id }}
                style={{
                  display: "grid",
                  gridTemplateColumns: "6px 1fr auto auto",
                  gap: 12,
                  padding: "8px 0",
                  borderBottom:
                    idx === detail.children.length - 1
                      ? "none"
                      : "1px solid var(--color-rule-light)",
                  alignItems: "baseline",
                  textDecoration: "none",
                  color: "inherit",
                  fontSize: 13,
                }}
              >
                <span
                  style={{
                    width: 6,
                    height: 6,
                    borderRadius: "50%",
                    background: child.health
                      ? child.health === "green"
                        ? "var(--color-garden-sage)"
                        : child.health === "yellow"
                          ? "var(--color-spice-turmeric)"
                          : child.health === "red"
                            ? "var(--color-spice-terracotta)"
                            : "var(--color-text-tertiary)"
                      : "var(--color-text-tertiary)",
                    flexShrink: 0,
                    alignSelf: "center",
                  }}
                />
                <span
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 13,
                    fontWeight: 500,
                    color: "var(--color-text-primary)",
                  }}
                >
                  {child.name}
                </span>
                {child.arr != null && (
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 11,
                      color: "var(--color-text-tertiary)",
                      textAlign: "right",
                    }}
                  >
                    ${formatArr(child.arr)}
                  </span>
                )}
                {child.openActionCount > 0 && (
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      color: "var(--color-text-tertiary)",
                      textAlign: "right",
                    }}
                  >
                    {child.openActionCount} action
                    {child.openActionCount !== 1 ? "s" : ""}
                  </span>
                )}
              </Link>
            ))}
          </div>
          {onCreateChild && (
            <button
              onClick={onCreateChild}
              style={{
                ...monoActionButtonStyle,
                padding: "6px 0",
                marginTop: 4,
              }}
            >
              + Add Business Unit
            </button>
          )}
        </div>
      )}

      {/* Standalone child creation when no children yet */}
      {detail.children.length === 0 && onCreateChild && (
        <div style={{ marginBottom: 40 }}>
          <button
            onClick={onCreateChild}
            style={monoActionButtonStyle}
          >
            + Add Business Unit
          </button>
        </div>
      )}
    </section>
  );
}
