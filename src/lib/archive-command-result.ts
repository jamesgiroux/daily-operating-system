import { toast } from "sonner";

export interface ArchiveCommandItemResult {
  folderStatus: string;
}

export interface ArchiveEntityCommandResult {
  status: "succeeded" | "partial" | string;
  changedIds: string[];
  childrenRestored: number;
  itemResults: ArchiveCommandItemResult[];
}

const ATTENTION_STATUSES = new Set([
  "folder_failed",
  "metadata_failed",
  "metadata_pending",
  "missing_source",
  "folder_missing",
  "restore_conflict",
]);

export function archiveResultNeedsAttention(result: ArchiveEntityCommandResult): boolean {
  if (result.status === "partial") return true;
  return result.itemResults.some((item) => ATTENTION_STATUSES.has(item.folderStatus));
}

export function warnOnPartialArchiveResult(
  result: ArchiveEntityCommandResult,
  actionLabel: "archived" | "restored",
): void {
  if (!archiveResultNeedsAttention(result)) return;

  const folderCount = result.itemResults.filter((item) =>
    ATTENTION_STATUSES.has(item.folderStatus)
  ).length;

  toast.warning(`Record ${actionLabel}, but workspace folders need attention`, {
    description: folderCount > 0
      ? `${folderCount} folder ${folderCount === 1 ? "result needs" : "results need"} review.`
      : "Review the workspace folder repair plan in diagnostics.",
  });
}
