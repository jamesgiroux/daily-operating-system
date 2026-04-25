/**
 * useMe — Orchestrator hook for the /me user entity page.
 * Mirrors usePersonDetail pattern: load on mount, field editing,
 * context entry CRUD, event-driven refresh.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { UserEntity, UserContextEntry } from "@/types";
import { useTauriEvent } from "./useTauriEvent";

export function useMe() {
  const [userEntity, setUserEntity] = useState<UserEntity | null>(null);
  const [contextEntries, setContextEntries] = useState<UserContextEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // ─── Core data loading ────────────────────────────────────────────────

  const fetchData = useCallback(async (showLoading: boolean) => {
    try {
      if (showLoading) setLoading(true);
      setError(null);
      const [entity, entries] = await Promise.all([
        invoke<UserEntity>("get_user_entity"),
        invoke<UserContextEntry[]>("get_user_context_entries"),
      ]);
      setUserEntity(entity);
      setContextEntries(entries);
    } catch (e) {
      setError(String(e));
    } finally {
      if (showLoading) setLoading(false);
    }
  }, []);

  const load = useCallback(() => fetchData(true), [fetchData]);
  const silentRefresh = useCallback(() => fetchData(false), [fetchData]);

  useEffect(() => {
    load();
  }, [load]);

  // ─── Event listeners ──────────────────────────────────────────────────

  useTauriEvent("user-entity-updated", silentRefresh);

  // ─── Field editing ────────────────────────────────────────────────────

  const saveField = useCallback(async (field: string, value: string) => {
    try {
      setSaving(true);
      await invoke("update_user_entity_field", { field, value });
      // Optimistically update the local entity
      setUserEntity((prev) => {
        if (!prev) return prev;
        // Convert field name to camelCase for the local state update
        const camelField = field.replace(/_([a-z])/g, (_, c) => c.toUpperCase());
        return { ...prev, [camelField]: value || null };
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }, []);

  // ─── Context entry CRUD ───────────────────────────────────────────────

  const createEntry = useCallback(async (title: string, content: string) => {
    try {
      setSaving(true);
      const entry = await invoke<UserContextEntry>("create_user_context_entry", {
        title,
        content,
      });
      setContextEntries((prev) => [...prev, entry]);
      return entry;
    } catch (e) {
      setError(String(e));
      return null;
    } finally {
      setSaving(false);
    }
  }, []);

  const updateEntry = useCallback(async (id: string, title: string, content: string) => {
    try {
      setSaving(true);
      await invoke("update_user_context_entry", { id, title, content });
      setContextEntries((prev) =>
        prev.map((e) => (e.id === id ? { ...e, title, content } : e)),
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }, []);

  const deleteEntry = useCallback(async (id: string) => {
    try {
      setSaving(true);
      await invoke("delete_user_context_entry", { id });
      setContextEntries((prev) => prev.filter((e) => e.id !== id));
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }, []);

  return {
    userEntity,
    contextEntries,
    loading,
    saving,
    error,
    load,
    saveField,
    createEntry,
    updateEntry,
    deleteEntry,
  };
}
