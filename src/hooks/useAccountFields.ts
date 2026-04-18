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
  /**
   * DOS-229 Wave 0e Fix 5: direct-apply for AccountDetailResult returns.
   * update_account_field now returns the fresh detail assembled on the
   * writer connection; the hook consumes it via this callback instead of
   * issuing a follow-up reload that hits a different pool reader.
   */
  applyDetail?: (d: AccountDetail) => void,
) {
  const [editing, setEditing] = useState(false);
  const [editName, setEditName] = useState("");
  const [editHealth, setEditHealth] = useState("");
  const [editLifecycle, setEditLifecycle] = useState("");
  const [editArr, setEditArr] = useState("");
  const [editNps, setEditNps] = useState("");
  const [editRenewal, setEditRenewal] = useState("");
  const [editParentId, setEditParentId] = useState("");
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
    setEditParentId(detail.parentId ?? "");
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
      if (editParentId !== (detail.parentId ?? "")) fieldUpdates.push(["parent_id", editParentId]);

      // DOS-229 Wave 0e Fix 5: each update_account_field returns the
      // refreshed AccountDetail from the writer connection. Apply the LAST
      // response directly — dropping the follow-up reload that hits a
      // different pool reader whose WAL snapshot can lag.
      let latest: AccountDetail | null = null;
      for (const [field, value] of fieldUpdates) {
        latest = await invoke<AccountDetail>("update_account_field", {
          accountId: detail.id,
          field,
          value,
        });
      }
      setDirty(false);
      setEditing(false);
      if (latest && applyDetail) {
        applyDetail(latest);
      } else {
        await reload();
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }, [detail, editName, editHealth, editLifecycle, editArr, editNps, editRenewal, editParentId, reload, setError, applyDetail]);

  // Save a single field immediately with the provided value.
  // Avoids the stale-state problem where setState hasn't flushed
  // before handleSave reads the field from React state.
  const saveField = useCallback(async (field: string, value: string) => {
    if (!detail) return;
    try {
      const result = await invoke<AccountDetail>("update_account_field", {
        accountId: detail.id,
        field,
        value,
      });
      if (applyDetail) {
        applyDetail(result);
      } else {
        await reload();
      }
    } catch (e) {
      setError(String(e));
    }
  }, [detail, reload, setError, applyDetail]);

  const handleCancelEdit = useCallback(() => {
    if (!detail) return;
    setEditName(detail.name);
    setEditHealth(detail.health ?? "");
    setEditLifecycle(detail.lifecycle ?? "");
    setEditArr(detail.arr?.toString() ?? "");
    setEditNps(detail.nps?.toString() ?? "");
    setEditRenewal(detail.renewalDate ?? "");
    setEditParentId(detail.parentId ?? "");
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
    editParentId, setEditParentId,
    dirty, setDirty,
    saving,
    handleSave,
    saveField,
    handleCancelEdit,
  };
}
