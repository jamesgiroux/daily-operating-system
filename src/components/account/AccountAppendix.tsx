/**
 * AccountAppendix — Appendix section.
 * Lifecycle events, notes, files, company context, value delivered, portfolio, BUs.
 * Styled to match the editorial appendix mockup: double rule, mono labels, grid rows.
 */
import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import type {
  AccountDetail,
  AccountEvent,
  ContentFile,
  EntityIntelligence,
} from "@/types";
import {
  formatArr,
  formatFileSize,
  formatRelativeDate as formatRelativeDateShort,
  formatShortDate,
} from "@/lib/utils";

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

/* ── Document icon SVG (14x14) ───────────────────────────────────────── */

function FileIcon() {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 14 14"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      style={{ color: "var(--color-text-tertiary)", flexShrink: 0 }}
    >
      <path
        d="M3 1.5h5l3 3v8a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1v-10a1 1 0 0 1 1-1Z"
        stroke="currentColor"
        strokeWidth="1"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path
        d="M8 1.5v3h3"
        stroke="currentColor"
        strokeWidth="1"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
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
  const [filesExpanded, setFilesExpanded] = useState(false);

  const visibleFiles = filesExpanded ? files : files.slice(0, 10);
  const hasMoreFiles = files.length > 10;

  const companyContext = intelligence?.companyContext ?? detail.companyOverview;

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
      <div style={{ marginBottom: 40 }}>
        <div
          style={{
            display: "flex",
            alignItems: "baseline",
            justifyContent: "space-between",
          }}
        >
          <div style={sectionTitleStyle}>
            Files{files.length > 0 ? ` \u00B7 ${files.length}` : ""}
          </div>
          <div style={{ display: "flex", gap: 12, alignItems: "baseline" }}>
            {indexFeedback && (
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  color: "var(--color-garden-sage)",
                }}
              >
                {indexFeedback}
              </span>
            )}
            {onIndexFiles && (
              <button
                onClick={onIndexFiles}
                disabled={indexing}
                style={{
                  ...monoActionButtonStyle,
                  cursor: indexing ? "default" : "pointer",
                }}
              >
                {indexing ? "Indexing..." : "Re-index"}
              </button>
            )}
          </div>
        </div>
        {files.length > 0 ? (
          <>
            <ul style={{ listStyle: "none", margin: 0, padding: 0 }}>
              {visibleFiles.map((f, idx) => (
                <li
                  key={f.id}
                  onClick={() =>
                    invoke("reveal_in_finder", { path: f.absolutePath })
                  }
                  style={{
                    display: "grid",
                    gridTemplateColumns: "20px 1fr auto auto",
                    gap: 10,
                    padding: "7px 0",
                    borderBottom:
                      idx === visibleFiles.length - 1
                        ? "none"
                        : "1px solid var(--color-rule-light)",
                    alignItems: "center",
                    fontSize: 13,
                    cursor: "pointer",
                  }}
                >
                  <span>
                    <FileIcon />
                  </span>
                  <span
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 13,
                      color: "var(--color-text-primary)",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                  >
                    {f.filename}
                  </span>
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      color: "var(--color-text-tertiary)",
                    }}
                  >
                    {formatFileSize(f.fileSize)}
                  </span>
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      color: "var(--color-text-tertiary)",
                    }}
                  >
                    {formatRelativeDateShort(f.modifiedAt)}
                  </span>
                </li>
              ))}
            </ul>
            {hasMoreFiles && !filesExpanded && (
              <button
                onClick={() => setFilesExpanded(true)}
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 4,
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  fontWeight: 500,
                  textTransform: "uppercase",
                  letterSpacing: "0.06em",
                  color: "var(--color-text-tertiary)",
                  cursor: "pointer",
                  padding: "6px 0",
                  marginTop: 4,
                  border: "none",
                  background: "none",
                }}
              >
                +{files.length - 10} more files
              </button>
            )}
          </>
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
            No files indexed.
          </p>
        )}
      </div>

      {/* ── Company Context ───────────────────────────────────────── */}
      {companyContext && (
        <div style={{ marginBottom: 40 }}>
          <div style={sectionTitleStyle}>Company Context</div>
          {intelligence?.companyContext ? (
            <>
              {intelligence.companyContext.description && (
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
                  {intelligence.companyContext.description}
                </p>
              )}
              {intelligence.companyContext.additionalContext && (
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
                  {intelligence.companyContext.additionalContext}
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
                {intelligence.companyContext.industry && (
                  <span>Industry: {intelligence.companyContext.industry}</span>
                )}
                {intelligence.companyContext.size && (
                  <span>Size: {intelligence.companyContext.size}</span>
                )}
                {intelligence.companyContext.headquarters && (
                  <span>HQ: {intelligence.companyContext.headquarters}</span>
                )}
              </div>
            </>
          ) : detail.companyOverview ? (
            <>
              {detail.companyOverview.description && (
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
                  {detail.companyOverview.description}
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
                {detail.companyOverview.industry && (
                  <span>Industry: {detail.companyOverview.industry}</span>
                )}
                {detail.companyOverview.size && (
                  <span>Size: {detail.companyOverview.size}</span>
                )}
                {detail.companyOverview.headquarters && (
                  <span>HQ: {detail.companyOverview.headquarters}</span>
                )}
              </div>
            </>
          ) : null}
        </div>
      )}

      {/* ── Value Delivered ───────────────────────────────────────── */}
      {(intelligence?.valueDelivered?.length ?? 0) > 0 && (
        <div style={{ marginBottom: 40 }}>
          <div style={sectionTitleStyle}>
            Value Delivered {"\u00B7"} {intelligence!.valueDelivered.length}
          </div>
          <div>
            {intelligence!.valueDelivered.map((v, i) => (
              <div
                key={i}
                style={{
                  display: "grid",
                  gridTemplateColumns: "80px 1fr auto",
                  gap: 12,
                  padding: "8px 0",
                  borderBottom:
                    i === intelligence!.valueDelivered.length - 1
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
        </div>
      )}

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
                BUs
              </div>
              <div
                style={{
                  color: "var(--color-text-primary)",
                  fontWeight: 600,
                }}
              >
                {detail.parentAggregate.buCount}
              </div>
            </div>
            {detail.parentAggregate.totalArr != null && (
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
                  Total ARR
                </div>
                <div
                  style={{
                    color: "var(--color-text-primary)",
                    fontWeight: 600,
                  }}
                >
                  ${formatArr(detail.parentAggregate.totalArr)}
                </div>
              </div>
            )}
            {detail.parentAggregate.worstHealth && (
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
                  Worst Health
                </div>
                <div
                  style={{
                    color: "var(--color-text-primary)",
                    fontWeight: 600,
                    textTransform: "capitalize",
                  }}
                >
                  {detail.parentAggregate.worstHealth}
                </div>
              </div>
            )}
            {detail.parentAggregate.nearestRenewal && (
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
                  Next Renewal
                </div>
                <div
                  style={{
                    color: "var(--color-text-primary)",
                    fontWeight: 600,
                  }}
                >
                  {formatShortDate(detail.parentAggregate.nearestRenewal)}
                </div>
              </div>
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
