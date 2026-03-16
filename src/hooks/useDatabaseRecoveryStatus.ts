import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { DatabaseRecoveryStatus } from "@/types";

const DEFAULT_STATUS: DatabaseRecoveryStatus = {
  required: false,
  reason: "",
  detail: "",
  dbPath: "",
};

export function useDatabaseRecoveryStatus() {
  const [status, setStatus] = useState<DatabaseRecoveryStatus>(DEFAULT_STATUS);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const next = await invoke<DatabaseRecoveryStatus>("get_database_recovery_status");
      setStatus(next);
    } catch {
      setStatus(DEFAULT_STATUS);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { status, loading, refresh };
}
