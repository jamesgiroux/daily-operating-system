import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTauriEvent } from "./useTauriEvent";

/**
 * Workflow phase during execution
 */
export type WorkflowPhase = "preparing" | "enriching" | "delivering";

/**
 * Error information from workflow execution
 */
export interface WorkflowError {
  message: string;
  errorType: "retryable" | "nonRetryable" | "requiresUserAction";
  canRetry: boolean;
  recoverySuggestion: string;
}

/**
 * Workflow status discriminated union
 */
export type WorkflowStatus =
  | { status: "idle" }
  | {
      status: "running";
      startedAt: string;
      phase: WorkflowPhase;
      executionId: string;
    }
  | {
      status: "completed";
      finishedAt: string;
      durationSecs: number;
      executionId: string;
    }
  | {
      status: "failed";
      error: WorkflowError;
      executionId: string;
    };

/**
 * Execution history record
 */
export interface ExecutionRecord {
  id: string;
  workflow: "today" | "archive" | "week" | "inboxbatch" | "inbox_batch";
  startedAt: string;
  finishedAt?: string;
  durationSecs?: number;
  success: boolean;
  errorMessage?: string;
  errorPhase?: "preparing" | "enriching" | "delivering";
  canRetry?: boolean;
  trigger: "scheduled" | "manual" | "missed";
}

/**
 * Hook options
 */
interface UseWorkflowOptions {
  /** Workflow to monitor (default: "today") */
  workflow?: "today" | "archive" | "week" | "inbox_batch";
}

/**
 * Hook return value
 */
interface UseWorkflowReturn {
  /** Current workflow status */
  status: WorkflowStatus;
  /** Recent execution history */
  history: ExecutionRecord[];
  /** Next scheduled run time (ISO string) */
  nextRunTime: string | null;
  /** Trigger manual workflow execution */
  runNow: () => Promise<void>;
  /** Whether a run is currently in progress */
  isRunning: boolean;
  /** Refresh status from backend */
  refresh: () => Promise<void>;
}

/**
 * Hook to interact with workflow execution
 *
 * @example
 * ```tsx
 * function StatusDisplay() {
 *   const { status, runNow, isRunning } = useWorkflow();
 *
 *   return (
 *     <div>
 *       <span>Status: {status.status}</span>
 *       <button onClick={runNow} disabled={isRunning}>
 *         Run Now
 *       </button>
 *     </div>
 *   );
 * }
 * ```
 */
export function useWorkflow(options: UseWorkflowOptions = {}): UseWorkflowReturn {
  const { workflow = "today" } = options;

  const [status, setStatus] = useState<WorkflowStatus>({ status: "idle" });
  const [history, setHistory] = useState<ExecutionRecord[]>([]);
  const [nextRunTime, setNextRunTime] = useState<string | null>(null);

  // Fetch current status
  const fetchStatus = useCallback(async () => {
    try {
      const result = await invoke<WorkflowStatus>("get_workflow_status", {
        workflow,
      });
      setStatus(result);
    } catch (err) {
      console.error("Failed to fetch workflow status:", err); // Expected: background status poll
    }
  }, [workflow]);

  // Fetch execution history
  const fetchHistory = useCallback(async () => {
    try {
      const result = await invoke<ExecutionRecord[]>("get_execution_history", {
        limit: 5,
      });
      setHistory(result);
    } catch (err) {
      console.error("Failed to fetch execution history:", err); // Expected: background data fetch
    }
  }, []);

  // Fetch next run time
  const fetchNextRunTime = useCallback(async () => {
    try {
      const result = await invoke<string | null>("get_next_run_time", {
        workflow,
      });
      setNextRunTime(result);
    } catch (err) {
      console.error("Failed to fetch next run time:", err); // Expected: background data fetch
    }
  }, [workflow]);

  // Refresh all data
  const refresh = useCallback(async () => {
    await Promise.all([fetchStatus(), fetchHistory(), fetchNextRunTime()]);
  }, [fetchStatus, fetchHistory, fetchNextRunTime]);

  // Trigger manual execution
  const runNow = useCallback(async () => {
    try {
      await invoke("run_workflow", { workflow });
      // Immediately fetch status to show running state
      await fetchStatus();
    } catch (err) {
      console.error("Failed to run workflow:", err); // Re-thrown for caller to handle
      throw err;
    }
  }, [workflow, fetchStatus]);

  // Initial fetch
  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleWorkflowStatus = useCallback(
    (nextStatus: WorkflowStatus) => {
      setStatus(nextStatus);

      // Refresh history when completed or failed
      if (nextStatus.status === "completed" || nextStatus.status === "failed") {
        fetchHistory();
      }
    },
    [fetchHistory],
  );

  // Listen for status events
  useTauriEvent<WorkflowStatus>(
    `workflow-status-${workflow}`,
    handleWorkflowStatus,
  );

  // Re-fetch status when window regains focus (covers app resume from background).
  // Replaces the previous 5-second polling loop — the event listener above
  // handles real-time updates during normal operation.
  useEffect(() => {
    const onFocus = () => { void fetchStatus(); };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [fetchStatus]);

  return {
    status,
    history,
    nextRunTime,
    runNow,
    isRunning: status.status === "running",
    refresh,
  };
}

/**
 * Get a human-readable description of the current phase
 */
export function getPhaseDescription(phase: WorkflowPhase): string {
  switch (phase) {
    case "preparing":
      return "Preparing data...";
    case "enriching":
      return "Enriching with AI...";
    case "delivering":
      return "Delivering output...";
  }
}

/**
 * Format duration in seconds to human-readable string
 */
export function formatDuration(seconds: number): string {
  if (seconds < 60) {
    return `${seconds}s`;
  }
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  return `${minutes}m ${remainingSeconds}s`;
}
