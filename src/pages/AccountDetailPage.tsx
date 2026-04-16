/**
 * AccountDetailPage — Clean rebuild of the account detail page.
 *
 * Single flat route, state-based view switching, no child routes.
 * Built step by step per plan at ~/.claude/plans/deep-wiggling-hearth.md.
 *
 * Step 1: Shell chrome only (folio bar, nav island, atmosphere, entity name).
 */
import { useState, useMemo } from "react";
import { useParams, useNavigate } from "@tanstack/react-router";
import { useAccountDetail } from "@/hooks/useAccountDetail";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useRegisterMagazineShell, useUpdateFolioVolatile } from "@/hooks/useMagazineShell";
import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";
import { FolioReportsDropdown } from "@/components/folio/FolioReportsDropdown";
import { FolioToolsDropdown } from "@/components/folio/FolioToolsDropdown";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { buildChapters } from "@/components/account/account-detail-utils";

import shared from "@/styles/entity-detail.module.css";

export default function AccountDetailPage() {
  const { accountId } = useParams({ strict: false });
  const navigate = useNavigate();
  const acct = useAccountDetail(accountId);
  useRevealObserver(!acct.loading && !!acct.detail);

  const chapters = useMemo(
    () => buildChapters(acct.detail?.isParent ?? false, !!acct.intelligence?.health),
    [acct.detail?.isParent, acct.intelligence?.health],
  );

  const shellConfig = useMemo(() => ({
    folioLabel: acct.detail?.accountType === "internal" ? "Internal" : acct.detail?.accountType === "partner" ? "Partner" : "Account",
    atmosphereColor: acct.detail?.accountType === "internal" ? "larkspur" as const : "turmeric" as const,
    activePage: "accounts" as const,
    backLink: { label: "Back", onClick: () => window.history.length > 1 ? window.history.back() : navigate({ to: "/accounts" }) },
    chapters,
  }), [navigate, acct.detail?.accountType, chapters]);
  useRegisterMagazineShell(shellConfig);

  const [_mergeDialogOpen, setMergeDialogOpen] = useState(false);

  useUpdateFolioVolatile({
    folioActions: (
      <div className={shared.folioActions}>
        {acct.detail && !acct.detail.archived && (
          <FolioRefreshButton onClick={acct.handleEnrich} loading={!!acct.enriching}
            loadingProgress={acct.enriching ? acct.enrichmentPercentage != null ? `${acct.enrichmentPercentage}%` : `${acct.enrichSeconds ?? 0}s` : undefined} />
        )}
        <FolioReportsDropdown accountId={accountId!} />
        <FolioToolsDropdown onCreateChild={() => acct.setCreateChildOpen(true)} onMerge={() => setMergeDialogOpen(true)}
          onArchive={() => {}} onUnarchive={acct.handleUnarchive} onIndexFiles={acct.handleIndexFiles}
          isArchived={!!acct.detail?.archived} isIndexing={acct.indexing} hasDetail={!!acct.detail} />
      </div>
    ),
  }, accountId);

  if (acct.loading) return <EditorialLoading />;
  if (acct.error || !acct.detail) return <EditorialError message={acct.error ?? "Account not found"} onRetry={acct.load} />;

  const detail = acct.detail;

  return (
    <>
      <section id="headline" className={shared.chapterSection}>
        <h1 style={{ fontFamily: "var(--font-serif)", fontSize: 76, fontWeight: 400, letterSpacing: "-0.025em", lineHeight: 1.06, margin: 0 }}>
          {detail.name}
        </h1>
      </section>
    </>
  );
}
