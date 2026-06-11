import { useState, useEffect, useCallback, useMemo, useTransition } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTauriEvent } from "./useTauriEvent";
import type { CopyToInboxReport, InboxFile } from "@/types";

interface InboxUpdate {
  count: number;
}

interface InboxResult {
  status: "success" | "empty" | "error";
  files: InboxFile[];
  count: number;
  message?: string;
}

interface UseInboxReturn {
  count: number;
  files: InboxFile[];
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

/**
 * Hook for inbox state with live updates from the file watcher.
 *
 * Listens for `inbox-updated` Tauri events emitted by the Rust
 * file watcher. Maintains current count (for sidebar badge) and
 * full file list (for inbox page).
 */
export function useInbox(): UseInboxReturn {
  const [count, setCount] = useState(0);
  const [files, setFiles] = useState<InboxFile[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [, startTransition] = useTransition();

  const loadFiles = useCallback(async (silent = false) => {
    try {
      const result = await invoke<InboxResult>("get_inbox_files");
      if (result.status === "error") {
        setError(result.message || "Failed to load inbox");
      } else {
        const apply = () => {
          setFiles(result.files);
          setCount(result.count);
          setError(null);
        };
        if (silent) {
          startTransition(apply);
        } else {
          apply();
        }
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unknown error");
    } finally {
      setLoading(false);
    }
  }, []);

  // Initial load
  useEffect(() => {
    loadFiles();
  }, [loadFiles]);

  // Listen for watcher events — silent refresh to avoid blink
  const onInboxUpdated = useCallback((update: InboxUpdate) => {
    setCount(update.count);
    loadFiles(true);
  }, [loadFiles]);
  useTauriEvent("inbox-updated", onInboxUpdated);

  return { count, files, loading, error, refresh: loadFiles };
}

/**
 * Lightweight hook that only tracks inbox count for sidebar badge.
 * Avoids loading full file list when only count is needed.
 */
export function useInboxCount(): number {
  const [count, setCount] = useState(0);

  // Get initial count from a lightweight invoke
  useEffect(() => {
    invoke<InboxResult>("get_inbox_files")
      .then((result) => setCount(result.count))
      .catch((err) => {
        console.error("get_inbox_files (count) failed:", err); // Expected: background init on mount
      });
  }, []);

  // Listen for watcher events
  const onCountUpdated = useCallback((update: InboxUpdate) => {
    setCount(update.count);
  }, []);
  useTauriEvent("inbox-updated", onCountUpdated);

  return count;
}

export interface InboxProcessingResultPayload {
  status: "routed" | "needs_enrichment" | "needs_entity" | "error";
  classification?: string;
  destination?: string;
  message?: string;
  suggestedName?: string;
}

export interface InboxPickerAccount {
  id: string;
  name: string;
  parentName?: string;
  accountType: string;
}

export function useInboxCommands() {
  const copyToInbox = useCallback((paths: string[]) => {
    return invoke<CopyToInboxReport>("copy_to_inbox", { paths });
  }, []);

  const getInboxFileContent = useCallback((filename: string) => {
    return invoke<string>("get_inbox_file_content", { filename });
  }, []);

  const processInboxFile = useCallback((filename: string) => {
    return invoke<InboxProcessingResultPayload>("process_inbox_file", { filename });
  }, []);

  const enrichInboxFile = useCallback((filename: string, entityId: unknown) => {
    return invoke<{ status: string; message?: string }>("enrich_inbox_file", {
      filename,
      entityId,
    });
  }, []);

  const processAllInbox = useCallback(() => {
    return invoke<[string, InboxProcessingResultPayload][]>("process_all_inbox");
  }, []);

  const assignInboxEntity = useCallback((
    fileId: string,
    account: InboxPickerAccount,
    entityName: string,
  ) => {
    return invoke("assign_inbox_entity", {
      fileId,
      entityTypeSlug: "account",
      entityId: account.id,
      entityName,
      sourceTypeSlug: "inbox",
    });
  }, []);

  const getAccountsForPicker = useCallback(() => {
    return invoke<InboxPickerAccount[]>("get_accounts_for_picker");
  }, []);

  return useMemo(
    () => ({
      assignInboxEntity,
      copyToInbox,
      enrichInboxFile,
      getAccountsForPicker,
      getInboxFileContent,
      processAllInbox,
      processInboxFile,
    }),
    [
      assignInboxEntity,
      copyToInbox,
      enrichInboxFile,
      getAccountsForPicker,
      getInboxFileContent,
      processAllInbox,
      processInboxFile,
    ],
  );
}
