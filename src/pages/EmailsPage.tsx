import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Link } from "@tanstack/react-router";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { EditorialEmpty } from "@/components/editorial/EditorialEmpty";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { getPersonalityCopy } from "@/lib/personality";
import { usePersonality } from "@/hooks/usePersonality";
import { AlertTriangle, Eye, Archive, Zap } from "lucide-react";
import type { EmailBriefingData, EnrichedEmail, EntityEmailThread } from "@/types";
import type { ReadinessStat } from "@/components/layout/FolioBar";
import type { ChapterItem } from "@/components/layout/FloatingNavIsland";

// =============================================================================
// Chapters
// =============================================================================

function buildChapters(data: EmailBriefingData | null): ChapterItem[] {
  if (!data) return [];
  const chapters: ChapterItem[] = [];
  if (data.stats.highCount > 0) chapters.push({ id: "emails-attention", label: "Attention", icon: <AlertTriangle size={18} strokeWidth={1.5} /> });
  if (data.stats.mediumCount > 0) chapters.push({ id: "emails-look", label: "Worth a Look", icon: <Eye size={18} strokeWidth={1.5} /> });
  if (data.stats.lowCount > 0) chapters.push({ id: "emails-filed", label: "Filed Away", icon: <Archive size={18} strokeWidth={1.5} /> });
  if (data.entityThreads.length > 0) chapters.push({ id: "emails-signals", label: "Signals", icon: <Zap size={18} strokeWidth={1.5} /> });
  return chapters;
}

// =============================================================================
// Signal dot color by type
// =============================================================================

function signalColor(signalType: string): string {
  const lower = signalType.toLowerCase();
  if (lower.includes("risk") || lower.includes("churn") || lower.includes("escalation"))
    return "var(--color-spice-terracotta)";
  if (lower.includes("expansion") || lower.includes("upsell"))
    return "var(--color-spice-turmeric)";
  if (lower.includes("positive") || lower.includes("success") || lower.includes("win"))
    return "var(--color-garden-sage)";
  return "var(--color-text-tertiary)";
}

function priorityDotColor(priority: string): string {
  if (priority === "high") return "var(--color-spice-terracotta)";
  if (priority === "medium") return "var(--color-spice-turmeric)";
  return "var(--color-text-tertiary)";
}

// =============================================================================
// Page
// =============================================================================

export default function EmailsPage() {
  const { personality } = usePersonality();
  const [data, setData] = useState<EmailBriefingData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [archivedExpanded, setArchivedExpanded] = useState(false);
  const [archiving, setArchiving] = useState(false);
  const [confirmArchive, setConfirmArchive] = useState(false);

  const loadEmails = useCallback(async () => {
    try {
      const result = await invoke<EmailBriefingData>("get_emails_enriched");
      setData(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadEmails();
  }, [loadEmails]);

  // Archive low priority — preserved exactly
  async function handleArchiveLow() {
    setArchiving(true);
    try {
      await invoke<number>("archive_low_priority_emails");
      // Optimistic removal
      setData((prev) =>
        prev
          ? {
              ...prev,
              lowPriority: [],
              stats: { ...prev.stats, lowCount: 0, total: prev.stats.highCount + prev.stats.mediumCount },
            }
          : null,
      );
      setConfirmArchive(false);
      setArchivedExpanded(false);
    } catch (err) {
      console.error("Archive failed:", err);
    } finally {
      setArchiving(false);
    }
  }

  // Chapters for FloatingNavIsland
  const chapters = useMemo(() => buildChapters(data), [data]);

  // FolioBar readiness stats
  const folioStats = useMemo((): ReadinessStat[] => {
    if (!data) return [];
    const stats: ReadinessStat[] = [];
    if (data.stats.needsAction > 0)
      stats.push({ label: `${data.stats.needsAction} need action`, color: "terracotta" });
    return stats;
  }, [data]);

  const shellConfig = useMemo(
    () => ({
      folioLabel: "Email Intelligence",
      atmosphereColor: "turmeric" as const,
      activePage: "inbox" as const,
      chapters,
      folioReadinessStats: folioStats,
    }),
    [chapters, folioStats],
  );
  useRegisterMagazineShell(shellConfig);

  if (loading) return <EditorialLoading count={4} />;
  if (error) return <EditorialError message={error} onRetry={loadEmails} />;

  const isEmpty = !data || data.stats.total === 0;

  // Hero epigraph — auto-generated from data
  const epigraph = !data
    ? ""
    : data.hasEnrichment
      ? buildEpigraph(data)
      : "Email triage from this morning's scan.";

  return (
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
      {/* ═══ HERO ═══ */}
      <section style={{ paddingTop: 80, paddingBottom: 24 }}>
        <h1
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 36,
            fontWeight: 400,
            letterSpacing: "-0.02em",
            color: "var(--color-text-primary)",
            margin: 0,
          }}
        >
          Email Intelligence
        </h1>

        {!isEmpty && epigraph && (
          <p
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 20,
              fontStyle: "italic",
              fontWeight: 300,
              color: "var(--color-text-tertiary)",
              marginTop: 12,
              marginBottom: 0,
              maxWidth: 600,
              lineHeight: 1.5,
            }}
          >
            {epigraph}
          </p>
        )}

        {/* Heavy rule */}
        <div style={{ height: 2, background: "var(--color-desk-charcoal)", marginTop: 16, marginBottom: 16 }} />

        {/* Stat strip */}
        {data && data.stats.total > 0 && (
          <div
            style={{
              display: "flex",
              gap: 16,
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              letterSpacing: "0.06em",
              color: "var(--color-text-tertiary)",
            }}
          >
            <span>TOTAL {data.stats.total}</span>
            <span style={{ color: "var(--color-spice-terracotta)" }}>
              ACTION {data.stats.needsAction}
            </span>
            <span>HIGH {data.stats.highCount}</span>
            <span>MED {data.stats.mediumCount}</span>
            <span>LOW {data.stats.lowCount}</span>
          </div>
        )}
      </section>

      {/* ═══ EMPTY STATE ═══ */}
      {isEmpty && (
        <EditorialEmpty {...getPersonalityCopy("emails-empty", personality)} />
      )}

      {/* ═══ CHAPTER 1: NEEDS YOUR ATTENTION ═══ */}
      {data && data.highPriority.length > 0 && (
        <section id="emails-attention" style={{ marginBottom: 48 }}>
          <ChapterHeading
            title="Needs Your Attention"
            epigraph={
              data.highPriority.length === 1
                ? "One conversation requires a response today."
                : `${data.highPriority.length} conversations require a response today.`
            }
          />
          <div style={{ display: "flex", flexDirection: "column" }}>
            {data.highPriority.map((email, i) => (
              <HighPriorityCard
                key={email.id}
                email={email}
                showBorder={i < data.highPriority.length - 1}
              />
            ))}
          </div>
        </section>
      )}

      {/* ═══ CHAPTER 2: WORTH A LOOK ═══ */}
      {data && data.mediumPriority.length > 0 && (
        <section id="emails-look" style={{ marginBottom: 48 }}>
          <ChapterHeading
            title="Worth a Look"
            epigraph="These don't demand action, but staying aware pays off."
          />
          <div style={{ display: "flex", flexDirection: "column" }}>
            {data.mediumPriority.map((email, i) => (
              <MediumPriorityRow
                key={email.id}
                email={email}
                showBorder={i < data.mediumPriority.length - 1}
              />
            ))}
          </div>
        </section>
      )}

      {/* ═══ CHAPTER 3: FILED AWAY ═══ */}
      {data && data.lowPriority.length > 0 && (
        <section id="emails-filed" style={{ marginBottom: 48 }}>
          <ChapterHeading title="Filed Away" />

          {/* Collapsed summary + archive actions */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              marginBottom: 16,
            }}
          >
            <button
              onClick={() => setArchivedExpanded(!archivedExpanded)}
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 14,
                color: "var(--color-text-tertiary)",
                background: "none",
                border: "none",
                cursor: "pointer",
                padding: 0,
              }}
            >
              {data.lowPriority.length} email{data.lowPriority.length !== 1 ? "s" : ""} reviewed
              and deprioritized.
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  marginLeft: 8,
                  color: "var(--color-text-tertiary)",
                }}
              >
                {archivedExpanded ? "COLLAPSE" : "EXPAND"}
              </span>
            </button>

            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              {confirmArchive ? (
                <>
                  <span
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 12,
                      color: "var(--color-text-tertiary)",
                    }}
                  >
                    Archive {data.lowPriority.length} in Gmail?
                  </span>
                  <EditorialButton
                    label={archiving ? "Archiving..." : "Confirm"}
                    color="var(--color-spice-terracotta)"
                    onClick={handleArchiveLow}
                    disabled={archiving}
                  />
                  <EditorialButton
                    label="Cancel"
                    color="var(--color-text-tertiary)"
                    onClick={() => setConfirmArchive(false)}
                    disabled={archiving}
                  />
                </>
              ) : (
                <EditorialButton
                  label="Archive all in Gmail"
                  color="var(--color-text-tertiary)"
                  onClick={() => setConfirmArchive(true)}
                />
              )}
            </div>
          </div>

          {/* Expanded low-priority rows */}
          {archivedExpanded && (
            <div style={{ display: "flex", flexDirection: "column" }}>
              {data.lowPriority.map((email, i) => (
                <div
                  key={email.id}
                  style={{
                    padding: "8px 0",
                    borderBottom:
                      i < data.lowPriority.length - 1
                        ? "1px solid var(--color-rule-light)"
                        : "none",
                  }}
                >
                  <span
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 13,
                      color: "var(--color-text-tertiary)",
                    }}
                  >
                    {email.sender}
                  </span>
                  {email.subject && (
                    <span
                      style={{
                        fontFamily: "var(--font-sans)",
                        fontSize: 13,
                        color: "var(--color-text-tertiary)",
                        opacity: 0.7,
                        marginLeft: 8,
                      }}
                    >
                      {email.subject}
                    </span>
                  )}
                </div>
              ))}
            </div>
          )}
        </section>
      )}

      {/* ═══ CHAPTER 4: ENTITY SIGNALS ═══ */}
      {data && data.entityThreads.length > 0 && (
        <section id="emails-signals" style={{ marginBottom: 48 }}>
          <ChapterHeading
            title="Entity Signals"
            epigraph="Intelligence extracted from today's email."
          />
          <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
            {data.entityThreads.map((thread) => (
              <EntityThreadCard key={thread.entityId} thread={thread} />
            ))}
          </div>
        </section>
      )}

      {/* ═══ FINIS ═══ */}
      {data && data.stats.total > 0 && <FinisMarker />}
    </div>
  );
}

// =============================================================================
// High Priority Card — full-depth with AI summary as prose
// =============================================================================

function HighPriorityCard({
  email,
  showBorder,
}: {
  email: EnrichedEmail;
  showBorder: boolean;
}) {
  // First signal entity name for context
  const entitySignal = email.signals?.length > 0 ? email.signals[0] : null;

  return (
    <div
      style={{
        padding: "20px 0",
        borderBottom: showBorder ? "1px solid var(--color-rule-light)" : "none",
      }}
    >
      {/* Entity context line */}
      {entitySignal && (
        <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 8 }}>
          <span
            style={{
              width: 6,
              height: 6,
              borderRadius: "50%",
              background: signalColor(entitySignal.signalType),
              flexShrink: 0,
            }}
          />
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              letterSpacing: "0.06em",
              textTransform: "uppercase",
              color: "var(--color-text-tertiary)",
            }}
          >
            {entitySignal.signalType}
          </span>
        </div>
      )}

      {/* Subject as heading */}
      <h3
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 20,
          fontWeight: 400,
          color: "var(--color-text-primary)",
          margin: 0,
          lineHeight: 1.4,
        }}
      >
        {email.subject}
      </h3>

      {/* Sender */}
      <p
        style={{
          fontFamily: "var(--font-sans)",
          fontSize: 13,
          color: "var(--color-text-tertiary)",
          marginTop: 4,
          marginBottom: 0,
        }}
      >
        {email.sender}
        {email.senderEmail && (
          <span style={{ opacity: 0.6, marginLeft: 6 }}>{email.senderEmail}</span>
        )}
      </p>

      {/* AI summary — full editorial body prose */}
      {email.summary && (
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 15,
            fontWeight: 300,
            lineHeight: 1.65,
            color: "var(--color-text-secondary)",
            marginTop: 12,
            marginBottom: 0,
          }}
        >
          {email.summary}
        </p>
      )}

      {/* Fallback snippet when no enrichment */}
      {!email.summary && email.snippet && (
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            color: "var(--color-text-tertiary)",
            marginTop: 8,
            marginBottom: 0,
          }}
        >
          {email.snippet}
        </p>
      )}

      {/* Recommended action */}
      {email.recommendedAction && (
        <p
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 14,
            fontStyle: "italic",
            color: "var(--color-spice-terracotta)",
            marginTop: 10,
            marginBottom: 0,
          }}
        >
          → {email.recommendedAction}
        </p>
      )}

      {/* Conversation arc */}
      {email.conversationArc && (
        <p
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            color: "var(--color-text-tertiary)",
            marginTop: 8,
            marginBottom: 0,
            opacity: 0.7,
          }}
        >
          {email.conversationArc}
        </p>
      )}

      {/* Signal badges */}
      {email.signals?.length > 0 && (
        <div style={{ display: "flex", gap: 8, marginTop: 10, flexWrap: "wrap" }}>
          {email.signals.map((sig, i) => (
            <span
              key={i}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                letterSpacing: "0.04em",
                color: signalColor(sig.signalType),
                border: `1px solid ${signalColor(sig.signalType)}`,
                borderRadius: 3,
                padding: "1px 6px",
              }}
            >
              {sig.signalType}
              {sig.urgency && (
                <span style={{ opacity: 0.6, marginLeft: 4 }}>{sig.urgency}</span>
              )}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

// =============================================================================
// Medium Priority Row — compact
// =============================================================================

function MediumPriorityRow({
  email,
  showBorder,
}: {
  email: EnrichedEmail;
  showBorder: boolean;
}) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "flex-start",
        gap: 10,
        padding: "12px 0",
        borderBottom: showBorder ? "1px solid var(--color-rule-light)" : "none",
      }}
    >
      <span
        style={{
          width: 6,
          height: 6,
          borderRadius: "50%",
          background: priorityDotColor("medium"),
          flexShrink: 0,
          marginTop: 7,
        }}
      />
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
          <span
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              color: "var(--color-text-secondary)",
            }}
          >
            {email.sender}
          </span>
          <span
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 15,
              color: "var(--color-text-primary)",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {email.subject}
          </span>
        </div>
        {(email.summary || email.snippet) && (
          <p
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              fontWeight: 300,
              color: "var(--color-text-tertiary)",
              marginTop: 2,
              marginBottom: 0,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {email.summary || email.snippet}
          </p>
        )}
      </div>
    </div>
  );
}

// =============================================================================
// Entity Thread Card — intelligence layer
// =============================================================================

function EntityThreadCard({ thread }: { thread: EntityEmailThread }) {
  const detailLink =
    thread.entityType === "account"
      ? `/accounts/${thread.entityId}`
      : `/projects/${thread.entityId}`;

  return (
    <div>
      {/* Entity name with dot */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
        <span
          style={{
            width: 8,
            height: 8,
            borderRadius: "50%",
            background: "var(--color-spice-turmeric)",
            flexShrink: 0,
          }}
        />
        <Link
          to={detailLink}
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 18,
            fontWeight: 400,
            color: "var(--color-text-primary)",
            textDecoration: "none",
          }}
        >
          {thread.entityName}
        </Link>
      </div>

      {/* Stats line */}
      <p
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          color: "var(--color-text-tertiary)",
          letterSpacing: "0.04em",
          margin: 0,
        }}
      >
        {thread.emailCount} email{thread.emailCount !== 1 ? "s" : ""}
        {thread.signalSummary && ` · ${thread.signalSummary}`}
      </p>

      {/* Signal narrative */}
      {thread.signals.length > 0 && (
        <div style={{ marginTop: 8 }}>
          {thread.signals.slice(0, 3).map((sig, i) => (
            <p
              key={i}
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 13,
                fontWeight: 300,
                color: "var(--color-text-secondary)",
                marginTop: i > 0 ? 4 : 0,
                marginBottom: 0,
                lineHeight: 1.5,
              }}
            >
              <span style={{ color: signalColor(sig.signalType), fontWeight: 500 }}>
                {sig.signalType}:
              </span>{" "}
              {sig.signalText}
            </p>
          ))}
          {thread.signals.length > 3 && (
            <p
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                color: "var(--color-text-tertiary)",
                marginTop: 4,
              }}
            >
              + {thread.signals.length - 3} more signal{thread.signals.length - 3 !== 1 ? "s" : ""}
            </p>
          )}
        </div>
      )}
    </div>
  );
}

// =============================================================================
// Shared editorial button
// =============================================================================

function EditorialButton({
  label,
  color,
  onClick,
  disabled,
}: {
  label: string;
  color: string;
  onClick: () => void;
  disabled?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      style={{
        fontFamily: "var(--font-mono)",
        fontSize: 11,
        fontWeight: 600,
        letterSpacing: "0.06em",
        textTransform: "uppercase",
        color,
        background: "none",
        border: `1px solid ${color}`,
        borderRadius: 4,
        padding: "2px 10px",
        cursor: disabled ? "default" : "pointer",
        opacity: disabled ? 0.5 : 1,
      }}
    >
      {label}
    </button>
  );
}

// =============================================================================
// Helpers
// =============================================================================

function buildEpigraph(data: EmailBriefingData): string {
  const parts: string[] = [];
  if (data.stats.highCount > 0) {
    parts.push(
      `${data.stats.highCount} conversation${data.stats.highCount !== 1 ? "s" : ""} need${data.stats.highCount === 1 ? "s" : ""} your attention`,
    );
  }
  const fyiCount = data.stats.mediumCount + data.stats.lowCount;
  if (fyiCount > 0) {
    parts.push(`${fyiCount} ${fyiCount === 1 ? "is" : "are"} FYI only`);
  }
  if (parts.length === 0) return "";
  return parts.join(". ") + ".";
}
