import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTauriEvent } from "./useTauriEvent";
import type { InboxFile } from "@/types";

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

  const loadFiles = useCallback(async () => {
    try {
      const result = await invoke<InboxResult>("get_inbox_files");
      if (result.status === "error") {
        setError(result.message || "Failed to load inbox");
      } else {
        setFiles(result.files);
        setCount(result.count);
        setError(null);
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

  // Listen for watcher events
  const onInboxUpdated = useCallback((update: InboxUpdate) => {
    setCount(update.count);
    loadFiles();
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
        console.error("get_inbox_files (count) failed:", err);
      });
  }, []);

  // Listen for watcher events
  const onCountUpdated = useCallback((update: InboxUpdate) => {
    setCount(update.count);
  }, []);
  useTauriEvent("inbox-updated", onCountUpdated);

  return count;
}
