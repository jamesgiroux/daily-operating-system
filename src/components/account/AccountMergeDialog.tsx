/**
 * AccountMergeDialog — Dialog to merge this account into another.
 * Shows account picker, preview of what will be moved, and confirm button.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import type { PickerAccount } from "@/types";
import styles from "./AccountMergeDialog.module.css";

interface AccountMergeDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sourceAccountId: string;
  sourceAccountName: string;
  onMerged: () => void;
}

interface MergeResult {
  actions_moved: number;
  meetings_moved: number;
  people_moved: number;
  events_moved: number;
  children_moved: number;
}

export function AccountMergeDialog({
  open,
  onOpenChange,
  sourceAccountId,
  sourceAccountName,
  onMerged,
}: AccountMergeDialogProps) {
  const [accounts, setAccounts] = useState<PickerAccount[]>([]);
  const [targetId, setTargetId] = useState("");
  const [merging, setMerging] = useState(false);
  const [result, setResult] = useState<MergeResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) {
      setTargetId("");
      setResult(null);
      setError(null);
      return;
    }
    invoke<PickerAccount[]>("get_accounts_for_picker")
      .then((all) => setAccounts(all.filter((a) => a.id !== sourceAccountId)))
      .catch((err) => {
        console.error("get_accounts_for_picker failed:", err); // Expected: background data fetch on dialog open
        setAccounts([]);
      });
  }, [open, sourceAccountId]);

  const handleMerge = async () => {
    if (!targetId) return;
    setMerging(true);
    setError(null);
    try {
      const res = await invoke<MergeResult>("merge_accounts", {
        fromId: sourceAccountId,
        intoId: targetId,
      });
      setResult(res);
    } catch (e) {
      setError(String(e));
    } finally {
      setMerging(false);
    }
  };

  const targetName = accounts.find((a) => a.id === targetId)?.name ?? "";

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className={styles.title}>
            Merge Account
          </DialogTitle>
          <DialogDescription className={styles.description}>
            Merge <strong>{sourceAccountName}</strong> into another account.
            All actions, meetings, people, and events will be reassigned.
          </DialogDescription>
        </DialogHeader>

        {result ? (
          <div className={styles.body}>
            <p className={styles.successMessage}>
              Merge complete. {sourceAccountName} has been archived.
            </p>
            <div className={styles.resultStats}>
              <span>{result.actions_moved} actions moved</span>
              <span>{result.meetings_moved} meeting links updated</span>
              <span>{result.people_moved} people links updated</span>
              <span>{result.events_moved} events moved</span>
              <span>{result.children_moved} child accounts reassigned</span>
            </div>
            <div className={styles.actions}>
              <Button
                onClick={() => {
                  onOpenChange(false);
                  onMerged();
                }}
                className={styles.buttonText}
              >
                Done
              </Button>
            </div>
          </div>
        ) : (
          <div className={styles.body}>
            <label className={styles.label}>
              Merge Into
            </label>
            <select
              value={targetId}
              onChange={(e) => setTargetId(e.target.value)}
              className={styles.select}
            >
              <option value="">Select target account...</option>
              {accounts.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.name}
                  {a.parentName ? ` (${a.parentName})` : ""}
                </option>
              ))}
            </select>

            {targetId && (
              <p className={styles.previewText}>
                All data from <strong>{sourceAccountName}</strong> will be moved to{" "}
                <strong>{targetName}</strong>. The source account will be archived.
              </p>
            )}

            {error && (
              <p className={styles.errorText}>
                {error}
              </p>
            )}

            <div className={styles.formActions}>
              <Button
                variant="ghost"
                onClick={() => onOpenChange(false)}
                className={styles.buttonText}
              >
                Cancel
              </Button>
              <Button
                onClick={handleMerge}
                disabled={!targetId || merging}
                className={styles.buttonText}
              >
                {merging ? "Merging..." : "Merge"}
              </Button>
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
