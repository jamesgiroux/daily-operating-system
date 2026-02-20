import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { EditorialEmpty } from "@/components/editorial/EditorialEmpty";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { getPersonalityCopy } from "@/lib/personality";
import { usePersonality } from "@/hooks/usePersonality";
import { RefreshCw, X } from "lucide-react";
import s from "@/styles/editorial-briefing.module.css";
import type { EmailBriefingData } from "@/types";

// =============================================================================
// Page
// =============================================================================

export default function EmailsPage() {
  const { personality } = usePersonality();
  const [data, setData] = useState<EmailBriefingData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [dismissed, setDismissed] = useState<Set<string>>(new Set());

  const loadEmails = useCallback(async () => {
    try {
      const [result, dismissedItems] = await Promise.all([
        invoke<EmailBriefingData>("get_emails_enriched"),
        invoke<string[]>("list_dismissed_email_items").catch((err) => {
          console.error("list_dismissed_email_items failed:", err);
          return [] as string[];
        }),
      ]);
      setData(result);
      setDismissed(new Set(dismissedItems));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadEmails();
  }, [loadEmails]);

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

  const handleRefresh = useCallback(async () => {
    setRefreshing(true);
    try {
      await invoke<string>("refresh_emails");
      await loadEmails();
    } catch (err) {
      console.error("Email refresh failed:", err);
    } finally {
      setRefreshing(false);
    }
  }, [loadEmails]);

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

  // Count intelligence stats for hero stat line
  const repliesNeeded = (data?.repliesNeeded ?? []).filter(
    (r) => !dismissed.has(`reply_needed:${r.subject}`),
  );
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

  const folioActions = useMemo(() => (
    <button
      onClick={handleRefresh}
      disabled={refreshing}
      className="flex items-center gap-1.5 rounded-sm px-2 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground disabled:opacity-50 disabled:cursor-not-allowed"
      title={refreshing ? "Refreshing emails..." : "Check for new emails"}
    >
      <RefreshCw size={14} className={refreshing ? "animate-spin" : ""} />
      <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, letterSpacing: "0.04em" }}>
        {refreshing ? "Refreshing..." : "Refresh"}
      </span>
    </button>
  ), [handleRefresh, refreshing]);

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
  const hasReplies = repliesNeeded.length > 0;
  const hasContent = hasReplies || hasExtracted || hasSignals;

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
            {repliesNeeded.length > 0 && (
              <span style={{ color: "var(--color-spice-terracotta)" }}>
                {repliesNeeded.length} AWAITING REPLY
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
      </section>

      <div className={s.sectionRule} />

      {/* EMPTY STATE */}
      {isEmpty && (
        <EditorialEmpty {...getPersonalityCopy("emails-empty", personality)} />
      )}

      {/* ═══ YOUR MOVE ═══ */}
      {hasReplies && (
        <section style={{ marginBottom: 48 }}>
          <div className={s.marginGrid}>
            <div className={s.marginLabel} style={{ color: "var(--color-spice-terracotta)" }}>
              YOUR MOVE
            </div>
            <div className={s.marginContent}>
              {repliesNeeded.slice(0, 3).map((reply) => (
                <div key={reply.threadId} className="group" style={{ marginBottom: 20, position: "relative" }}>
                  <div
                    style={{
                      fontFamily: "var(--font-serif)",
                      fontSize: 19,
                      fontWeight: 400,
                      lineHeight: 1.35,
                      color: "var(--color-text-primary)",
                    }}
                  >
                    {reply.subject}
                  </div>
                  <div
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 13,
                      color: "var(--color-text-tertiary)",
                      marginTop: 3,
                    }}
                  >
                    {reply.from}
                    {reply.waitDuration && (
                      <span> — waiting {reply.waitDuration}</span>
                    )}
                  </div>
                  <button
                    onClick={() => handleDismiss("reply_needed", reply.threadId, reply.subject)}
                    className="opacity-0 group-hover:opacity-100 transition-opacity"
                    style={{
                      position: "absolute",
                      top: 0,
                      right: 0,
                      background: "none",
                      border: "none",
                      cursor: "pointer",
                      color: "var(--color-text-tertiary)",
                      padding: 4,
                    }}
                    title="Dismiss"
                  >
                    <X size={14} />
                  </button>
                </div>
              ))}
              {repliesNeeded.length > 3 && (
                <div
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 14,
                    color: "var(--color-text-tertiary)",
                    marginTop: 4,
                  }}
                >
                  {repliesNeeded.slice(3).map((r) => r.from).join(", ")}
                  {" — also waiting on you."}
                </div>
              )}
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

      {/* FINIS */}
      {hasContent && <FinisMarker />}
    </div>
  );
}
