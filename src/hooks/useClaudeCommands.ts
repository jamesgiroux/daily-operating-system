import { useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";

export function useClaudeCommands() {
  const launchClaudeLogin = useCallback(() => {
    return invoke("launch_claude_login");
  }, []);

  const installClaudeCli = useCallback(() => {
    return invoke("install_claude_cli");
  }, []);

  return useMemo(() => ({
    installClaudeCli,
    launchClaudeLogin,
  }), [installClaudeCli, launchClaudeLogin]);
}
