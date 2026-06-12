/**
 * useAccountSnapshotActions — write wiring for account composition snapshot edits.
 *
 * Account snapshot edits and refreshes route through Tauri commands here so
 * AccountDetailPage stays a rendering surface.
 */
import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

function readableError(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  if (error && typeof error === "object" && typeof (error as { message?: unknown }).message === "string") {
    return (error as { message: string }).message;
  }
  return "Refresh failed";
}

export function useAccountSnapshotActions(
  accountId: string | undefined,
  refetchComposition: () => Promise<void>,
) {
  const [enriching, setEnriching] = useState(false);
  const [enrichError, setEnrichError] = useState<string | null>(null);

  useEffect(() => {
    setEnrichError(null);
  }, [accountId]);

  const onSnapshotFieldSave = useCallback(
    async (field: string, value: string) => {
      if (!accountId) return;
      setEnrichError(null);
      await invoke("update_account_field", { accountId, field, value });
      await refetchComposition();
    },
    [accountId, refetchComposition],
  );

  const onEnrich = useCallback(async () => {
    if (!accountId || enriching) return;
    setEnriching(true);
    setEnrichError(null);
    try {
      await invoke("enrich_account", { accountId });
      await refetchComposition();
    } catch (error) {
      console.error("enrich_account failed:", error);
      setEnrichError(readableError(error));
    } finally {
      setEnriching(false);
    }
  }, [accountId, enriching, refetchComposition]);

  return { onSnapshotFieldSave, onEnrich, enriching, enrichError };
}
