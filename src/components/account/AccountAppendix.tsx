/**
 * AccountAppendix — Appendix section.
 * Lifecycle events, notes, files, company context, value delivered, portfolio, BUs.
 * Styled to match the editorial appendix mockup: double rule, mono labels, grid rows.
 */
import { useState } from "react";
import { Link } from "@tanstack/react-router";
import type {
  AccountDetail,
  AccountEvent,
  ContentFile,
  EntityIntelligence,
} from "@/types";
import { formatArr, formatShortDate } from "@/lib/utils";
import { FileListSection } from "@/components/entity/FileListSection";

interface AccountAppendixProps {
  detail: AccountDetail;
  intelligence: EntityIntelligence | null;
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

/** Stat cell used in Portfolio Summary (label + value). */
function PortfolioStat({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <div
        style={{
          color: "var(--color-text-tertiary)",
          fontSize: 10,
          textTransform: "uppercase",
          letterSpacing: "0.1em",
          marginBottom: 2,
        }}
      >
        {label}
      </div>
      <div style={{ color: "var(--color-text-primary)", fontWeight: 600 }}>
        {children}
      </div>
    </div>
  );
}

/** Normalized company context display — handles both intelligence and dashboard sources. */
function CompanyContextBlock({
  description,
  additionalContext,
  industry,
  size,
  headquarters,
}: {
  description?: string;
  additionalContext?: string;
  industry?: string;
  size?: string;
  headquarters?: string;
}) {
  return (
    <>
      {description && (
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            lineHeight: 1.6,
            color: "var(--color-text-secondary)",
            maxWidth: 580,
            margin: "0 0 8px",
          }}
        >
          {description}
        </p>
      )}
      {additionalContext && (
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 13,
            lineHeight: 1.5,
            color: "var(--color-text-secondary)",
            maxWidth: 580,
            margin: "0 0 8px",
          }}
        >
          {additionalContext}
        </p>
      )}
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          gap: 16,
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          color: "var(--color-text-tertiary)",
        }}
      >
        {industry && <span>Industry: {industry}</span>}
        {size && <span>Size: {size}</span>}
        {headquarters && <span>HQ: {headquarters}</span>}
      </div>
    </>
  );
}

/* ── Component ───────────────────────────────────────────────────────── */

export function AccountAppendix({
  detail,
  intelligence,
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
}: AccountAppendixProps) {
  const companyContext = intelligence?.companyContext ?? detail.companyOverview;
  const [expandedValue, setExpandedValue] = useState(false);
  const VALUE_LIMIT = 3;

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

      {/* ── Company Context ───────────────────────────────────────── */}
      {companyContext && (
        <div style={{ marginBottom: 40 }}>
          <div style={sectionTitleStyle}>Company Context</div>
          {intelligence?.companyContext ? (
            <CompanyContextBlock
              description={intelligence.companyContext.description}
              additionalContext={intelligence.companyContext.additionalContext}
              industry={intelligence.companyContext.industry}
              size={intelligence.companyContext.size}
              headquarters={intelligence.companyContext.headquarters}
            />
          ) : detail.companyOverview ? (
            <CompanyContextBlock
              description={detail.companyOverview.description}
              industry={detail.companyOverview.industry}
              size={detail.companyOverview.size}
              headquarters={detail.companyOverview.headquarters}
            />
          ) : null}
        </div>
      )}

      {/* ── Value Delivered ───────────────────────────────────────── */}
      {(intelligence?.valueDelivered?.length ?? 0) > 0 && (() => {
        const allValue = intelligence!.valueDelivered;
        const visibleValue = expandedValue ? allValue : allValue.slice(0, VALUE_LIMIT);
        const hasMoreValue = allValue.length > VALUE_LIMIT && !expandedValue;
        return (
          <div style={{ marginBottom: 40 }}>
            <div style={sectionTitleStyle}>
              Value Delivered {"\u00B7"} {allValue.length}
            </div>
            <div>
              {visibleValue.map((v, i) => (
                <div
                  key={i}
                  style={{
                    display: "grid",
                    gridTemplateColumns: "80px 1fr auto",
                    gap: 12,
                    padding: "8px 0",
                    borderBottom:
                      i === visibleValue.length - 1 && !hasMoreValue
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
                    {v.date || ""}
                  </span>
                  <span
                    style={{
                      fontFamily: "var(--font-sans)",
                      color: "var(--color-text-primary)",
                    }}
                  >
                    {v.statement}
                  </span>
                  {v.source && (
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 10,
                        color: "var(--color-text-tertiary)",
                        textAlign: "right",
                      }}
                    >
                      {v.source}
                    </span>
                  )}
                </div>
              ))}
            </div>
            {hasMoreValue && (
              <button
                onClick={() => setExpandedValue(true)}
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  color: "var(--color-text-tertiary)",
                  background: "none",
                  border: "none",
                  cursor: "pointer",
                  padding: "8px 0 0",
                  textTransform: "uppercase",
                  letterSpacing: "0.06em",
                }}
              >
                Show {allValue.length - VALUE_LIMIT} more
              </button>
            )}
          </div>
        );
      })()}

      {/* ── Portfolio Summary (parent accounts) ───────────────────── */}
      {detail.parentAggregate && (
        <div style={{ marginBottom: 40 }}>
          <div style={sectionTitleStyle}>Portfolio Summary</div>
          <div
            style={{
              display: "flex",
              flexWrap: "wrap",
              gap: 24,
              fontFamily: "var(--font-mono)",
              fontSize: 13,
            }}
          >
            <PortfolioStat label="BUs">{detail.parentAggregate.buCount}</PortfolioStat>
            {detail.parentAggregate.totalArr != null && (
              <PortfolioStat label="Total ARR">${formatArr(detail.parentAggregate.totalArr)}</PortfolioStat>
            )}
            {detail.parentAggregate.worstHealth && (
              <PortfolioStat label="Worst Health">
                <span style={{ textTransform: "capitalize" }}>{detail.parentAggregate.worstHealth}</span>
              </PortfolioStat>
            )}
            {detail.parentAggregate.nearestRenewal && (
              <PortfolioStat label="Next Renewal">{formatShortDate(detail.parentAggregate.nearestRenewal)}</PortfolioStat>
            )}
          </div>
        </div>
      )}

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
