import { useState, useEffect, useCallback, useMemo, useRef, useTransition } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";
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
import { EntityPicker } from "@/components/ui/entity-picker";
import { DatePicker } from "@/components/ui/date-picker";
import { compareEmailRank } from "@/lib/email-ranking";
import { Archive, Check, Clock, ExternalLink, Pin, X } from "lucide-react";
import { toast } from "sonner";
import clsx from "clsx";
import s from "@/styles/editorial-briefing.module.css";
import e from "./EmailsPage.module.css";
import type { EmailBriefingData, EmailSyncStats, EnrichedEmail, TrackedEmailCommitment } from "@/types";

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
      toast.error("Failed to refresh emails");
    } finally {
      setRefreshing(false);
    }
  }, []);

  return (
    <FolioRefreshButton
      onClick={handleRefresh}
      loading={refreshing}
      loadingLabel="Refreshing..."
      title={refreshing ? "Refreshing emails..." : "Check for new emails"}
    />
  );
}

function formatCadenceLabel(normalIntervalDays: number) {
  if (normalIntervalDays <= 3) return "usually every few days";
  if (normalIntervalDays <= 9) return "usually weekly";
  if (normalIntervalDays <= 18) return "usually every other week";
  return `usually every ${Math.round(normalIntervalDays)} days`;
}

function formatLastEmailDate(value?: string) {
  if (!value) return null;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return null;
  return date.toLocaleDateString("en-US", { month: "short", day: "numeric" });
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
  const [dismissedQuiet, setDismissedQuiet] = useState<Set<string>>(new Set());
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

  // I581: Dismiss gone-quiet cadence alert
  const handleDismissQuiet = useCallback(async (entityId: string) => {
    setDismissedQuiet((prev) => new Set(prev).add(entityId));
    try {
      await invoke("dismiss_gone_quiet", { entityId });
    } catch (err) {
      console.error("Dismiss gone quiet failed:", err);
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
    entityType?: "account" | "project";
    trackedCommitment?: TrackedEmailCommitment;
  }
  const { allCommitments, allQuestions } = useMemo(() => {
    if (!data) return { allCommitments: [] as ContextualItem[], allQuestions: [] as ContextualItem[] };

    // Build email-id -> entity-name lookup from entity threads
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
      const entityId = email.signals?.find((sig) => sig.entityId)?.entityId ?? email.entityId;
      const entityType = (email.entityType === "account" || email.entityType === "project")
        ? email.entityType
        : undefined;

      if (email.commitments) {
        for (const c of email.commitments) {
          if (dismissed.has(`commitment:${c}`)) continue;
          commitments.push({
            text: c,
            emailId: email.id,
            sender,
            senderDomain,
            subject,
            emailType: email.emailType,
            entityName: displayEntity,
            entityId,
            entityType,
            trackedCommitment: email.trackedCommitments?.find((tracked) => tracked.commitmentText === c),
          });
        }
      }
      if (email.questions) {
        for (const q of email.questions) {
          if (dismissed.has(`question:${q}`)) continue;
          questions.push({ text: q, emailId: email.id, sender, senderDomain, subject, emailType: email.emailType, entityName: displayEntity, entityId, entityType });
        }
      }
    }
    return { allCommitments: commitments, allQuestions: questions };
  }, [data, dismissed]);

  // I395: "Priority" derived from scored, unread, enriched emails.
  // Only show emails with intelligence (summary) that are still unread.
  const yourMoveEmails = useMemo(() => {
    if (!data) return [];
    return [...data.highPriority, ...data.mediumPriority, ...data.lowPriority]
      .filter((e) => e.summary && e.summary.trim().length > 0)
      .filter((e) => e.isUnread !== false) // only unread (or unknown)
      .filter((e) => (e.relevanceScore ?? 0) >= 0.15)
      .sort(compareEmailRank)
      .slice(0, 5);
  }, [data]);

  // I577/I578: Reply debt data is still computed by the backend for future use,
  // but the section is hidden until email classification can distinguish real
  // conversations from calendar invites/notifications.

  const [archivedIds, setArchivedIds] = useState<Set<string>>(new Set());
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

  // I581: Gone-quiet accounts — filtered by user dismissals
  const goneQuietAccounts = useMemo(() => {
    if (!data?.goneQuiet) return [];
    return data.goneQuiet.filter((a) => !dismissedQuiet.has(a.entityId));
  }, [data, dismissedQuiet]);
  const inlineGoneQuietAccounts = useMemo(
    () => goneQuietAccounts.length > 0 && goneQuietAccounts.length < 3 ? goneQuietAccounts : [],
    [goneQuietAccounts]
  );
  const standaloneGoneQuietAccounts = useMemo(
    () => goneQuietAccounts.length >= 3 ? goneQuietAccounts : [],
    [goneQuietAccounts]
  );

  // All emails with intelligence for the correspondence section (I395: sorted by relevance score)
  // Excludes locally-archived emails so archive propagates across all sections instantly.
  const allEnrichedEmails = useMemo(() => {
    if (!data) return [];
    return [...data.highPriority, ...data.mediumPriority, ...data.lowPriority]
      .filter((e) => !archivedIds.has(e.id))
      .sort(compareEmailRank);
  }, [data, archivedIds]);

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
    return words.slice(0, 12).join(" ") + "\u2026";
  })();

  const hasExtracted = allCommitments.length > 0 || allQuestions.length > 0;
  const hasSignals = entityThreads.length > 0 || inlineGoneQuietAccounts.length > 0;
  const hasYourMove = yourMoveEmails.length > 0;

  return (
    <div className={e.pageContainer}>
      {/* === HERO === */}
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

      {/* === PRIORITY -- Top scored emails with intelligence === */}
      {hasYourMove && (
        <section className={e.sectionSpacing}>
          <div className={s.marginGrid}>
            <div className={clsx(s.marginLabel, e.marginLabelYourMove)}>
              PRIORITY
            </div>
            <div className={s.marginContent}>
              {yourMoveEmails.map((email) => (
                <EmailIntelItem key={email.id} email={email} dismissed={dismissed} onDismiss={handleDismiss} sentimentColor={sentimentColor} onEntityChanged={loadEmails} onArchived={silentRefresh} archivedIds={archivedIds} setArchivedIds={setArchivedIds} />
              ))}
            </div>
          </div>
          <div className={clsx(s.sectionRule, e.sectionRuleTop)} />
        </section>
      )}

      {/* I577/I578: Reply debt section REMOVED — "user_is_last_sender = 0" doesn't
         distinguish genuine reply-debt conversations from calendar invites, newsletters,
         notifications, etc. The INBOX PRIORITY/MONITORING bands serve this purpose better.
         Reply debt will return when email classification can filter to real conversations. */}

      {/* === EXTRACTED === */}
      {hasExtracted && (
        <section className={e.sectionSpacing}>
          {allCommitments.length > 0 && (
            <div className={clsx(s.marginGrid, allQuestions.length > 0 && e.commitmentGridSpacing)}>
              <div className={s.marginLabel}>COMMITMENTS</div>
              <div className={s.marginContent}>
                {allCommitments.map((c, i) => (
                  <div key={i} className={clsx("group", e.extractedItem, i < allCommitments.length - 1 && e.extractedItemSpacing)}>
                    <p className={clsx(e.extractedItemText, c.trackedCommitment && e.extractedItemTextTracked)}>
                      {c.text}{!c.text.endsWith(".") ? "." : ""}
                    </p>
                    <span className={e.extractedItemMeta}>
                      {c.entityName ? `${c.entityName} \u00b7 ` : ""}{c.sender}{c.subject ? ` \u00b7 ${c.subject}` : ""}
                    </span>
                    <CommitmentTrackControl
                      emailId={c.emailId}
                      commitmentText={c.text}
                      defaultEntityId={c.entityId}
                      defaultEntityType={c.entityType}
                      defaultOwner={c.sender}
                      trackedCommitment={c.trackedCommitment}
                    />
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
                      {q.entityName ? `${q.entityName} \u00b7 ` : ""}{q.sender}{q.subject ? ` \u00b7 ${q.subject}` : ""}
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

      {/* === GONE QUIET -- Accounts with declining email cadence (I581) === */}
      {standaloneGoneQuietAccounts.length > 0 && (
        <section className={e.sectionSpacing}>
          <div className={s.marginGrid}>
            <div className={clsx(s.marginLabel, e.marginLabelQuiet)}>
              GONE QUIET
            </div>
            <div className={s.marginContent}>
              {standaloneGoneQuietAccounts.map((acct) => (
                <GoneQuietItem key={acct.entityId} account={acct} onDismiss={handleDismissQuiet} />
              ))}
            </div>
          </div>
          <div className={clsx(s.sectionRule, e.sectionRuleTop)} />
        </section>
      )}

      {/* === UPDATES === */}
      {hasSignals && (() => {
        // Filter threads: keep only those with visible (non-dismissed) signals
        const liveThreads = entityThreads.filter((thread) =>
          thread.signals.some((sig) => sig.id != null && !dismissedSignals.has(sig.id))
        );
        if (liveThreads.length === 0 && inlineGoneQuietAccounts.length === 0) return null;
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
                  {" -- routine correspondence."}
                </div>
              )}
              {inlineGoneQuietAccounts.length > 0 && (
                <>
                  {shown.length > 0 && <div className={clsx(s.sectionRule, e.updateSectionRule)} />}
                  {inlineGoneQuietAccounts.map((acct) => (
                    <GoneQuietItem key={acct.entityId} account={acct} onDismiss={handleDismissQuiet} />
                  ))}
                </>
              )}
            </div>
          </div>
        </section>
        );
      })()}

      {/* === ALL CORRESPONDENCE -- Intelligence-first email list === */}
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
                        <EmailIntelItem key={email.id} email={email} dismissed={dismissed} onDismiss={handleDismiss} sentimentColor={sentimentColor} onEntityChanged={loadEmails} onArchived={silentRefresh} archivedIds={archivedIds} setArchivedIds={setArchivedIds} />
                      ))}
                    </>
                  )}
                  {monitoringEmails.length > 0 && (
                    <>
                      <div className={s.emailScoreBandLabel}>MONITORING</div>
                      {monitoringEmails.map((email) => (
                        <EmailIntelItem key={email.id} email={email} dismissed={dismissed} onDismiss={handleDismiss} sentimentColor={sentimentColor} onEntityChanged={loadEmails} onArchived={silentRefresh} archivedIds={archivedIds} setArchivedIds={setArchivedIds} />
                      ))}
                    </>
                  )}
                  {otherEmails.length > 0 && (
                    <>
                      <div className={s.emailScoreBandLabel}>OTHER</div>
                      {otherEmails.map((email) => (
                        <EmailIntelItem key={email.id} email={email} dismissed={dismissed} onDismiss={handleDismiss} sentimentColor={sentimentColor} onEntityChanged={loadEmails} onArchived={silentRefresh} archivedIds={archivedIds} setArchivedIds={setArchivedIds} />
                      ))}
                    </>
                  )}
                </>
              ) : (
                /* No scores yet -- flat list */
                allEnrichedEmails.map((email) => (
                  <EmailIntelItem key={email.id} email={email} dismissed={dismissed} onDismiss={handleDismiss} sentimentColor={sentimentColor} onEntityChanged={loadEmails} onArchived={silentRefresh} archivedIds={archivedIds} setArchivedIds={setArchivedIds} />
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
// Email Intel Item -- Extracted component for grouped INBOX rendering (I395)
// With triage actions (I579), commitment promotion (I580), meeting linkage (I582)
// =============================================================================

function GoneQuietItem({
  account,
  onDismiss,
}: {
  account: NonNullable<EmailBriefingData["goneQuiet"]>[number];
  onDismiss: (entityId: string) => void;
}) {
  const navigate = useNavigate();
  const lastEmailDate = formatLastEmailDate(account.lastEmailDate);

  return (
    <div className={e.quietItem}>
      <div className={e.quietRow}>
        <div>
          <button
            type="button"
            className={e.quietAccountLink}
            onClick={() => navigate({ to: "/accounts/$accountId", params: { accountId: account.entityId } })}
          >
            {account.entityName}
          </button>
          <div className={e.quietCadenceText}>
            {formatCadenceLabel(account.normalIntervalDays)}.
            {" "}
            Last email {account.daysSinceLastEmail} day{account.daysSinceLastEmail === 1 ? "" : "s"} ago
            {lastEmailDate ? ` (${lastEmailDate})` : ""}
            {account.lastEmailSender ? ` from ${account.lastEmailSender}` : ""}.
          </div>
        </div>
        <button
          onClick={() => onDismiss(account.entityId)}
          className={e.quietDismissButton}
          title="Acknowledge"
        >
          Noted
        </button>
      </div>
    </div>
  );
}

function CommitmentTrackControl({
  emailId,
  commitmentText,
  defaultEntityId,
  defaultEntityType,
  defaultOwner,
  trackedCommitment,
  compact = false,
}: {
  emailId: string;
  commitmentText: string;
  defaultEntityId?: string;
  defaultEntityType?: "account" | "project";
  defaultOwner?: string;
  trackedCommitment?: TrackedEmailCommitment;
  compact?: boolean;
}) {
  const navigate = useNavigate();
  const [expanded, setExpanded] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [localTracked, setLocalTracked] = useState<TrackedEmailCommitment | undefined>(trackedCommitment);
  const [title, setTitle] = useState(trackedCommitment?.actionTitle ?? commitmentText);
  const [entityId, setEntityId] = useState<string | null>(defaultEntityId ?? null);
  const [entityType, setEntityType] = useState<"account" | "project" | undefined>(defaultEntityType);
  const [dueDate, setDueDate] = useState(trackedCommitment?.dueDate ?? "");
  const [owner, setOwner] = useState(trackedCommitment?.owner ?? defaultOwner ?? "");

  useEffect(() => {
    setLocalTracked(trackedCommitment);
    if (trackedCommitment) {
      setExpanded(false);
      setTitle(trackedCommitment.actionTitle ?? commitmentText);
      setDueDate(trackedCommitment.dueDate ?? "");
      setOwner(trackedCommitment.owner ?? defaultOwner ?? "");
    }
  }, [trackedCommitment, commitmentText, defaultOwner]);

  const resetForm = useCallback(() => {
    setTitle(trackedCommitment?.actionTitle ?? commitmentText);
    setEntityId(defaultEntityId ?? null);
    setEntityType(defaultEntityType);
    setDueDate(trackedCommitment?.dueDate ?? "");
    setOwner(trackedCommitment?.owner ?? defaultOwner ?? "");
  }, [commitmentText, defaultEntityId, defaultEntityType, defaultOwner, trackedCommitment]);

  const handleSubmit = useCallback(async () => {
    setSubmitting(true);
    try {
      const actionTitle = title.trim() || commitmentText;
      const actionId = await invoke<string>("promote_commitment_to_action", {
        emailId,
        commitmentText,
        actionTitle,
        entityId: entityId ?? null,
        entityType: entityType ?? null,
        owner: owner.trim() || null,
        dueDate: dueDate || null,
      });
      setLocalTracked({
        actionId,
        commitmentText,
        actionTitle,
        dueDate: dueDate || undefined,
        owner: owner.trim() || undefined,
      });
      setExpanded(false);
      toast.success("Commitment tracked as action");
    } catch (err) {
      console.error("Promote commitment failed:", err);
      toast.error("Failed to track commitment");
    } finally {
      setSubmitting(false);
    }
  }, [commitmentText, dueDate, emailId, entityId, entityType, owner, title]);

  if (localTracked) {
    return (
      <div className={clsx(e.trackedCommitment, compact && e.trackedCommitmentCompact)}>
        <span className={e.trackedCommitmentBadge}>
          <Check size={12} className={e.inlineCommitmentCheck} />
          Tracked
        </span>
        {!compact && (
          <span className={e.trackedCommitmentTitle}>{localTracked.actionTitle}</span>
        )}
        <button
          type="button"
          className={e.trackedCommitmentLink}
          onClick={() => navigate({ to: "/actions/$actionId", params: { actionId: localTracked.actionId } })}
          title="Open tracked action"
        >
          <ExternalLink size={12} />
        </button>
      </div>
    );
  }

  if (!expanded) {
    return (
      <button
        type="button"
        onClick={() => setExpanded(true)}
        className={e.trackButton}
        title="Track as action"
      >
        Track
      </button>
    );
  }

  return (
    <div className={clsx(e.commitmentForm, compact && e.commitmentFormCompact)}>
      <input
        type="text"
        value={title}
        onChange={(event) => setTitle(event.target.value)}
        className={e.commitmentFormInput}
        placeholder="Action title"
      />
      <div className={e.commitmentFormRow}>
        <EntityPicker
          value={entityId}
          onChange={(id, _name, pickedType) => {
            setEntityId(id);
            setEntityType(pickedType ?? undefined);
          }}
          placeholder="Link entity"
        />
        <DatePicker
          value={dueDate || undefined}
          onChange={setDueDate}
          placeholder="Due date"
        />
      </div>
      <input
        type="text"
        value={owner}
        onChange={(event) => setOwner(event.target.value)}
        className={e.commitmentFormInput}
        placeholder="Owner"
      />
      <div className={e.commitmentFormActions}>
        <button
          type="button"
          onClick={handleSubmit}
          disabled={submitting}
          className={e.commitmentFormSubmit}
        >
          {submitting ? "Saving..." : "Save"}
        </button>
        <button
          type="button"
          onClick={() => {
            resetForm();
            setExpanded(false);
          }}
          disabled={submitting}
          className={e.commitmentFormCancel}
        >
          Cancel
        </button>
      </div>
    </div>
  );
}

function EmailIntelItem({
  email,
  dismissed,
  onDismiss: _onDismiss,
  sentimentColor,
  onEntityChanged,
  onArchived,
  archivedIds: _archivedIds,
  setArchivedIds,
}: {
  email: EnrichedEmail;
  dismissed: Set<string>;
  onDismiss: (itemType: string, emailId: string, itemText: string, senderDomain?: string, emailType?: string, entityId?: string) => void;
  sentimentColor: (s: string | undefined) => string;
  onEntityChanged?: () => void;
  onArchived?: () => void;
  archivedIds?: Set<string>;
  setArchivedIds?: React.Dispatch<React.SetStateAction<Set<string>>>;
}) {
  const navigate = useNavigate();
  const [isPinned, setIsPinned] = useState(!!email.pinnedAt);

  useEffect(() => {
    setIsPinned(!!email.pinnedAt);
  }, [email.pinnedAt]);

  const handleArchive = useCallback(async () => {
    // Optimistic: hide immediately via local state so refresh doesn't bring it back
    setArchivedIds?.((prev) => new Set(prev).add(email.id));
    try {
      const archivedId = await invoke<string>("archive_email", { emailId: email.id });
      toast("Archived", {
        action: {
          label: "Undo",
          onClick: async () => {
            try {
              await invoke("unarchive_email", { emailId: archivedId });
              setArchivedIds?.((prev) => { const next = new Set(prev); next.delete(archivedId); return next; });
              onArchived?.();
            } catch (err) {
              console.error("Unarchive failed:", err);
            }
          },
        },
      });
      onArchived?.();
    } catch (err) {
      console.error("Archive failed:", err);
      toast.error("Failed to archive");
      setArchivedIds?.((prev) => { const next = new Set(prev); next.delete(email.id); return next; });
    }
  }, [email.id, onArchived, setArchivedIds]);

  const handleOpenInGmail = useCallback(async () => {
    try {
      await open(`https://mail.google.com/mail/u/0/#inbox/${email.id}`);
    } catch (err) {
      console.error("Open in Gmail failed:", err);
    }
  }, [email.id]);

  const handlePin = useCallback(async () => {
    try {
      const nowPinned = await invoke<boolean>("pin_email", { emailId: email.id });
      setIsPinned(nowPinned);
    } catch (err) {
      console.error("Pin failed:", err);
    }
  }, [email.id]);

  const commitments = (email.commitments ?? []).filter(
    (c) => !dismissed.has(`commitment:${c}`)
  );

  return (
    <div className={clsx(s.emailIntelItem, e.emailIntelItemHoverable)}>
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
        {email.subject && <span>{email.subject.length > 40 ? email.subject.slice(0, 40) + "\u2026" : email.subject}</span>}
      </div>
      {/* I582: Meeting linkage badge */}
      {email.meetingLinked && (
        <button
          className={e.meetingLinkBadge}
          onClick={() => navigate({ to: "/meeting/$meetingId", params: { meetingId: email.meetingLinked!.meetingId } })}
          title={`View briefing for ${email.meetingLinked.title}`}
        >
          <Clock size={12} />
          <span>Before: {email.meetingLinked.title}</span>
          <span className={e.meetingLinkTime}>{formatMeetingTime(email.meetingLinked.startTime)}</span>
        </button>
      )}
      {email.scoreReason && (() => {
        const reason = email.entityName
          ? email.scoreReason.replace(email.entityName, "").replace(/^[\s\u00b7]+|[\s\u00b7]+$/g, "")
          : email.scoreReason;
        return reason ? <div className={s.emailScoreReason}>{reason}</div> : null;
      })()}

      {/* I580: Inline commitments with Track button */}
      {commitments.length > 0 && (
        <div className={e.inlineCommitments}>
          <span className={e.inlineCommitmentsLabel}>COMMITMENTS</span>
          {commitments.map((c, i) => (
            <div key={i} className={e.inlineCommitmentRow}>
              <span
                className={clsx(
                  e.inlineCommitmentText,
                  email.trackedCommitments?.some((tracked) => tracked.commitmentText === c) && e.inlineCommitmentTextPromoted
                )}
              >
                {c}
              </span>
              <CommitmentTrackControl
                emailId={email.id}
                commitmentText={c}
                defaultEntityId={email.entityId}
                defaultEntityType={email.entityType === "account" || email.entityType === "project" ? email.entityType : undefined}
                defaultOwner={email.sender || email.senderEmail}
                trackedCommitment={email.trackedCommitments?.find((tracked) => tracked.commitmentText === c)}
                compact
              />
            </div>
          ))}
        </div>
      )}

      {/* I579: Triage action bar */}
      <div className={e.triageBar}>
        <button
          onClick={handleArchive}
          className={e.triageButton}
          title="Archive"
        >
          <Archive size={14} />
        </button>
        <button
          onClick={handleOpenInGmail}
          className={e.triageButton}
          title="Open in Gmail"
        >
          <ExternalLink size={14} />
        </button>
        <button
          onClick={handlePin}
          className={clsx(e.triageButton, isPinned && e.triageButtonActive)}
          title={isPinned ? "Unpin" : "Pin"}
        >
          <Pin size={14} />
        </button>
      </div>
    </div>
  );
}

/** Format a meeting start time as a relative label: "Tomorrow at 2pm", "Wed at 10am", etc. */
function formatMeetingTime(isoString: string): string {
  try {
    const date = new Date(isoString);
    const now = new Date();
    const diffMs = date.getTime() - now.getTime();
    const diffDays = Math.floor(diffMs / 86_400_000);
    const timeStr = date.toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit" });

    if (diffDays < 0) return timeStr;
    if (diffDays === 0) return `Today at ${timeStr}`;
    if (diffDays === 1) return `Tomorrow at ${timeStr}`;
    const dayName = date.toLocaleDateString("en-US", { weekday: "short" });
    return `${dayName} at ${timeStr}`;
  } catch {
    return "";
  }
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
