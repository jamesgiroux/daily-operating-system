import { useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  EmailBriefingData,
  EmailSyncStats,
  FailedEmailPreview,
} from "@/types";

// Type aliases (not interfaces) so they satisfy InvokeArgs' implicit index signature.
type DismissEmailItemRequest = {
  itemType: string;
  emailId: string;
  itemText: string;
  senderDomain: string | null;
  emailType: string | null;
  entityId: string | null;
};

type PromoteCommitmentRequest = {
  emailId: string;
  commitmentText: string;
  actionTitle: string;
  entityId: string | null;
  entityType: "account" | "project" | null;
  owner: string | null;
  dueDate: string | null;
};

export function useEmailCommands() {
  const refreshEmails = useCallback(() => {
    return invoke<string>("refresh_emails");
  }, []);

  const loadEmailBriefing = useCallback(async () => {
    const [data, dismissedItems, stats] = await Promise.all([
      invoke<EmailBriefingData>("get_emails_enriched"),
      invoke<string[]>("list_dismissed_email_items").catch((err) => {
        console.error("list_dismissed_email_items failed:", err);
        return [] as string[];
      }),
      invoke<EmailSyncStats>("get_email_sync_status").catch(() => null),
    ]);

    return { data, dismissedItems, stats };
  }, []);

  const syncEmailInboxPresence = useCallback(() => {
    return invoke<boolean>("sync_email_inbox_presence");
  }, []);

  const dismissEmailItem = useCallback((request: DismissEmailItemRequest) => {
    return invoke("dismiss_email_item", request);
  }, []);

  const dismissGoneQuiet = useCallback((entityId: string) => {
    return invoke("dismiss_gone_quiet", { entityId });
  }, []);

  const dismissEmailSignal = useCallback((signalId: number) => {
    return invoke("dismiss_email_signal", { signalId });
  }, []);

  const retryFailedEmails = useCallback(() => {
    return invoke<number>("retry_failed_emails");
  }, []);

  const listPermanentlyFailedEmails = useCallback(() => {
    return invoke<FailedEmailPreview[]>("list_permanently_failed_emails");
  }, []);

  const skipFailedEmails = useCallback((emailIds: string[]) => {
    return invoke<number>("skip_failed_emails", { emailIds });
  }, []);

  const promoteCommitmentToAction = useCallback((request: PromoteCommitmentRequest) => {
    return invoke<string>("promote_commitment_to_action", request);
  }, []);

  const archiveEmail = useCallback((emailId: string) => {
    return invoke<string>("archive_email", { emailId });
  }, []);

  const unarchiveEmail = useCallback((emailId: string) => {
    return invoke("unarchive_email", { emailId });
  }, []);

  const pinEmail = useCallback((emailId: string) => {
    return invoke<boolean>("pin_email", { emailId });
  }, []);

  return useMemo(() => ({
    archiveEmail,
    dismissEmailItem,
    dismissEmailSignal,
    dismissGoneQuiet,
    listPermanentlyFailedEmails,
    loadEmailBriefing,
    pinEmail,
    promoteCommitmentToAction,
    refreshEmails,
    retryFailedEmails,
    skipFailedEmails,
    syncEmailInboxPresence,
    unarchiveEmail,
  }), [
    archiveEmail,
    dismissEmailItem,
    dismissEmailSignal,
    dismissGoneQuiet,
    listPermanentlyFailedEmails,
    loadEmailBriefing,
    pinEmail,
    promoteCommitmentToAction,
    refreshEmails,
    retryFailedEmails,
    skipFailedEmails,
    syncEmailInboxPresence,
    unarchiveEmail,
  ]);
}
