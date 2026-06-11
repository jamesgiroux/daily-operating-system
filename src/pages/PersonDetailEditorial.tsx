import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useParams, useNavigate } from "@tanstack/react-router";
import {
  Activity,
  AlignLeft,
  Archive,
  CheckSquare2,
  Eye,
  FileText,
  GitMerge,
  Network,
  Plus,
  RefreshCw,
  RotateCcw,
  Save,
  Sparkles,
  Trash2,
  Users,
  X,
  Zap,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { EditorialEmpty } from "@/components/editorial/EditorialEmpty";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { PersonAppendix } from "@/components/person/PersonAppendix";
import { PersonNetwork } from "@/components/person/PersonNetwork";
import { PersonRelationships } from "@/components/person/PersonRelationships";
import { ReactBlockRenderer } from "@/components/composition/ReactBlockRenderer";
import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";
import { useActivePreset } from "@/hooks/useActivePreset";
import { useEntityDetailCommands } from "@/hooks/useEntityDetailCommands";
import { usePersonDetail } from "@/hooks/usePersonDetail";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import {
  useChapterLayout,
  type RenderableCompositionBlock,
  type RenderableCompositionSection,
} from "@/hooks/useChapterLayout";
import { useProjectedComposition } from "@/hooks/useProjectedComposition";
import type { PersonDetail, PersonRelationshipEdge } from "@/types";
import type { ProjectedBlock } from "@/services/composition/contracts";
import shared from "@/styles/entity-detail.module.css";
import accountStyles from "./AccountDetailPage.module.css";
import styles from "./PersonDetailEditorial.module.css";

const SECTION_ICONS: Record<string, React.ReactNode> = {
  headline: <AlignLeft size={18} strokeWidth={1.5} />,
  "the-dynamic": <Zap size={18} strokeWidth={1.5} />,
  "the-rhythm": <RefreshCw size={18} strokeWidth={1.5} />,
  "their-orbit": <Network size={18} strokeWidth={1.5} />,
  "their-network": <Users size={18} strokeWidth={1.5} />,
  "the-landscape": <Eye size={18} strokeWidth={1.5} />,
  "open-threads": <CheckSquare2 size={18} strokeWidth={1.5} />,
  "the-record": <Activity size={18} strokeWidth={1.5} />,
  "the-work": <CheckSquare2 size={18} strokeWidth={1.5} />,
};

function personNameFromBlocks(
  blocks: ProjectedBlock[],
  detail: PersonDetail | null,
  personId: string | undefined,
): string {
  const overview = blocks.find((block) => block.selected_known_type_id === "account_overview");
  const person = overview?.payload.person;
  if (person && typeof person === "object" && !Array.isArray(person)) {
    const displayName = (person as Record<string, unknown>).display_name;
    if (typeof displayName === "string" && displayName.trim()) return displayName;
  }
  const account = overview?.payload.account;
  if (account && typeof account === "object" && !Array.isArray(account)) {
    const displayName = (account as Record<string, unknown>).display_name;
    if (typeof displayName === "string" && displayName.trim()) return displayName;
  }
  return detail?.name ?? personId ?? "Person";
}

function PersonControlsPanel({
  detail,
  person,
  onArchive,
  onAfterMutation,
}: {
  detail: PersonDetail;
  person: ReturnType<typeof usePersonDetail>;
  onArchive: () => void;
  onAfterMutation: () => Promise<void>;
}) {
  const runMutationAndRefresh = (mutation: () => Promise<void>) => {
    void (async () => {
      await mutation();
      await onAfterMutation();
    })();
  };

  return (
    <section className={styles.personControlPanel} aria-label="Person controls">
      <form
        className={styles.personControlForm}
        onSubmit={(event) => {
          event.preventDefault();
          runMutationAndRefresh(person.handleSave);
        }}
      >
        <label className={styles.personControlField}>
          <span>Name</span>
          <Input
            value={person.editName}
            onChange={(event) => {
              person.setEditName(event.target.value);
              person.setDirty(true);
            }}
          />
        </label>
        <label className={styles.personControlField}>
          <span>Role</span>
          <Input
            value={person.editRole}
            onChange={(event) => {
              person.setEditRole(event.target.value);
              person.setDirty(true);
            }}
          />
        </label>
        <div className={styles.personControlActions}>
          <Button type="submit" disabled={!person.dirty || person.saving}>
            <Save size={15} strokeWidth={1.7} />
            {person.saving ? "Saving" : "Save"}
          </Button>
          <Button
            type="button"
            variant="ghost"
            disabled={!person.dirty}
            onClick={person.handleCancelEdit}
          >
            <X size={15} strokeWidth={1.7} />
            Cancel
          </Button>
          <Button
            type="button"
            variant="ghost"
            onClick={() => runMutationAndRefresh(person.handleEnrich)}
            disabled={person.enriching}
          >
            <Sparkles size={15} strokeWidth={1.7} />
            {person.enriching ? `${person.enrichSeconds}s` : "Refresh"}
          </Button>
          <Button
            type="button"
            variant="ghost"
            onClick={detail.archived ? () => runMutationAndRefresh(person.handleUnarchive) : onArchive}
          >
            {detail.archived ? (
              <RotateCcw size={15} strokeWidth={1.7} />
            ) : (
              <Archive size={15} strokeWidth={1.7} />
            )}
            {detail.archived ? "Restore" : "Archive"}
          </Button>
          <Button type="button" variant="ghost" onClick={person.openMergeDialog}>
            <GitMerge size={15} strokeWidth={1.7} />
            Merge
          </Button>
          <Button
            type="button"
            variant="ghost"
            onClick={() => person.setDeleteConfirmOpen(true)}
          >
            <Trash2 size={15} strokeWidth={1.7} />
            Delete
          </Button>
        </div>
      </form>
      <div className={styles.personActionComposer}>
        {person.addingAction ? (
          <>
            <Input
              value={person.newActionTitle}
              onChange={(event) => person.setNewActionTitle(event.target.value)}
              placeholder="Action title"
            />
            <Button
              type="button"
              onClick={() => runMutationAndRefresh(person.handleCreateAction)}
              disabled={person.creatingAction || !person.newActionTitle.trim()}
            >
              <Plus size={15} strokeWidth={1.7} />
              {person.creatingAction ? "Creating" : "Create"}
            </Button>
            <Button type="button" variant="ghost" onClick={() => person.setAddingAction(false)}>
              <X size={15} strokeWidth={1.7} />
              Cancel
            </Button>
          </>
        ) : (
          <Button type="button" variant="ghost" onClick={() => person.setAddingAction(true)}>
            <Plus size={15} strokeWidth={1.7} />
            Action
          </Button>
        )}
      </div>
    </section>
  );
}

export default function PersonDetailEditorial() {
  const { personId } = useParams({ strict: false });
  const navigate = useNavigate();
  const person = usePersonDetail(personId);
  const { getPersonRelationships } = useEntityDetailCommands();
  const preset = useActivePreset();
  const composition = useProjectedComposition(
    personId ? { entityType: "person", entityId: personId } : undefined,
  );
  const projection = composition.data?.projection ?? null;
  const layout = useChapterLayout({ projection, entityType: "person" });
  const visibleSections = layout.view.sections;
  const personName = personNameFromBlocks(projection?.blocks ?? [], person.detail, personId);
  const [archiveDialogOpen, setArchiveDialogOpen] = useState(false);
  const [relationships, setRelationships] = useState<PersonRelationshipEdge[]>([]);

  useRevealObserver(!composition.loading && !!projection);

  const forceRefreshProjectedComposition = useCallback(
    () => composition.refetch({ forceRefresh: true }),
    [composition.refetch],
  );

  const loadRelationships = useCallback(() => {
    if (!personId) return;
    getPersonRelationships(personId)
      .then(setRelationships)
      .catch(() => setRelationships([]));
  }, [personId, getPersonRelationships]);

  useEffect(() => {
    loadRelationships();
  }, [loadRelationships]);

  const handleLinkEntity = useCallback(
    async (entityId: string) => {
      await person.handleLinkEntity(entityId);
      await forceRefreshProjectedComposition();
    },
    [forceRefreshProjectedComposition, person],
  );

  const handleUnlinkEntity = useCallback(
    async (entityId: string) => {
      await person.handleUnlinkEntity(entityId);
      await forceRefreshProjectedComposition();
    },
    [forceRefreshProjectedComposition, person],
  );

  const handleRelationshipsChanged = useCallback(() => {
    loadRelationships();
    void forceRefreshProjectedComposition();
  }, [forceRefreshProjectedComposition, loadRelationships]);

  const chapters = useMemo(
    () =>
      visibleSections.map(({ section, label }) => ({
        id: section.section_id,
        label,
        icon: SECTION_ICONS[section.section_id] ?? <FileText size={18} strokeWidth={1.5} />,
      })),
    [visibleSections],
  );

  const shellConfig = useMemo(
    () => ({
      folioLabel: "Person",
      atmosphereColor: "larkspur" as const,
      activePage: "people" as const,
      breadcrumbs: [
        { label: "People", onClick: () => navigate({ to: "/people" }) },
        { label: personName },
      ],
      chapters,
      folioStatusText: composition.loading
        ? "Composing..."
        : person.saving
          ? "Saving..."
          : composition.data?.served_from_cache
            ? "Projected from cache"
            : undefined,
      folioActions: (
        <div className={shared.folioActions}>
          <FolioRefreshButton onClick={composition.refetch} loading={composition.loading} />
        </div>
      ),
    }),
    [
      chapters,
      composition.data?.served_from_cache,
      composition.loading,
      composition.refetch,
      navigate,
      person.saving,
      personName,
    ],
  );
  useRegisterMagazineShell(shellConfig);

  if (composition.loading && !projection) return <EditorialLoading />;

  if (composition.error) {
    return <EditorialError message={composition.error} onRetry={composition.refetch} />;
  }

  if (!projection || projection.sections.length === 0 || projection.blocks.length === 0) {
    return (
      <EditorialEmpty
        title="No person composition"
        message="DailyOS has not produced a person surface yet."
      />
    );
  }

  const renderBlock = (item: RenderableCompositionBlock) => (
    <ReactBlockRenderer
      key={item.block.block_id}
      block={item.block}
      entityId={personId}
      entityType="person"
      renderedProvenance={composition.renderedProvenance}
    />
  );

  const renderSectionTitle = (section: RenderableCompositionSection) => (
    <h2 className={accountStyles.compositionSectionTitle}>{section.label}</h2>
  );

  const detail = person.detail;

  return (
    <main
      className={accountStyles.compositionSurface}
      data-composition-id={projection.composition_id}
      data-composition-version={projection.composition_version ?? 0}
      data-fallback-policy-version={projection.fallback_policy_version}
    >
      {person.error && (
        <div className={accountStyles.compositionDegradedState} role="status">
          <p className={accountStyles.compositionStateLabel}>Person controls unavailable</p>
          <p className={accountStyles.compositionStateText}>{person.error}</p>
        </div>
      )}

      {visibleSections.map((renderableSection) => {
        const section = renderableSection.section;
        const blocks = renderableSection.blocks;
        if (section.section_id === "headline") {
          return (
            <section
              key={section.section_id}
              id={section.section_id}
              className={accountStyles.compositionMasthead}
              data-section-id={section.section_id}
              data-section-layout={section.layout}
            >
              <div className={accountStyles.compositionMastheadGrid}>
                {blocks.map(renderBlock)}
              </div>
            </section>
          );
        }

        return (
          <section
            key={section.section_id}
            id={section.section_id}
            className={accountStyles.compositionSection}
            data-section-id={section.section_id}
            data-section-layout={section.layout}
            data-section-salience={section.salience.band}
          >
            <div className={accountStyles.compositionSectionLabel}>{renderableSection.label}</div>
            <div className={accountStyles.compositionSectionBody}>
              <header className={accountStyles.compositionSectionHeader}>
                <div>{renderSectionTitle(renderableSection)}</div>
                <p className={accountStyles.compositionSectionMeta}>{section.salience.reason}</p>
              </header>
              <div className={accountStyles.compositionBlockStack}>
                {blocks.length > 0 ? (
                  blocks.map(renderBlock)
                ) : (
                  <div className={accountStyles.compositionDegradedState}>
                    <p className={accountStyles.compositionStateLabel}>Empty section</p>
                    <p className={accountStyles.compositionStateText}>
                      No renderable blocks are available for this section.
                    </p>
                  </div>
                )}
              </div>
            </div>
          </section>
        );
      })}

      {detail && (
        <>
          <PersonControlsPanel
            detail={detail}
            person={person}
            onArchive={() => setArchiveDialogOpen(true)}
            onAfterMutation={forceRefreshProjectedComposition}
          />
          <section className={styles.personRelationshipControls} aria-label="Relationship controls">
            <PersonNetwork
              entities={detail.entities}
              personId={personId}
              onLink={handleLinkEntity}
              onUnlink={handleUnlinkEntity}
              chapterTitle="Their Orbit"
            />
            <PersonRelationships
              personId={personId ?? ""}
              network={person.intelligence?.network}
              relationships={relationships}
              preset={preset ?? undefined}
              chapterTitle="Their Network"
              onRelationshipsChanged={handleRelationshipsChanged}
            />
          </section>
          <PersonAppendix
            detail={detail}
            duplicateCandidates={person.duplicateCandidates}
            onMergeSuggested={person.handleOpenSuggestedMerge}
            merging={person.merging}
            files={person.files}
            onIndexFiles={person.handleIndexFiles}
            indexing={person.indexing}
            indexFeedback={person.indexFeedback}
          />
        </>
      )}

      <FinisMarker enrichedAt={person.intelligence?.enrichedAt} />

      {detail && (
        <AlertDialog open={archiveDialogOpen} onOpenChange={setArchiveDialogOpen}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Archive Person</AlertDialogTitle>
              <AlertDialogDescription>
                This will hide {detail.name} from active views. You can restore them later.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction onClick={person.handleArchive}>Archive</AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      )}

      {detail && (
        <Dialog open={person.mergeDialogOpen} onOpenChange={person.setMergeDialogOpen}>
          <DialogContent className="sm:max-w-md">
            <DialogHeader>
              <DialogTitle>Merge {detail.name} into...</DialogTitle>
              <DialogDescription>
                Search for the person to merge into. Meetings, entity links, and actions transfer to the selected person.
              </DialogDescription>
            </DialogHeader>
            <Input
              placeholder="Search by name or email..."
              value={person.mergeSearchQuery}
              onChange={(event) => person.setMergeSearchQuery(event.target.value)}
              autoFocus
            />
            {person.mergeSearchResults.length > 0 && (
              <div className={shared.mergeSearchResults}>
                {person.mergeSearchResults.map((candidate) => (
                  <button
                    key={candidate.id}
                    type="button"
                    onClick={() => {
                      person.setMergeTarget(candidate);
                      person.setMergeDialogOpen(false);
                      person.setMergeConfirmOpen(true);
                    }}
                    className={`${shared.mergeSearchButton} hover:bg-muted`}
                  >
                    <div className={shared.mergeAvatar}>{candidate.name.charAt(0).toUpperCase()}</div>
                    <div className={shared.mergePersonInfo}>
                      <div className={shared.mergePersonName}>{candidate.name}</div>
                      <div className={shared.mergePersonEmail}>
                        {candidate.email}
                        {candidate.organization && ` · ${candidate.organization}`}
                      </div>
                    </div>
                  </button>
                ))}
              </div>
            )}
            {person.mergeSearchQuery.length >= 2 && person.mergeSearchResults.length === 0 && (
              <p className={shared.mergeEmptyState}>No matching people found</p>
            )}
          </DialogContent>
        </Dialog>
      )}

      {detail && (
        <AlertDialog open={person.mergeConfirmOpen} onOpenChange={person.setMergeConfirmOpen}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Merge {detail.name}?</AlertDialogTitle>
              <AlertDialogDescription>
                Permanently merge <strong>{detail.name}</strong> ({detail.email}) into{" "}
                <strong>{person.mergeTarget?.name}</strong> ({person.mergeTarget?.email}).
                Meetings, entity links, and actions transfer. This cannot be undone.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel disabled={person.merging}>Cancel</AlertDialogCancel>
              <AlertDialogAction onClick={person.handleMerge} disabled={person.merging}>
                {person.merging ? "Merging..." : "Merge"}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      )}

      {detail && (
        <AlertDialog open={person.deleteConfirmOpen} onOpenChange={person.setDeleteConfirmOpen}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Delete {detail.name}?</AlertDialogTitle>
              <AlertDialogDescription>
                All meeting attendance records, entity links, and action associations for{" "}
                <strong>{detail.name}</strong> will be removed. This cannot be undone.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel disabled={person.merging}>Cancel</AlertDialogCancel>
              <AlertDialogAction onClick={person.handleDelete} disabled={person.merging}>
                {person.merging ? "Deleting..." : "Delete"}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      )}
    </main>
  );
}
