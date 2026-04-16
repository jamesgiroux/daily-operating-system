/**
 * AccountDetailContext — React Context for the account detail shell route.
 *
 * DOS-111: The shell route (AccountDetailShell) calls all the hooks and
 * passes the combined state down via this context. Child routes (the legacy
 * page and future tab routes) consume it via useAccountDetailCtx().
 */
import { createContext, useContext } from "react";
import type { RolePreset } from "@/types/preset";
import type { VitalConflict } from "@/components/entity/EditableVitalsStrip";

/* ── hook return types ── */

export type AccountDetailHookReturn = ReturnType<typeof import("@/hooks/useAccountDetail").useAccountDetail>;

export interface AccountDetailContextValue {
  accountId: string;
  acct: AccountDetailHookReturn;
  preset: RolePreset | null;
  feedback: {
    getFeedback: (field: string) => "positive" | "negative" | null;
    submitFeedback: (field: string, type: "positive" | "negative", context?: string) => Promise<void>;
  };
  entityCtx: {
    entries: import("@/types").EntityContextEntry[];
    loading: boolean;
    createEntry: (title: string, content: string) => Promise<void>;
    updateEntry: (id: string, title: string, content: string) => Promise<void>;
    deleteEntry: (id: string) => Promise<void>;
  };
  handleUpdateIntelField: (field: string, value: string) => Promise<void>;
  saveStatus: "idle" | "saving" | "saved";
  setFolioSaveStatus: (s: "idle" | "saving" | "saved") => void;
  saveMetadata: (updated: Record<string, string>) => Promise<void>;
  saveAccountField: (field: string, value: string) => Promise<void>;
  conflictsForStrip: Map<string, VitalConflict>;
  metadataValues: Record<string, string>;
  handleMetadataChange: (key: string, value: string) => void;
  ancestors: { id: string; name: string }[];
  rolloverDismissed: boolean;
  setRolloverDismissed: (v: boolean) => void;
  mergeDialogOpen: boolean;
  setMergeDialogOpen: (v: boolean) => void;
  archiveDialogOpen: boolean;
  setArchiveDialogOpen: (v: boolean) => void;
}

const AccountDetailContext = createContext<AccountDetailContextValue | null>(null);

export function AccountDetailProvider({
  value,
  children,
}: {
  value: AccountDetailContextValue;
  children: React.ReactNode;
}) {
  return (
    <AccountDetailContext.Provider value={value}>
      {children}
    </AccountDetailContext.Provider>
  );
}

export function useAccountDetailCtx(): AccountDetailContextValue {
  const ctx = useContext(AccountDetailContext);
  if (!ctx) {
    throw new Error("useAccountDetailCtx must be used within <AccountDetailProvider>");
  }
  return ctx;
}
