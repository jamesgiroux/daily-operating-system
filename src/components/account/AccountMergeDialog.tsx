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
        console.error("get_accounts_for_picker failed:", err);
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
          <DialogTitle
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 20,
              fontWeight: 400,
            }}
          >
            Merge Account
          </DialogTitle>
          <DialogDescription
            style={{ fontFamily: "var(--font-sans)", fontSize: 13 }}
          >
            Merge <strong>{sourceAccountName}</strong> into another account.
            All actions, meetings, people, and events will be reassigned.
          </DialogDescription>
        </DialogHeader>

        {result ? (
          <div style={{ marginTop: 16 }}>
            <p
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 14,
                color: "var(--color-garden-sage)",
                marginBottom: 12,
              }}
            >
              Merge complete. {sourceAccountName} has been archived.
            </p>
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                color: "var(--color-text-tertiary)",
                display: "flex",
                flexDirection: "column",
                gap: 4,
              }}
            >
              <span>{result.actions_moved} actions moved</span>
              <span>{result.meetings_moved} meeting links updated</span>
              <span>{result.people_moved} people links updated</span>
              <span>{result.events_moved} events moved</span>
              <span>{result.children_moved} child accounts reassigned</span>
            </div>
            <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 16 }}>
              <Button
                onClick={() => {
                  onOpenChange(false);
                  onMerged();
                }}
                style={{ fontFamily: "var(--font-sans)", fontSize: 13 }}
              >
                Done
              </Button>
            </div>
          </div>
        ) : (
          <div style={{ marginTop: 16 }}>
            <label
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                fontWeight: 600,
                textTransform: "uppercase",
                letterSpacing: "0.06em",
                color: "var(--color-text-tertiary)",
                marginBottom: 4,
                display: "block",
              }}
            >
              Merge Into
            </label>
            <select
              value={targetId}
              onChange={(e) => setTargetId(e.target.value)}
              style={{
                width: "100%",
                padding: "8px 12px",
                borderRadius: 4,
                border: "1px solid var(--color-rule-light)",
                background: "var(--color-paper-warm-white)",
                fontFamily: "var(--font-sans)",
                fontSize: 14,
                color: "var(--color-text-primary)",
                outline: "none",
                height: 38,
              }}
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
              <p
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 13,
                  color: "var(--color-text-secondary)",
                  marginTop: 12,
                }}
              >
                All data from <strong>{sourceAccountName}</strong> will be moved to{" "}
                <strong>{targetName}</strong>. The source account will be archived.
              </p>
            )}

            {error && (
              <p
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 13,
                  color: "var(--color-spice-terracotta)",
                  marginTop: 8,
                }}
              >
                {error}
              </p>
            )}

            <div
              style={{
                display: "flex",
                justifyContent: "flex-end",
                gap: 8,
                marginTop: 20,
              }}
            >
              <Button
                variant="ghost"
                onClick={() => onOpenChange(false)}
                style={{ fontFamily: "var(--font-sans)", fontSize: 13 }}
              >
                Cancel
              </Button>
              <Button
                onClick={handleMerge}
                disabled={!targetId || merging}
                style={{ fontFamily: "var(--font-sans)", fontSize: 13 }}
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
