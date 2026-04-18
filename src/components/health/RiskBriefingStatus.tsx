/**
 * RiskBriefingStatus — DOS-228 Wave 0e Fix 4.
 *
 * Renders the current risk-briefing job status at the top of the Health
 * tab. When `status === "running"` this component is pinned (callers
 * render it before the chapter sections). When `status === "failed"` it
 * exposes a retry button wired to `retry_risk_briefing`. When
 * `status === "complete"` or no job exists it renders nothing — the
 * briefing content itself lives in the reports table and is surfaced
 * elsewhere.
 */
import type { RiskBriefingJob } from "@/types";

interface RiskBriefingStatusProps {
  job: RiskBriefingJob | null;
  onRetry: () => void | Promise<void>;
}

export function RiskBriefingStatus({ job, onRetry }: RiskBriefingStatusProps) {
  if (!job) return null;

  // Don't pin anything once the briefing is ready. The briefing content is
  // rendered in its own report surface; this component is a status strip.
  if (job.status === "complete") return null;

  const labelById: Record<RiskBriefingJob["status"], string> = {
    enqueued: "Risk briefing queued",
    running: "Generating risk briefing…",
    failed: "Risk briefing failed",
    complete: "Risk briefing ready",
  };

  return (
    <div
      role="status"
      aria-live="polite"
      data-risk-briefing-status={job.status}
      style={{
        // Minimal inline styling — page CSS governs the editorial look;
        // this keeps the component usable even if the stylesheet lags.
        border: "1px solid var(--border-subtle, #d4d4d4)",
        borderRadius: 6,
        padding: "0.75rem 1rem",
        marginBottom: "1rem",
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: "1rem",
      }}
    >
      <div>
        <strong>{labelById[job.status]}</strong>
        {job.status === "failed" && job.errorMessage && (
          <div
            style={{
              fontSize: "0.85em",
              opacity: 0.75,
              marginTop: "0.25rem",
            }}
          >
            {job.errorMessage}
          </div>
        )}
        {job.status === "running" && (
          <div style={{ fontSize: "0.85em", opacity: 0.75 }}>
            Started {new Date(job.enqueuedAt).toLocaleTimeString()}
          </div>
        )}
      </div>
      {job.status === "failed" && (
        <button
          type="button"
          onClick={() => void onRetry()}
          data-action="retry-risk-briefing"
        >
          Retry
        </button>
      )}
    </div>
  );
}
