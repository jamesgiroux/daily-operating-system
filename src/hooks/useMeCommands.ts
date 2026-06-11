import { useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { FeatureFlags } from "@/types";

export function useMeCommands() {
  const getConfig = useCallback(() => {
    return invoke<{ role?: string }>("get_config");
  }, []);

  const getFeatureFlags = useCallback(() => {
    return invoke<FeatureFlags>("get_feature_flags");
  }, []);

  const processUserAttachment = useCallback((path: string) => {
    return invoke<string>("process_user_attachment", { path });
  }, []);

  return useMemo(() => ({
    getConfig,
    getFeatureFlags,
    processUserAttachment,
  }), [getConfig, getFeatureFlags, processUserAttachment]);
}
