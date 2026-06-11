import { useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AccountDetail, AccountListItem, RiskBriefing } from "@/types";
import type { ReportRow } from "@/types/reports";

type ReportEntityType = "account" | "project" | "person" | "user";

interface ReportRequest {
  entityId: string;
  entityType: ReportEntityType;
  reportType: string;
}

interface SaveReportRequest extends ReportRequest {
  contentJson: string;
}

interface GenerateReportRequest extends ReportRequest {
  spotlightAccountIds?: string[];
}

export function useReportCommands() {
  const getUserEntityId = useCallback(async () => {
    const user = await invoke<{ id: string | number }>("get_user_entity");
    return String(user.id);
  }, []);

  const saveReport = useCallback((request: SaveReportRequest) => {
    return invoke("save_report", { ...request });
  }, []);

  const getReport = useCallback(<T extends ReportRow | null = ReportRow>(
    request: ReportRequest,
  ) => {
    return invoke<T>("get_report", { ...request });
  }, []);

  const generateReport = useCallback(<T extends ReportRow = ReportRow>(
    request: GenerateReportRequest,
  ) => {
    return invoke<T>("generate_report", { ...request });
  }, []);

  const getAccountDetail = useCallback(<T extends Pick<AccountDetail, "name">>(
    accountId: string,
  ) => {
    return invoke<T>("get_account_detail", { accountId });
  }, []);

  const getAccountsList = useCallback(() => {
    return invoke<AccountListItem[]>("get_accounts_list");
  }, []);

  const getChildAccountsList = useCallback((parentId: string) => {
    return invoke<AccountListItem[]>("get_child_accounts_list", { parentId });
  }, []);

  const generateRiskBriefing = useCallback((accountId: string) => {
    return invoke<RiskBriefing>("generate_risk_briefing", { accountId });
  }, []);

  return useMemo(() => ({
    generateReport,
    generateRiskBriefing,
    getAccountDetail,
    getAccountsList,
    getChildAccountsList,
    getReport,
    getUserEntityId,
    saveReport,
  }), [
    generateReport,
    generateRiskBriefing,
    getAccountDetail,
    getAccountsList,
    getChildAccountsList,
    getReport,
    getUserEntityId,
    saveReport,
  ]);
}
