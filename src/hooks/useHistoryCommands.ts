import { useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProcessingLogEntry } from "@/types";

export function useHistoryCommands() {
  const getProcessingHistory = useCallback((limit: number) => {
    return invoke<ProcessingLogEntry[]>("get_processing_history", { limit });
  }, []);

  return useMemo(() => ({
    getProcessingHistory,
  }), [getProcessingHistory]);
}
