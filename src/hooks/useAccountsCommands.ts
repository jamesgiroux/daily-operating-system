import { useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  AccountListItem,
  DiscoveredAccount,
  EphemeralBriefing,
  FeatureFlags,
  GleanAuthStatus,
} from "@/types";

/** Lightweight shape returned by get_archived_accounts (DbAccount from Rust). */
export interface ArchivedAccount {
  id: string;
  name: string;
  lifecycle?: string;
  arr?: number;
  health?: string;
  archived: boolean;
}

/** Payload for import_account_from_glean — built from a discovery row or an ephemeral briefing. */
export interface ImportAccountFromGleanRequest {
  name: string;
  myRole: string | null;
  evidence: string | null;
  source: string | null;
  domain: string | null;
  industry: string | null;
  contextPreview: string | null;
  sections: EphemeralBriefing["sections"];
  summary: string | null;
}

export function useAccountsCommands() {
  const getAccountsList = useCallback(() => {
    return invoke<AccountListItem[]>("get_accounts_list");
  }, []);

  const getChildAccountsList = useCallback((parentId: string) => {
    return invoke<AccountListItem[]>("get_child_accounts_list", { parentId });
  }, []);

  const getArchivedAccounts = useCallback(() => {
    return invoke<ArchivedAccount[]>("get_archived_accounts");
  }, []);

  const getFeatureFlags = useCallback(() => {
    return invoke<FeatureFlags>("get_feature_flags");
  }, []);

  const getGleanAuthStatus = useCallback(() => {
    return invoke<GleanAuthStatus>("get_glean_auth_status");
  }, []);

  const discoverAccountsFromGlean = useCallback(() => {
    return invoke<DiscoveredAccount[]>("discover_accounts_from_glean");
  }, []);

  const importAccountFromGlean = useCallback(
    (request: ImportAccountFromGleanRequest) => {
      return invoke<string>("import_account_from_glean", { request });
    },
    [],
  );

  const queryEphemeralAccount = useCallback((name: string) => {
    return invoke<EphemeralBriefing>("query_ephemeral_account", { name });
  }, []);

  const createAccount = useCallback(
    (
      name: string,
      accountType: "customer" | "internal" | "partner",
      parentId: string | null,
    ) => {
      return invoke<string>("create_account", { name, accountType, parentId });
    },
    [],
  );

  const bulkCreateAccounts = useCallback((names: string[]) => {
    return invoke<string[]>("bulk_create_accounts", { names });
  }, []);

  return useMemo(() => ({
    bulkCreateAccounts,
    createAccount,
    discoverAccountsFromGlean,
    getAccountsList,
    getArchivedAccounts,
    getChildAccountsList,
    getFeatureFlags,
    getGleanAuthStatus,
    importAccountFromGlean,
    queryEphemeralAccount,
  }), [
    bulkCreateAccounts,
    createAccount,
    discoverAccountsFromGlean,
    getAccountsList,
    getArchivedAccounts,
    getChildAccountsList,
    getFeatureFlags,
    getGleanAuthStatus,
    importAccountFromGlean,
    queryEphemeralAccount,
  ]);
}
