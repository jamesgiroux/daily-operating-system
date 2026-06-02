import { useMemo } from "react";
import type { ReactNode } from "react";
import { useNavigate, useParams } from "@tanstack/react-router";
import {
  Activity,
  AlignLeft,
  Award,
  Briefcase,
  Compass,
  Eye,
  FileText,
  Telescope,
  Users,
} from "lucide-react";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { EditorialEmpty } from "@/components/editorial/EditorialEmpty";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { ReactBlockRenderer } from "@/components/composition/ReactBlockRenderer";
import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";
import { useProjectedComposition } from "@/hooks/useProjectedComposition";
import { useRegisterMagazineShell, useUpdateFolioVolatile } from "@/hooks/useMagazineShell";
import type { ProjectedBlock, ProjectedSection } from "@/services/composition/contracts";
import shared from "@/styles/entity-detail.module.css";
import pageStyles from "./AccountDetailPage.module.css";

const SECTION_ICONS: Record<string, ReactNode> = {
  headline: <AlignLeft size={18} strokeWidth={1.5} />,
  outlook: <Telescope size={18} strokeWidth={1.5} />,
  "state-of-play": <Activity size={18} strokeWidth={1.5} />,
  "the-room": <Users size={18} strokeWidth={1.5} />,
  "whats-next": <Briefcase size={18} strokeWidth={1.5} />,
  "watch-list": <Eye size={18} strokeWidth={1.5} />,
  "value-commitments": <Award size={18} strokeWidth={1.5} />,
  "strategic-landscape": <Compass size={18} strokeWidth={1.5} />,
  "the-record": <Activity size={18} strokeWidth={1.5} />,
  "the-work": <Briefcase size={18} strokeWidth={1.5} />,
  reports: <FileText size={18} strokeWidth={1.5} />,
};

function sectionLabel(section: ProjectedSection): string {
  return section.label ?? section.section_id.replace(/-/g, " ");
}

function accountNameFromBlocks(blocks: ProjectedBlock[], accountId: string | undefined): string {
  const overview = blocks.find((block) => block.selected_known_type_id === "account_overview");
  const account = overview?.payload.account;
  if (account && typeof account === "object" && !Array.isArray(account)) {
    const displayName = (account as Record<string, unknown>).display_name;
    if (typeof displayName === "string" && displayName.trim()) return displayName;
  }
  return accountId ?? "Account";
}

function sectionBlocks(section: ProjectedSection, blocks: ProjectedBlock[]): ProjectedBlock[] {
  return section.block_indexes
    .map((index) => blocks[index])
    .filter((block): block is ProjectedBlock => Boolean(block));
}

export default function AccountDetailPage() {
  const { accountId } = useParams({ strict: false });
  const navigate = useNavigate();
  const composition = useProjectedComposition(accountId);
  const projection = composition.data?.projection ?? null;
  const accountName = accountNameFromBlocks(projection?.blocks ?? [], accountId);

  const chapters = useMemo(
    () =>
      (projection?.sections ?? []).map((section) => ({
        id: section.section_id,
        label: sectionLabel(section),
        icon: SECTION_ICONS[section.section_id] ?? <FileText size={18} strokeWidth={1.5} />,
      })),
    [projection?.sections],
  );

  const shellConfig = useMemo(
    () => ({
      folioLabel: "Account",
      atmosphereColor: "turmeric" as const,
      activePage: "accounts" as const,
      breadcrumbs: [
        { label: "Accounts", onClick: () => navigate({ to: "/accounts" }) },
        { label: accountName },
      ],
      chapters,
    }),
    [accountName, chapters, navigate],
  );
  useRegisterMagazineShell(shellConfig);

  useUpdateFolioVolatile(
    {
      folioStatusText: composition.loading
        ? "Composing..."
        : composition.data?.served_from_cache
          ? "Projected from cache"
          : undefined,
      folioActions: (
        <div className={shared.folioActions}>
          <FolioRefreshButton onClick={composition.refetch} loading={composition.loading} />
        </div>
      ),
    },
    accountId,
  );

  if (composition.loading && !projection) return <EditorialLoading />;
  if (composition.error) {
    return <EditorialError message={composition.error} onRetry={composition.refetch} />;
  }
  if (!projection || projection.sections.length === 0 || projection.blocks.length === 0) {
    return <EditorialEmpty title="No account composition" message="DailyOS has not produced an account surface yet." />;
  }

  return (
    <main
      className={pageStyles.compositionSurface}
      data-composition-id={projection.composition_id}
      data-composition-version={projection.composition_version ?? 0}
      data-fallback-policy-version={projection.fallback_policy_version}
    >
      {projection.sections.map((section) => {
        const blocks = sectionBlocks(section, projection.blocks);
        if (section.section_id === "headline") {
          return (
            <section
              key={section.section_id}
              id={section.section_id}
              className={pageStyles.compositionMasthead}
              data-section-id={section.section_id}
              data-section-layout={section.layout}
            >
              <div className={pageStyles.compositionMastheadGrid}>
                {blocks.map((block) => (
                  <ReactBlockRenderer
                    key={block.block_id}
                    block={block}
                    accountId={accountId}
                    renderedProvenance={composition.renderedProvenance}
                  />
                ))}
              </div>
            </section>
          );
        }

        return (
          <section
            key={section.section_id}
            id={section.section_id}
            className={pageStyles.compositionSection}
            data-section-id={section.section_id}
            data-section-layout={section.layout}
            data-section-salience={section.salience.band}
          >
            <div className={pageStyles.compositionSectionLabel}>{sectionLabel(section)}</div>
            <div className={pageStyles.compositionSectionBody}>
              <header className={pageStyles.compositionSectionHeader}>
                <h2 className={pageStyles.compositionSectionTitle}>{sectionLabel(section)}</h2>
                <p className={pageStyles.compositionSectionMeta}>{section.salience.reason}</p>
              </header>
              <div className={pageStyles.compositionBlockStack}>
                {blocks.length > 0 ? (
                  blocks.map((block) => (
                    <ReactBlockRenderer
                      key={block.block_id}
                      block={block}
                      accountId={accountId}
                      renderedProvenance={composition.renderedProvenance}
                    />
                  ))
                ) : (
                  <div className={pageStyles.compositionDegradedState}>
                    <p className={pageStyles.compositionStateLabel}>Empty section</p>
                    <p className={pageStyles.compositionStateText}>No renderable blocks are available for this section.</p>
                  </div>
                )}
              </div>
            </div>
          </section>
        );
      })}
      <FinisMarker />
    </main>
  );
}
