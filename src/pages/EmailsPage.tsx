import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { EditorialEmpty } from "@/components/editorial/EditorialEmpty";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { getPersonalityCopy } from "@/lib/personality";
import { usePersonality } from "@/hooks/usePersonality";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";
import { EmailEntityChip } from "@/components/ui/email-entity-chip";
import { X } from "lucide-react";
import clsx from "clsx";
import s from "@/styles/editorial-briefing.module.css";
import type { EmailBriefingData, EmailSyncStats, EnrichedEmail } from "@/types";

// =============================================================================
// Self-contained so refreshing-state renders don't bubble to the whole page.
function EmailRefreshButton() {
  const [refreshing, setRefreshing] = useState(false);

  const handleRefresh = useCallback(async () => {
    setRefreshing(true);
    try {
      await invoke<string>("refresh_emails");
    } catch (err) {
      console.error("Email refresh failed:", err);
    } finally {
      setRefreshing(false);
    }
  }, []);

  return (
    <FolioRefreshButton
      onClick={handleRefresh}
      loading={refreshing}
      loadingLabel="Refreshing…"
      title={refreshing ? "Refreshing emails..." : "Check for new emails"}
    />
  );
}

// =============================================================================
// Page
// =============================================================================

export default function EmailsPage() {
  const { personality } = usePersonality();
  const [data, setData] = useState<EmailBriefingData | null>(null);
  const [syncStats, setSyncStats] = useState<EmailSyncStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [dismissed, setDismissed] = useState<Set<string>>(new Set());

  const loadEmails = useCallback(async () => {
    try {
      const [result, dismissedItems, stats] = await Promise.all([
        invoke<EmailBriefingData>("get_emails_enriched"),
        invoke<string[]>("list_dismissed_email_items").catch((err) => {
          console.error("list_dismissed_email_items failed:", err);
          return [] as string[];
        }),
        invoke<EmailSyncStats>("get_email_sync_status").catch(() => null),
      ]);
      setData(result);
      setDismissed(new Set(dismissedItems));
      setSyncStats(stats);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadEmails();
  }, [loadEmails]);

  // Silent refresh on backend email events
  const silentRefresh = useCallback(() => { loadEmails(); }, [loadEmails]);
  useTauriEvent("emails-updated", silentRefresh);
  useTauriEvent("workflow-completed", silentRefresh);

  const handleDismiss = useCallback(async (
    itemType: string,
    emailId: string,
    itemText: string,
    senderDomain?: string,
    emailType?: string,
    entityId?: string,
  ) => {
    const key = `${itemType}:${itemText}`;
    setDismissed((prev) => new Set(prev).add(key));
    try {
      await invoke("dismiss_email_item", {
        itemType,
        emailId,
        itemText,
        senderDomain: senderDomain ?? null,
        emailType: emailType ?? null,
        entityId: entityId ?? null,
      });
    } catch (err) {
      console.error("Dismiss failed:", err);
    }
  }, []);

  // Email types to exclude — operational noise, not strategic intelligence
  const NOISE_EMAIL_TYPES = new Set([
    "support", "support_ticket", "ticket", "notification",
    "marketing", "newsletter", "internal_announcement", "automated",
    "noreply", "billing", "receipt",
  ]);

  // Aggregate commitments and questions from entity-linked emails only.
  // Filters out support tickets and other noise so only strategically
  // relevant items surface in "The Correspondent."
  interface ContextualItem {
    text: string;
    emailId: string;
    sender: string;
    senderDomain?: string;
    subject: string;
    emailType?: string;
    entityName?: string;
    entityId?: string;
  }
  const { allCommitments, allQuestions } = useMemo(() => {
    if (!data) return { allCommitments: [] as ContextualItem[], allQuestions: [] as ContextualItem[] };

    // Build email-id → entity-name lookup from entity threads
    const emailEntityMap = new Map<string, string>();
    for (const thread of data.entityThreads) {
      for (const sig of thread.signals) {
        if (sig.emailId) {
          emailEntityMap.set(sig.emailId, thread.entityName);
        }
      }
    }

    const commitments: ContextualItem[] = [];
    const questions: ContextualItem[] = [];
    for (const email of [...data.highPriority, ...data.mediumPriority, ...data.lowPriority]) {
      // Skip noise email types
      if (email.emailType && NOISE_EMAIL_TYPES.has(email.emailType.toLowerCase())) continue;

      // Only include emails linked to a tracked entity, OR high-priority
      const entityName = emailEntityMap.get(email.id)
        ?? email.signals?.find((s) => s.entityId)?.entityId;
      const isEntityLinked = !!entityName;
      const isHighPriority = email.priority === "high";
      if (!isEntityLinked && !isHighPriority) continue;

      const sender = email.sender || "Unknown";
      const subject = email.subject || "";
      const senderDomain = email.senderEmail?.split("@")[1];
      const displayEntity = emailEntityMap.get(email.id);
      const entityId = email.signals?.find((sig) => sig.entityId)?.entityId;

      if (email.commitments) {
        for (const c of email.commitments) {
          if (dismissed.has(`commitment:${c}`)) continue;
          commitments.push({ text: c, emailId: email.id, sender, senderDomain, subject, emailType: email.emailType, entityName: displayEntity, entityId });
        }
      }
      if (email.questions) {
        for (const q of email.questions) {
          if (dismissed.has(`question:${q}`)) continue;
          questions.push({ text: q, emailId: email.id, sender, senderDomain, subject, emailType: email.emailType, entityName: displayEntity, entityId });
        }
      }
    }
    return { allCommitments: commitments, allQuestions: questions };
  }, [data, dismissed]);

  // I395: "Your Move" derived from scored emails, not stale directive data.
  // Top scored emails with summaries are the ones worth the user's attention.
  const yourMoveEmails = useMemo(() => {
    if (!data) return [];
    return [...data.highPriority, ...data.mediumPriority, ...data.lowPriority]
      .filter((e) => e.summary && e.summary.trim().length > 0)
      .filter((e) => (e.relevanceScore ?? 0) >= 0.15)
      .sort((a, b) => (b.relevanceScore ?? 0) - (a.relevanceScore ?? 0))
      .slice(0, 5);
  }, [data]);
  const entityThreads = data?.entityThreads ?? [];
  const riskSignalCount = useMemo(() => {
    if (!entityThreads.length) return 0;
    return entityThreads.reduce((count, t) => {
      const risks = t.signals.filter((sig) => {
        const lower = sig.signalType.toLowerCase();
        return lower.includes("risk") || lower.includes("churn") || lower.includes("escalation");
      });
      return count + risks.length;
    }, 0);
  }, [entityThreads]);

  // All emails with intelligence for the correspondence section (I395: sorted by relevance score)
  const allEnrichedEmails = useMemo(() => {
    if (!data) return [];
    return [...data.highPriority, ...data.mediumPriority, ...data.lowPriority]
      .sort((a, b) => (b.relevanceScore ?? 0) - (a.relevanceScore ?? 0));
  }, [data]);

  // I395: Group by score bands (only when scores have been computed)
  const hasScores = useMemo(() =>
    allEnrichedEmails.some(e => e.relevanceScore != null),
    [allEnrichedEmails]
  );
  const priorityEmails = useMemo(() =>
    hasScores ? allEnrichedEmails.filter(e => (e.relevanceScore ?? 0) > 0.40) : [],
    [allEnrichedEmails, hasScores]
  );
  const monitoringEmails = useMemo(() =>
    hasScores ? allEnrichedEmails.filter(e => {
      const score = e.relevanceScore ?? 0;
      return score >= 0.15 && score <= 0.40;
    }) : [],
    [allEnrichedEmails, hasScores]
  );
  const otherEmails = useMemo(() =>
    hasScores ? allEnrichedEmails.filter(e => (e.relevanceScore ?? 0) < 0.15) : [],
    [allEnrichedEmails, hasScores]
  );

  const sentimentColor = (s: string | undefined) => {
    if (s === "positive") return "var(--color-garden-sage)";
    if (s === "negative") return "var(--color-spice-terracotta)";
    return "var(--color-text-tertiary)";
  };

  const folioActions = useMemo(() => <EmailRefreshButton />, []);

  const shellConfig = useMemo(
    () => ({
      folioLabel: "The Correspondent",
      atmosphereColor: "turmeric" as const,
      activePage: "emails" as const,
      folioActions,
    }),
    [folioActions],
  );
  useRegisterMagazineShell(shellConfig);

  if (loading) return <EditorialLoading count={4} />;
  if (error) return <EditorialError message={error} onRetry={loadEmails} />;

  const isEmpty = !data || data.stats.total === 0;

  // Hero headline: AI narrative or fallback, capped at ~12 words for 76px readability
  const rawNarrative = data?.emailNarrative
    ? data.emailNarrative
    : isEmpty
      ? "Your inbox is quiet."
      : "Email triage from this morning's scan.";
  const headline = (() => {
    const words = rawNarrative.split(/\s+/);
    if (words.length <= 12) return rawNarrative;
    // Take first sentence if it's short enough, else hard cap
    const firstSentence = rawNarrative.split(/\.\s/)[0];
    if (firstSentence.split(/\s+/).length <= 12) return firstSentence + (firstSentence.endsWith(".") ? "" : ".");
    return words.slice(0, 12).join(" ") + "…";
  })();

  const hasExtracted = allCommitments.length > 0 || allQuestions.length > 0;
  const hasSignals = entityThreads.length > 0;
  const hasYourMove = yourMoveEmails.length > 0;
  const hasContent = hasYourMove || hasExtracted || hasSignals;

  return (
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
      {/* ═══ HERO ═══ */}
      <section className={s.hero}>
        <h1 className={s.heroHeadline}>{headline}</h1>

        {/* Intelligence stat line */}
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
            {priorityEmails.length > 0 && (
              <span style={{ color: "var(--color-spice-terracotta)" }}>
                {priorityEmails.length} PRIORITY
              </span>
            )}
            {monitoringEmails.length > 0 && (
              <span>
                {monitoringEmails.length} MONITORING
              </span>
            )}
            {allCommitments.length > 0 && (
              <span>{allCommitments.length} COMMITMENT{allCommitments.length !== 1 ? "S" : ""}</span>
            )}
            {riskSignalCount > 0 && (
              <span>{riskSignalCount} RISK SIGNAL{riskSignalCount !== 1 ? "S" : ""}</span>
            )}
          </div>
        )}

        {/* Sync status (I373) */}
        {syncStats && (
          <div
            style={{
              display: "flex",
              gap: 12,
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              letterSpacing: "0.04em",
              color: "var(--color-text-tertiary)",
              marginTop: 8,
            }}
          >
            {syncStats.lastFetchAt && (
              <span>
                Updated {formatRelativeTime(syncStats.lastFetchAt)}
              </span>
            )}
            {syncStats.total > 0 && (
              <span>
                {syncStats.enriched}/{syncStats.total} ready
              </span>
            )}
            {syncStats.failed > 0 && (
              <span style={{ color: "var(--color-spice-terracotta)" }}>
                {syncStats.failed} failed
              </span>
            )}
            {syncStats.total === 0 && syncStats.lastFetchAt === null && (
              <span style={{ color: "var(--color-spice-terracotta)" }}>
                using cached data
              </span>
            )}
          </div>
        )}
      </section>

      <div className={s.sectionRule} />

      {/* EMPTY STATE */}
      {isEmpty && (
        <EditorialEmpty {...getPersonalityCopy("emails-empty", personality)} />
      )}

      {/* ═══ YOUR MOVE — Top scored emails with intelligence ═══ */}
      {hasYourMove && (
        <section style={{ marginBottom: 48 }}>
          <div className={s.marginGrid}>
            <div className={s.marginLabel} style={{ color: "var(--color-spice-terracotta)" }}>
              YOUR MOVE
            </div>
            <div className={s.marginContent}>
              {yourMoveEmails.map((email) => (
                <EmailIntelItem key={email.id} email={email} dismissed={dismissed} onDismiss={handleDismiss} sentimentColor={sentimentColor} onEntityChanged={loadEmails} />
              ))}
            </div>
          </div>
          <div className={s.sectionRule} style={{ marginTop: 24 }} />
        </section>
      )}

      {/* ═══ EXTRACTED ═══ */}
      {hasExtracted && (
        <section style={{ marginBottom: 48 }}>
          {allCommitments.length > 0 && (
            <div className={s.marginGrid} style={{ marginBottom: allQuestions.length > 0 ? 28 : 0 }}>
              <div className={s.marginLabel}>COMMITMENTS</div>
              <div className={s.marginContent}>
                {allCommitments.map((c, i) => (
                  <div key={i} className="group" style={{ marginBottom: i < allCommitments.length - 1 ? 12 : 0, position: "relative" }}>
                    <p
                      style={{
                        fontFamily: "var(--font-sans)",
                        fontSize: 15,
                        fontWeight: 400,
                        lineHeight: 1.65,
                        color: "var(--color-text-primary)",
                        margin: 0,
                        maxWidth: 640,
                      }}
                    >
                      {c.text}{!c.text.endsWith(".") ? "." : ""}
                    </p>
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 11,
                        letterSpacing: "0.04em",
                        color: "var(--color-text-tertiary)",
                      }}
                    >
                      {c.entityName ? `${c.entityName} · ` : ""}{c.sender}{c.subject ? ` · ${c.subject}` : ""}
                    </span>
                    <button
                      onClick={() => handleDismiss("commitment", c.emailId, c.text, c.senderDomain, c.emailType, c.entityId)}
                      className="opacity-0 group-hover:opacity-100 transition-opacity"
                      style={{ position: "absolute", top: 0, right: 0, background: "none", border: "none", cursor: "pointer", color: "var(--color-text-tertiary)", padding: 4 }}
                      title="Dismiss"
                    >
                      <X size={14} />
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}
          {allQuestions.length > 0 && (
            <div className={s.marginGrid}>
              <div className={s.marginLabel}>OPEN QUESTIONS</div>
              <div className={s.marginContent}>
                {allQuestions.map((q, i) => (
                  <div key={i} className="group" style={{ marginBottom: i < allQuestions.length - 1 ? 12 : 0, position: "relative" }}>
                    <p
                      style={{
                        fontFamily: "var(--font-sans)",
                        fontSize: 15,
                        fontWeight: 400,
                        lineHeight: 1.65,
                        color: "var(--color-text-primary)",
                        margin: 0,
                        maxWidth: 640,
                      }}
                    >
                      {q.text}{!q.text.endsWith("?") && !q.text.endsWith(".") ? "?" : ""}
                    </p>
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 11,
                        letterSpacing: "0.04em",
                        color: "var(--color-text-tertiary)",
                      }}
                    >
                      {q.entityName ? `${q.entityName} · ` : ""}{q.sender}{q.subject ? ` · ${q.subject}` : ""}
                    </span>
                    <button
                      onClick={() => handleDismiss("question", q.emailId, q.text, q.senderDomain, q.emailType, q.entityId)}
                      className="opacity-0 group-hover:opacity-100 transition-opacity"
                      style={{ position: "absolute", top: 0, right: 0, background: "none", border: "none", cursor: "pointer", color: "var(--color-text-tertiary)", padding: 4 }}
                      title="Dismiss"
                    >
                      <X size={14} />
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}
          <div className={s.sectionRule} style={{ marginTop: 24 }} />
        </section>
      )}

      {/* ═══ SIGNALS ═══ */}
      {hasSignals && (
        <section style={{ marginBottom: 48 }}>
          <div className={s.marginGrid}>
            <div className={s.marginLabel}>SIGNALS</div>
            <div className={s.marginContent}>
              {entityThreads.slice(0, 3).map((thread, i) => (
                <div key={thread.entityId}>
                  <div
                    style={{
                      fontFamily: "var(--font-serif)",
                      fontSize: 22,
                      fontWeight: 400,
                      lineHeight: 1.3,
                      color: "var(--color-text-primary)",
                      marginBottom: 6,
                    }}
                  >
                    {thread.entityName}
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 11,
                        color: "var(--color-text-tertiary)",
                        letterSpacing: "0.04em",
                        marginLeft: 10,
                      }}
                    >
                      {thread.emailCount} email{thread.emailCount !== 1 ? "s" : ""}
                    </span>
                  </div>
                  {thread.signalSummary && (
                    <p
                      style={{
                        fontFamily: "var(--font-serif)",
                        fontSize: 16,
                        fontWeight: 300,
                        lineHeight: 1.65,
                        color: "var(--color-text-secondary)",
                        margin: "0 0 16px 0",
                        maxWidth: 640,
                      }}
                    >
                      {thread.signalSummary}
                    </p>
                  )}
                  {i < Math.min(entityThreads.length, 3) - 1 && (
                    <div className={s.sectionRule} style={{ marginBottom: 16 }} />
                  )}
                </div>
              ))}
              {entityThreads.length > 3 && (
                <div
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 14,
                    color: "var(--color-text-tertiary)",
                    marginTop: 12,
                  }}
                >
                  {entityThreads.slice(3).map((t) => t.entityName).join(", ")}
                  {" — routine correspondence."}
                </div>
              )}
            </div>
          </div>
        </section>
      )}

      {/* ═══ ALL CORRESPONDENCE — Intelligence-first email list ═══ */}
      {allEnrichedEmails.length > 0 && (
        <section style={{ marginBottom: 48 }}>
          <div className={s.marginGrid}>
            <div className={s.marginLabel}>
              INBOX
              <span className={s.marginLabelCount}>{allEnrichedEmails.length}</span>
            </div>
            <div className={s.marginContent}>
              {hasScores ? (
                <>
                  {priorityEmails.length > 0 && (
                    <>
                      <div className={s.emailScoreBandLabel}>PRIORITY</div>
                      {priorityEmails.map((email) => (
                        <EmailIntelItem key={email.id} email={email} dismissed={dismissed} onDismiss={handleDismiss} sentimentColor={sentimentColor} onEntityChanged={loadEmails} />
                      ))}
                    </>
                  )}
                  {monitoringEmails.length > 0 && (
                    <>
                      <div className={s.emailScoreBandLabel}>MONITORING</div>
                      {monitoringEmails.map((email) => (
                        <EmailIntelItem key={email.id} email={email} dismissed={dismissed} onDismiss={handleDismiss} sentimentColor={sentimentColor} onEntityChanged={loadEmails} />
                      ))}
                    </>
                  )}
                  {otherEmails.length > 0 && (
                    <>
                      <div className={s.emailScoreBandLabel}>OTHER</div>
                      {otherEmails.map((email) => (
                        <EmailIntelItem key={email.id} email={email} dismissed={dismissed} onDismiss={handleDismiss} sentimentColor={sentimentColor} onEntityChanged={loadEmails} />
                      ))}
                    </>
                  )}
                </>
              ) : (
                /* No scores yet — flat list */
                allEnrichedEmails.map((email) => (
                  <EmailIntelItem key={email.id} email={email} dismissed={dismissed} onDismiss={handleDismiss} sentimentColor={sentimentColor} onEntityChanged={loadEmails} />
                ))
              )}
            </div>
          </div>
        </section>
      )}

      {/* FINIS */}
      {hasContent && <FinisMarker />}
    </div>
  );
}

// =============================================================================
// Email Intel Item — Extracted component for grouped INBOX rendering (I395)
// =============================================================================

function EmailIntelItem({
  email,
  dismissed: _dismissed,
  onDismiss: _onDismiss,
  sentimentColor,
  onEntityChanged,
}: {
  email: EnrichedEmail;
  dismissed: Set<string>;
  onDismiss: (itemType: string, emailId: string, itemText: string, senderDomain?: string, emailType?: string, entityId?: string) => void;
  sentimentColor: (s: string | undefined) => string;
  onEntityChanged?: () => void;
}) {
  return (
    <div className={s.emailIntelItem}>
      {email.urgency === "high" && (
        <span className={s.emailIntelUrgencyBadge}>urgent</span>
      )}
      {email.summary ? (
        <p className={clsx(s.emailIntelSummary, email.urgency === "high" && s.emailIntelSummaryHighUrgency)}>
          {email.summary}
        </p>
      ) : (
        <p className={s.emailIntelBuilding}>Building context...</p>
      )}
      <div className={s.emailIntelMeta}>
        <EmailEntityChip
          entityType={email.entityType}
          entityName={email.entityName}
          editable
          emailId={email.id}
          onEntityChanged={onEntityChanged}
        />
        {email.sentiment && email.sentiment !== "neutral" && (
          <span className={s.emailIntelSentiment}>
            <span
              className={s.emailIntelSentimentDot}
              style={{ background: sentimentColor(email.sentiment) }}
            />
            {email.sentiment}
          </span>
        )}
        {/* Only show sender when it adds info beyond entity name */}
        {(!email.entityName || email.sender !== email.entityName) && (
          <span>{email.sender || email.senderEmail}</span>
        )}
        {email.subject && <span>{email.subject.length > 40 ? email.subject.slice(0, 40) + "…" : email.subject}</span>}
      </div>
      {email.scoreReason && (() => {
        const reason = email.entityName
          ? email.scoreReason.replace(email.entityName, "").replace(/^[\s·]+|[\s·]+$/g, "")
          : email.scoreReason;
        return reason ? <div className={s.emailScoreReason}>{reason}</div> : null;
      })()}
    </div>
  );
}

/** Format an ISO timestamp as relative time: "2 min ago", "1h ago", etc. */
function formatRelativeTime(isoString: string): string {
  try {
    const date = new Date(isoString);
    const diffMs = Date.now() - date.getTime();
    if (diffMs < 0) return "just now";
    const mins = Math.round(diffMs / 60000);
    if (mins < 1) return "just now";
    if (mins < 60) return `${mins} min ago`;
    const hrs = Math.round(mins / 60);
    if (hrs < 24) return `${hrs}h ago`;
    const days = Math.round(hrs / 24);
    return `${days}d ago`;
  } catch {
    return "";
  }
}
