/**
 * useAccountFields — Edit field state, save, cancel, and dirty tracking.
 * Extracted from useAccountDetail to isolate field-editing concerns.
 */
import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AccountDetail } from "@/types";

export function useAccountFields(
  detail: AccountDetail | null,
  reload: () => Promise<void>,
  setError: (e: string | null) => void,
) {
  const [editing, setEditing] = useState(false);
  const [editName, setEditName] = useState("");
  const [editHealth, setEditHealth] = useState("");
  const [editLifecycle, setEditLifecycle] = useState("");
  const [editArr, setEditArr] = useState("");
  const [editNps, setEditNps] = useState("");
  const [editRenewal, setEditRenewal] = useState("");
  const [editNotes, setEditNotes] = useState("");
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);

  // Track the detail ID so we only auto-reset fields when the loaded entity changes,
  // not on every intelligence refresh (which updates detail but shouldn't blow away edits).
  const syncedDetailId = useRef<string | null>(null);
  useEffect(() => {
    if (!detail) return;
    if (syncedDetailId.current === detail.id) return;
    syncedDetailId.current = detail.id;
    setEditName(detail.name);
    setEditHealth(detail.health ?? "");
    setEditLifecycle(detail.lifecycle ?? "");
    setEditArr(detail.arr?.toString() ?? "");
    setEditNps(detail.nps?.toString() ?? "");
    setEditRenewal(detail.renewalDate ?? "");
    setEditNotes(detail.notes ?? "");
    setDirty(false);
  }, [detail]);

  const handleSave = useCallback(async () => {
    if (!detail) return;
    setSaving(true);
    try {
      const fieldUpdates: [string, string][] = [];
      if (editName !== detail.name) fieldUpdates.push(["name", editName]);
      if (editHealth !== (detail.health ?? "")) fieldUpdates.push(["health", editHealth]);
      if (editLifecycle !== (detail.lifecycle ?? "")) fieldUpdates.push(["lifecycle", editLifecycle]);
      if (editArr !== (detail.arr?.toString() ?? "")) fieldUpdates.push(["arr", editArr]);
      if (editNps !== (detail.nps?.toString() ?? "")) fieldUpdates.push(["nps", editNps]);
      if (editRenewal !== (detail.renewalDate ?? "")) fieldUpdates.push(["contract_end", editRenewal]);

      for (const [field, value] of fieldUpdates) {
        await invoke("update_account_field", { accountId: detail.id, field, value });
      }
      if (editNotes !== (detail.notes ?? "")) {
        await invoke("update_account_notes", { accountId: detail.id, notes: editNotes });
      }
      setDirty(false);
      setEditing(false);
      await reload();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }, [detail, editName, editHealth, editLifecycle, editArr, editNps, editRenewal, editNotes, reload, setError]);

  const handleCancelEdit = useCallback(() => {
    if (!detail) return;
    setEditName(detail.name);
    setEditHealth(detail.health ?? "");
    setEditLifecycle(detail.lifecycle ?? "");
    setEditArr(detail.arr?.toString() ?? "");
    setEditNps(detail.nps?.toString() ?? "");
    setEditRenewal(detail.renewalDate ?? "");
    setDirty(false);
    setEditing(false);
  }, [detail]);

  return {
    editing, setEditing,
    editName, setEditName,
    editHealth, setEditHealth,
    editLifecycle, setEditLifecycle,
    editArr, setEditArr,
    editNps, setEditNps,
    editRenewal, setEditRenewal,
    editNotes, setEditNotes,
    dirty, setDirty,
    saving,
    handleSave,
    handleCancelEdit,
  };
}
