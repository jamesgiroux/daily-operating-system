import { useState, useEffect, useCallback, useMemo, useRef, useTransition } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "@tanstack/react-router";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { EmptyState } from "@/components/editorial/EmptyState";
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
import e from "./EmailsPage.module.css";
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
  const navigate = useNavigate();
  const { personality } = usePersonality();
  const [data, setData] = useState<EmailBriefingData | null>(null);
  const [syncStats, setSyncStats] = useState<EmailSyncStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [dismissed, setDismissed] = useState<Set<string>>(new Set());
  const [dismissedSignals, setDismissedSignals] = useState<Set<number>>(new Set());
  const [, startTransition] = useTransition();
  const inboxSyncInFlight = useRef(false);

  const loadEmails = useCallback(async (silent = false) => {
    try {
      const [result, dismissedItems, stats] = await Promise.all([
        invoke<EmailBriefingData>("get_emails_enriched"),
        invoke<string[]>("list_dismissed_email_items").catch((err) => {
          console.error("list_dismissed_email_items failed:", err);
          return [] as string[];
        }),
        invoke<EmailSyncStats>("get_email_sync_status").catch(() => null),
      ]);
      const apply = () => {
        setData(result);
        setDismissed(new Set(dismissedItems));
        setSyncStats(stats);
      };
      // Silent refreshes use startTransition to avoid jarring content flashes.
      // React keeps the old content visible until the new render is ready.
      if (silent) {
        startTransition(apply);
      } else {
        apply();
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadEmails();
  }, [loadEmails]);

  const syncInboxPresence = useCallback(async () => {
    if (inboxSyncInFlight.current) return;
    inboxSyncInFlight.current = true;
    try {
      await invoke<boolean>("sync_email_inbox_presence");
    } catch (err) {
      console.debug("sync_email_inbox_presence failed:", err);
    } finally {
      inboxSyncInFlight.current = false;
    }
  }, []);

  useEffect(() => {
    void syncInboxPresence();

    const handleFocus = () => { void syncInboxPresence(); };
    const handleVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        void syncInboxPresence();
      }
    };

    window.addEventListener("focus", handleFocus);
    document.addEventListener("visibilitychange", handleVisibilityChange);

    const intervalId = window.setInterval(() => {
      if (document.visibilityState === "visible") {
        void syncInboxPresence();
      }
    }, 20000);

    return () => {
      window.removeEventListener("focus", handleFocus);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      window.clearInterval(intervalId);
    };
  }, [syncInboxPresence]);

  // Silent refresh on backend email events — uses transition to avoid blink
  const silentRefresh = useCallback(() => { loadEmails(true); }, [loadEmails]);
  useTauriEvent("emails-updated", silentRefresh);
  useTauriEvent("workflow-completed", silentRefresh);
  useTauriEvent("email-enrichment-progress", silentRefresh);

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

  const handleDismissSignal = useCallback(async (signalId: number) => {
    setDismissedSignals((prev) => new Set(prev).add(signalId));
    try {
      await invoke("dismiss_email_signal", { signalId });
    } catch (err) {
      console.error("Dismiss signal failed:", err);
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

  return (
    <div className={e.pageContainer}>
      {/* ═══ HERO ═══ */}
      <section className={s.hero}>
        <h1 className={s.heroHeadline}>{headline}</h1>

        {/* Intelligence stat line */}
        {data && data.stats.total > 0 && (
          <div className={e.heroStatLine}>
            {priorityEmails.length > 0 && (
              <span className={e.heroStatPriority}>
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
              <span>{riskSignalCount} RISK FLAG{riskSignalCount !== 1 ? "S" : ""}</span>
            )}
          </div>
        )}

        {/* Sync status (I373) */}
        {syncStats && (
          <div className={e.syncStatus}>
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
              <span className={e.syncStatusAlert}>
                {syncStats.failed} failed
              </span>
            )}
            {syncStats.total === 0 && syncStats.lastFetchAt === null && (
              <span className={e.syncStatusAlert}>
                using cached data
              </span>
            )}
          </div>
        )}
      </section>

      <div className={s.sectionRule} />

      {/* EMPTY STATE */}
      {isEmpty && (() => {
        const copy = getPersonalityCopy("emails-empty", personality);
        return (
          <EmptyState
            headline={copy.title}
            explanation={copy.explanation ?? copy.message ?? ""}
            benefit={copy.benefit}
            action={{ label: "Connect Gmail", onClick: () => navigate({ to: "/settings", search: { tab: "connectors" } }) }}
          />
        );
      })()}

      {/* ═══ YOUR MOVE — Top scored emails with intelligence ═══ */}
      {hasYourMove && (
        <section className={e.sectionSpacing}>
          <div className={s.marginGrid}>
            <div className={clsx(s.marginLabel, e.marginLabelYourMove)}>
              YOUR MOVE
            </div>
            <div className={s.marginContent}>
              {yourMoveEmails.map((email) => (
                <EmailIntelItem key={email.id} email={email} dismissed={dismissed} onDismiss={handleDismiss} sentimentColor={sentimentColor} onEntityChanged={loadEmails} />
              ))}
            </div>
          </div>
          <div className={clsx(s.sectionRule, e.sectionRuleTop)} />
        </section>
      )}

      {/* ═══ EXTRACTED ═══ */}
      {hasExtracted && (
        <section className={e.sectionSpacing}>
          {allCommitments.length > 0 && (
            <div className={clsx(s.marginGrid, allQuestions.length > 0 && e.commitmentGridSpacing)}>
              <div className={s.marginLabel}>COMMITMENTS</div>
              <div className={s.marginContent}>
                {allCommitments.map((c, i) => (
                  <div key={i} className={clsx("group", e.extractedItem, i < allCommitments.length - 1 && e.extractedItemSpacing)}>
                    <p className={e.extractedItemText}>
                      {c.text}{!c.text.endsWith(".") ? "." : ""}
                    </p>
                    <span className={e.extractedItemMeta}>
                      {c.entityName ? `${c.entityName} · ` : ""}{c.sender}{c.subject ? ` · ${c.subject}` : ""}
                    </span>
                    <button
                      onClick={() => handleDismiss("commitment", c.emailId, c.text, c.senderDomain, c.emailType, c.entityId)}
                      className={clsx("opacity-0 group-hover:opacity-100 transition-opacity", e.dismissButton)}
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
                  <div key={i} className={clsx("group", e.extractedItem, i < allQuestions.length - 1 && e.extractedItemSpacing)}>
                    <p className={e.extractedItemText}>
                      {q.text}{!q.text.endsWith("?") && !q.text.endsWith(".") ? "?" : ""}
                    </p>
                    <span className={e.extractedItemMeta}>
                      {q.entityName ? `${q.entityName} · ` : ""}{q.sender}{q.subject ? ` · ${q.subject}` : ""}
                    </span>
                    <button
                      onClick={() => handleDismiss("question", q.emailId, q.text, q.senderDomain, q.emailType, q.entityId)}
                      className={clsx("opacity-0 group-hover:opacity-100 transition-opacity", e.dismissButton)}
                      title="Dismiss"
                    >
                      <X size={14} />
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}
          <div className={clsx(s.sectionRule, e.sectionRuleTop)} />
        </section>
      )}

      {/* ═══ UPDATES ═══ */}
      {hasSignals && (() => {
        // Filter threads: keep only those with visible (non-dismissed) signals
        const liveThreads = entityThreads.filter((thread) =>
          thread.signals.some((sig) => sig.id != null && !dismissedSignals.has(sig.id))
        );
        if (liveThreads.length === 0) return null;
        const shown = liveThreads.slice(0, 3);
        const overflow = liveThreads.slice(3);
        return (
        <section className={e.sectionSpacing}>
          <div className={s.marginGrid}>
            <div className={s.marginLabel}>UPDATES</div>
            <div className={s.marginContent}>
              {shown.map((thread, i) => {
                const visibleSignals = thread.signals.filter(
                  (sig) => sig.id != null && !dismissedSignals.has(sig.id)
                );
                return (
                <div key={thread.entityId}>
                  <div className={e.updateEntityName}>
                    {thread.entityName}
                    <span className={e.updateEntityCount}>
                      {thread.emailCount} email{thread.emailCount !== 1 ? "s" : ""}
                    </span>
                  </div>
                  <div className={e.updateSignalsContainer}>
                    {visibleSignals.map((sig) => (
                      <div key={sig.id} className={e.updateSignalRow}>
                        <span className={e.updateSignalType}>
                          {sig.signalType}
                        </span>
                        <span className={e.updateSignalText}>
                          {sig.signalText}
                        </span>
                        <button
                          onClick={() => handleDismissSignal(sig.id!)}
                          className={e.dismissSignalButton}
                          title="Dismiss"
                        >
                          <X size={12} />
                        </button>
                      </div>
                    ))}
                  </div>
                  {i < shown.length - 1 && (
                    <div className={clsx(s.sectionRule, e.updateSectionRule)} />
                  )}
                </div>
                );
              })}
              {overflow.length > 0 && (
                <div className={e.overflowText}>
                  {overflow.map((t) => t.entityName).join(", ")}
                  {" — routine correspondence."}
                </div>
              )}
            </div>
          </div>
        </section>
        );
      })()}

      {/* ═══ ALL CORRESPONDENCE — Intelligence-first email list ═══ */}
      {allEnrichedEmails.length > 0 && (
        <section className={e.sectionSpacing}>
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
      <FinisMarker />
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
