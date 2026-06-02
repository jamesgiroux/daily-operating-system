import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import styles from "./EntityListShell.module.css";

export interface BulkArchivePreview {
  entityType: "account" | "project" | "person";
  requestedIds: string[];
  rootIds: string[];
  changedIds: string[];
  selectedIds: string[];
  cascadedChildIds: string[];
  coveredChildIds: string[];
  notFoundIds: string[];
  alreadyArchivedIds: string[];
  totalChangedCount: number;
  directCascadeCount: number;
  planId: string;
  planFingerprint: string;
  expiresAt: string;
}

export interface BulkArchiveResult {
  status: "succeeded" | "partial" | "preview_stale";
  preview: BulkArchivePreview;
  changedIds: string[];
  alreadyArchivedIds: string[];
  notFoundIds: string[];
  itemResults: Array<{
    entityType: "account" | "project" | "person";
    entityId: string;
    folderStatus: string;
    message?: string | null;
    originalRelativePath?: string | null;
    archivedRelativePath?: string | null;
  }>;
}

interface BulkArchiveSelectionActionProps {
  selectedIds: readonly string[];
  entityLabel: string;
  entityPluralLabel: string;
  previewCommand: string;
  executeCommand: string;
  onArchived: (result: BulkArchiveResult) => void | Promise<void>;
  onClearSelection: () => void;
}

function pluralize(count: number, singular: string, plural: string): string {
  return count === 1 ? singular : plural;
}

function formatPreviewMessage(
  preview: BulkArchivePreview,
  entityLabel: string,
  entityPluralLabel: string,
): string {
  const changedLabel = pluralize(preview.totalChangedCount, entityLabel, entityPluralLabel);
  if (preview.totalChangedCount === 0) {
    return `No active ${entityPluralLabel} will be archived.`;
  }
  if (preview.directCascadeCount === 0) {
    return `Archive ${preview.totalChangedCount} ${changedLabel}.`;
  }
  const childLabel = pluralize(preview.directCascadeCount, "descendant row", "descendant rows");
  return `Archive ${preview.totalChangedCount} ${changedLabel}, including ${preview.directCascadeCount} ${childLabel}.`;
}

export function BulkArchiveSelectionAction({
  selectedIds,
  entityLabel,
  entityPluralLabel,
  previewCommand,
  executeCommand,
  onArchived,
  onClearSelection,
}: BulkArchiveSelectionActionProps) {
  const [open, setOpen] = useState(false);
  const [preview, setPreview] = useState<BulkArchivePreview | null>(null);
  const [loadingPreview, setLoadingPreview] = useState(false);
  const [executing, setExecuting] = useState(false);

  useEffect(() => {
    if (selectedIds.length === 0) {
      setOpen(false);
      setPreview(null);
    }
  }, [selectedIds.length]);

  const previewMessage = useMemo(() => (
    preview ? formatPreviewMessage(preview, entityLabel, entityPluralLabel) : ""
  ), [entityLabel, entityPluralLabel, preview]);

  async function handlePreview() {
    if (selectedIds.length === 0 || loadingPreview) return;
    setLoadingPreview(true);
    try {
      const nextPreview = await invoke<BulkArchivePreview>(previewCommand, {
        ids: selectedIds,
      });
      setPreview(nextPreview);
      setOpen(true);
    } catch (err) {
      toast.error("Archive preview failed", {
        description: String(err),
      });
    } finally {
      setLoadingPreview(false);
    }
  }

  async function handleArchive() {
    if (!preview || executing) return;
    setExecuting(true);
    try {
      const result = await invoke<BulkArchiveResult>(executeCommand, {
        ids: preview.requestedIds,
        planId: preview.planId,
        planFingerprint: preview.planFingerprint,
      });

      if (result.status === "preview_stale") {
        toast.warning("Archive preview changed. Review again before archiving.");
        setPreview(null);
        setOpen(false);
        return;
      }

      await onArchived(result);
      onClearSelection();
      setOpen(false);
      setPreview(null);

      if (result.status === "partial") {
        const failedCount = result.itemResults.filter((item) => item.folderStatus !== "succeeded").length;
        toast.warning("Archived, but folders need attention", {
          description: failedCount > 0 ? `${failedCount} folder ${failedCount === 1 ? "result needs" : "results need"} review.` : undefined,
        });
      } else {
        const changedCount = result.changedIds.length;
        const changedLabel = pluralize(changedCount, entityLabel, entityPluralLabel);
        toast.success(`Archived ${changedCount} ${changedLabel}`);
      }
    } catch (err) {
      toast.error("Archive failed", {
        description: String(err),
      });
    } finally {
      setExecuting(false);
    }
  }

  return (
    <>
      <button
        type="button"
        className={styles.selectionButton}
        onClick={handlePreview}
        disabled={selectedIds.length === 0 || loadingPreview}
        aria-label={`Archive ${selectedIds.length} selected ${pluralize(selectedIds.length, entityLabel, entityPluralLabel)}`}
      >
        {loadingPreview ? "Previewing..." : "Archive selected"}
      </button>
      <AlertDialog open={open} onOpenChange={setOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Archive selected {entityPluralLabel}</AlertDialogTitle>
            <AlertDialogDescription>
              {previewMessage}
            </AlertDialogDescription>
          </AlertDialogHeader>
          {preview && (
            <div className={styles.archivePreviewDetails}>
              {preview.coveredChildIds.length > 0 && (
                <p>
                  {preview.coveredChildIds.length} selected child {preview.coveredChildIds.length === 1 ? "row is" : "rows are"} already covered by a selected parent.
                </p>
              )}
              {preview.alreadyArchivedIds.length > 0 && (
                <p>{preview.alreadyArchivedIds.length} already archived.</p>
              )}
              {preview.notFoundIds.length > 0 && (
                <p>{preview.notFoundIds.length} no longer found.</p>
              )}
            </div>
          )}
          <AlertDialogFooter>
            <AlertDialogCancel disabled={executing}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              disabled={executing || !preview || preview.totalChangedCount === 0}
              onClick={(event) => {
                event.preventDefault();
                void handleArchive();
              }}
            >
              {executing ? "Archiving..." : "Archive"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
