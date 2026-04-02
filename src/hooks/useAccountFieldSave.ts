/**
 * useAccountFieldSave — Field save, metadata save, and conflict resolution for account detail.
 *
 * Extracted from AccountDetailEditorial to keep the page component thin.
 */
import { useState, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type { AccountFieldConflictSuggestion, AccountDetail } from "@/types";
import type { VitalConflict } from "@/components/entity/EditableVitalsStrip";
import {
  formatTrackedFieldLabel,
  formatSuggestedValue,
} from "@/components/account/account-detail-utils";

interface UseAccountFieldSaveOpts {
  accountId: string | undefined;
  detail: AccountDetail | null;
  load: () => Promise<void>;
  silentRefresh: () => Promise<void>;
  setFolioSaveStatus: (s: "idle" | "saving" | "saved") => void;
}

export function useAccountFieldSave({
  accountId,
  detail,
  load,
  silentRefresh,
  setFolioSaveStatus,
}: UseAccountFieldSaveOpts) {
  const [pendingConflictField, setPendingConflictField] = useState<string | null>(null);

  const finishFolioSave = () => {
    setFolioSaveStatus("saved");
    window.setTimeout(() => setFolioSaveStatus("idle"), 2000);
  };

  const saveMetadata = async (updated: Record<string, string>) => {
    if (!accountId) return;
    setFolioSaveStatus("saving");
    try {
      await invoke("update_entity_metadata", {
        entityId: accountId, entityType: "account", metadata: JSON.stringify(updated),
      });
      finishFolioSave();
    } catch (err) {
      console.error("update_entity_metadata failed:", err);
      toast.error("Failed to save metadata");
      setFolioSaveStatus("idle");
      throw err;
    }
  };

  const saveAccountField = async (field: string, value: string) => {
    if (!detail) return;
    setFolioSaveStatus("saving");
    try {
      await invoke("update_account_field", { accountId: detail.id, field, value });
      await load();
      finishFolioSave();
    } catch (err) {
      console.error("update_account_field failed:", err);
      toast.error("Failed to save field");
      setFolioSaveStatus("idle");
    }
  };

  const fieldConflictMap = useMemo(
    () => new Map((detail?.fieldConflicts ?? []).map((item: AccountFieldConflictSuggestion) => [item.field, item])),
    [detail?.fieldConflicts],
  );

  const handleAcceptConflict = async (field: string) => {
    const conflict = fieldConflictMap.get(field);
    if (!conflict || !detail) return;
    setPendingConflictField(field);
    try {
      await invoke("accept_account_field_conflict", {
        accountId: detail.id, field: conflict.field, suggestedValue: conflict.suggestedValue,
        source: conflict.source, signalId: conflict.signalId || null,
      });
      await load();
      toast.success(`${formatTrackedFieldLabel(field)} updated`);
    } catch (err) {
      console.error("accept_account_field_conflict failed:", err);
      toast.error(`Failed to update ${formatTrackedFieldLabel(field)}`);
    } finally { setPendingConflictField(null); }
  };

  const handleDismissConflict = async (field: string) => {
    const conflict = fieldConflictMap.get(field);
    if (!conflict || !detail) return;
    setPendingConflictField(field);
    try {
      await invoke("dismiss_account_field_conflict", {
        accountId: detail.id, field: conflict.field, signalId: conflict.signalId,
        source: conflict.source, suggestedValue: conflict.suggestedValue,
      });
      await silentRefresh();
      toast.success(`${formatTrackedFieldLabel(field)} suggestion dismissed`);
    } catch (err) {
      console.error("dismiss_account_field_conflict failed:", err);
      toast.error(`Failed to dismiss ${formatTrackedFieldLabel(field)} suggestion`);
    } finally { setPendingConflictField(null); }
  };

  const conflictsForStrip = useMemo(() => new Map(
    ["lifecycle", "arr", "contract_end", "nps"].flatMap((field) => {
      const c = fieldConflictMap.get(field);
      if (!c) return [];
      const conflict: VitalConflict = {
        source: c.source, suggestedValue: formatSuggestedValue(field, c.suggestedValue),
        detectedAt: c.detectedAt, pending: pendingConflictField === field,
        onAccept: () => void handleAcceptConflict(field),
        onDismiss: () => void handleDismissConflict(field),
      };
      return [[field, conflict]];
    })
  // eslint-disable-next-line react-hooks/exhaustive-deps
  ), [fieldConflictMap, pendingConflictField]);

  return {
    saveMetadata,
    saveAccountField,
    conflictsForStrip,
  };
}
