import { useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ActionDetail, LinearPushResult } from "@/types";

interface LinearStatus {
  enabled: boolean;
  apiKeySet: boolean;
}

interface LinearTeam {
  id: string;
  name: string;
}

// Type alias (not interface) so it satisfies InvokeArgs' implicit index signature.
type PushActionToLinearRequest = {
  actionId: string;
  teamId: string;
  title: string;
};

export function useActionDetailCommands() {
  const getActionDetail = useCallback((actionId: string) => {
    return invoke<ActionDetail>("get_action_detail", { actionId });
  }, []);

  const getLinearStatus = useCallback(() => {
    return invoke<LinearStatus>("get_linear_status");
  }, []);

  const getLinearTeams = useCallback(() => {
    return invoke<LinearTeam[]>("get_linear_teams");
  }, []);

  const reopenAction = useCallback((id: string) => {
    return invoke("reopen_action", { id });
  }, []);

  const completeAction = useCallback((id: string) => {
    return invoke("complete_action", { id });
  }, []);

  const updateAction = useCallback((request: Record<string, unknown>) => {
    return invoke("update_action", { request });
  }, []);

  const pushActionToLinear = useCallback((request: PushActionToLinearRequest) => {
    return invoke<LinearPushResult>("push_action_to_linear", request);
  }, []);

  return useMemo(() => ({
    completeAction,
    getActionDetail,
    getLinearStatus,
    getLinearTeams,
    pushActionToLinear,
    reopenAction,
    updateAction,
  }), [
    completeAction,
    getActionDetail,
    getLinearStatus,
    getLinearTeams,
    pushActionToLinear,
    reopenAction,
    updateAction,
  ]);
}
