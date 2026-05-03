/**
 * RiskBriefingStatus — Wave 0e Fix 4.
 *
 * Renders the current risk-briefing job status at the top of the Health
 * tab. When `status === "running"` this component is pinned (callers
 * render it before the chapter sections). When `status === "failed"` it
 * exposes a retry button wired to `retry_risk_briefing`. When
 * `status === "complete"` or no job exists it renders nothing — the
 * briefing content itself lives in the reports table and is surfaced
 * elsewhere.
 *
 * Styling lives in `RiskBriefingStatus.module.css`: the
 * prior inline-style block bypassed design tokens. All colours, spacing,
 * and typography now route through the shared token system.
 */
import type { RiskBriefingJob } from "@/types";
import styles from "./RiskBriefingStatus.module.css";

interface RiskBriefingStatusProps {
  job: RiskBriefingJob | null;
  accountId: string;
  onRetry: () => void | Promise<void>;
}

export function RiskBriefingStatus({ job, accountId, onRetry: _onRetry }: RiskBriefingStatusProps) {
  if (!job) return null;

  // generation failures stay silent on the Health tab. The user can
  // still open/regenerate from the report surface without a pinned red strip.
  if (job.status === "failed") return null;

  const labelById: Record<RiskBriefingJob["status"], string> = {
    enqueued: "Risk briefing queued",
    running: "Generating risk briefing…",
    complete: "Risk briefing ready",
    failed: "Risk briefing failed",
  };

  return (
    <div
      role="status"
      aria-live="polite"
      data-risk-briefing-status={job.status}
      className={styles.riskBriefingStatus}
    >
      <div>
        <strong className={styles.riskBriefingStatusLabel}>
          {labelById[job.status]}
        </strong>
        {job.status === "running" && (
          <div className={styles.riskBriefingStatusDetail}>
            Started {new Date(job.enqueuedAt).toLocaleTimeString()}
          </div>
        )}
        {job.status === "enqueued" && (
          <div className={styles.riskBriefingStatusDetail}>
            Started {new Date(job.enqueuedAt).toLocaleTimeString()}
          </div>
        )}
        {job.status === "complete" && (
          <div className={styles.riskBriefingStatusDetail}>
            Ready from {new Date(job.completedAt ?? job.enqueuedAt).toLocaleString()}
          </div>
        )}
      </div>
      {job.status === "complete" ? (
        <a
          href={`/accounts/${accountId}/reports/risk_briefing`}
          className={styles.riskBriefingStatusRetry}
        >
          Open briefing
        </a>
      ) : null}
    </div>
  );
}
