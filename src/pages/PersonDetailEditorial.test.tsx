/** @vitest-environment jsdom */

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import PersonDetailEditorial from "@/pages/PersonDetailEditorial";
import type {
  ProjectedBlock,
  ProjectedComposition,
  ProjectedCompositionCommandResponse,
} from "@/services/composition/contracts";

const {
  invokeMock,
  navigateMock,
  usePersonDetailMock,
  useProjectedCompositionMock,
  projectedRefetchMock,
  useRegisterMagazineShellMock,
} = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  navigateMock: vi.fn(),
  usePersonDetailMock: vi.fn(),
  useProjectedCompositionMock: vi.fn(),
  projectedRefetchMock: vi.fn(),
  useRegisterMagazineShellMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tanstack/react-router", () => ({
  useParams: () => ({ personId: "person-test-1" }),
  useNavigate: () => navigateMock,
}));

vi.mock("@/hooks/usePersonDetail", () => ({
  usePersonDetail: () => usePersonDetailMock(),
}));

vi.mock("@/hooks/useProjectedComposition", () => ({
  useProjectedComposition: (subject: unknown) => useProjectedCompositionMock(subject),
}));

vi.mock("@/hooks/useActivePreset", () => ({
  useActivePreset: () => null,
}));

vi.mock("@/hooks/useRevealObserver", () => ({
  useRevealObserver: vi.fn(),
}));

vi.mock("@/hooks/useMagazineShell", () => ({
  useRegisterMagazineShell: (config: unknown) => useRegisterMagazineShellMock(config),
}));

vi.mock("@/hooks/useChapterLayout", () => ({
  useChapterLayout: ({ projection }: { projection: ProjectedComposition | null }) => {
    const sections =
      projection?.sections.map((section) => ({
        section,
        label: section.label ?? section.section_id,
        coreLocked: section.section_id === "headline",
        blocks: section.block_indexes.map((index) => {
          const block = projection.blocks[index];
          return {
            block,
            sectionId: section.section_id,
            label: String(block?.payload.title ?? block?.payload.text ?? block?.block_id),
            variant: "default",
            coreLocked: section.section_id === "headline",
          };
        }),
        hiddenBlocks: [],
      })) ?? [];
    return {
      overlay: null,
      layoutRevision: 0,
      view: {
        sections,
        hiddenSections: [],
        hiddenItems: [],
        hasVisibleNonCore: sections.some((section) => !section.coreLocked),
      },
      loading: false,
      saving: false,
      error: null,
      setBlockHidden: vi.fn(),
      setSectionHidden: vi.fn(),
      setBlockVariant: vi.fn(),
      setSectionLabel: vi.fn(),
      reorderBlocks: vi.fn(),
      resetLayout: vi.fn(),
    };
  },
}));

vi.mock("@/components/composition/ReactBlockRenderer", () => ({
  ReactBlockRenderer: ({
    block,
    entityId,
    entityType,
  }: {
    block: ProjectedBlock;
    entityId?: string;
    entityType?: string;
  }) => (
    <div
      data-testid="composition-block"
      data-block-id={block.block_id}
      data-entity-id={entityId}
      data-entity-type={entityType}
    >
      {String(block.payload.title ?? block.payload.text ?? block.block_id)}
    </div>
  ),
}));

vi.mock("@/components/person/PersonNetwork", () => ({
  PersonNetwork: ({
    onLink,
    onUnlink,
  }: {
    onLink?: (entityId: string) => Promise<void> | void;
    onUnlink?: (entityId: string) => Promise<void> | void;
  }) => (
    <section data-testid="person-network-controls">
      <button type="button" onClick={() => void onLink?.("entity-link-test")}>
        Link entity
      </button>
      <button type="button" onClick={() => void onUnlink?.("entity-unlink-test")}>
        Unlink entity
      </button>
    </section>
  ),
}));

vi.mock("@/components/person/PersonRelationships", () => ({
  PersonRelationships: ({
    onRelationshipsChanged,
  }: {
    onRelationshipsChanged?: () => void;
  }) => (
    <section data-testid="person-relationship-controls">
      <button type="button" onClick={() => onRelationshipsChanged?.()}>
        Relationship changed
      </button>
    </section>
  ),
}));

vi.mock("@/components/person/PersonAppendix", () => ({
  PersonAppendix: () => <section data-testid="person-appendix" />,
}));

vi.mock("@/components/ui/folio-refresh-button", () => ({
  FolioRefreshButton: () => <button type="button">Refresh</button>,
}));

vi.mock("@/components/editorial/EditorialLoading", () => ({
  EditorialLoading: () => <div data-testid="editorial-loading">Loading</div>,
}));

vi.mock("@/components/editorial/EditorialError", () => ({
  EditorialError: ({ message }: { message: string }) => (
    <div data-testid="editorial-error">{message}</div>
  ),
}));

vi.mock("@/components/editorial/EditorialEmpty", () => ({
  EditorialEmpty: ({ title }: { title: string }) => <div data-testid="editorial-empty">{title}</div>,
}));

vi.mock("@/components/editorial/FinisMarker", () => ({
  FinisMarker: () => <div data-testid="finis-marker" />,
}));

function block(
  blockId: string,
  sectionIndex: number,
  selectedKnownTypeId: string,
  payload: Record<string, unknown>,
): ProjectedBlock {
  return {
    block_id: blockId,
    block_index: sectionIndex,
    original_type_id: selectedKnownTypeId,
    selected_known_type_id: selectedKnownTypeId,
    payload,
    banner: null,
    trust_band: "likely_current",
    claim_refs: [],
    provenance: [],
    edit_routes: [],
    diagnostics: [],
  };
}

function projection(overrides: Partial<ProjectedComposition> = {}): ProjectedComposition {
  const blocks = [
    block("headline-block", 0, "account_overview", {
      person: { display_name: "Projected Person" },
      title: "Projected Person",
    }),
    block("dynamic-block", 1, "claim_summary", { text: "Relationship dynamic" }),
    block("network-block", 2, "relationship_map", { title: "Relationships" }),
    block("thread-block", 3, "action_list", { title: "Open threads" }),
  ];
  return {
    composition_id: "dailyos/person-overview:person:person-test-1",
    composition_version: 2,
    fallback_policy_version: 3,
    sections: [
      {
        section_id: "headline",
        section_index: 0,
        label: "Headline",
        layout: "stacked",
        salience: { weight: 0.95, band: "critical", reason: "person masthead" },
        block_ids: ["headline-block"],
        block_indexes: [0],
      },
      {
        section_id: "the-dynamic",
        section_index: 1,
        label: "The dynamic",
        layout: "stacked",
        salience: { weight: 0.86, band: "important", reason: "person dynamic" },
        block_ids: ["dynamic-block"],
        block_indexes: [1],
      },
      {
        section_id: "their-network",
        section_index: 2,
        label: "Their network",
        layout: "grid",
        salience: { weight: 0.72, band: "contextual", reason: "person network" },
        block_ids: ["network-block"],
        block_indexes: [2],
      },
      {
        section_id: "open-threads",
        section_index: 3,
        label: "Open threads",
        layout: "stacked",
        salience: { weight: 0.78, band: "important", reason: "person open threads" },
        block_ids: ["thread-block"],
        block_indexes: [3],
      },
    ],
    blocks,
    diagnostics: [],
    unknown_block_count: 0,
    unknown_block_cap: 5,
    dropped_unknown_block_count: 0,
    ...overrides,
  };
}

function response(
  projectionValue: ProjectedComposition,
): ProjectedCompositionCommandResponse {
  return {
    ok: true,
    request_id: "request-test",
    projection: projectionValue,
    cache_hint_token: "cache-test",
    served_from_cache: false,
    rendered_provenance: { surface: "tauri_app", value: { ok: true } },
  };
}

function personHook(overrides: Record<string, unknown> = {}) {
  return {
    detail: {
      id: "person-test-1",
      email: "person@example.com",
      name: "Detail Person",
      relationship: "external",
      meetingCount: 0,
      updatedAt: "2026-05-15T00:00:00Z",
      archived: false,
      entities: [],
      recentMeetings: [],
      recentCaptures: [],
      recentEmailSignals: [],
      openActions: [],
      upcomingMeetings: [],
    },
    intelligence: null,
    loading: false,
    error: null,
    load: vi.fn(),
    silentRefresh: vi.fn(),
    editName: "Detail Person",
    setEditName: vi.fn(),
    editRole: "",
    setEditRole: vi.fn(),
    dirty: false,
    setDirty: vi.fn(),
    saving: false,
    handleSave: vi.fn(),
    saveField: vi.fn(),
    handleCancelEdit: vi.fn(),
    enriching: false,
    enrichSeconds: 0,
    handleEnrich: vi.fn(),
    handleLinkEntity: vi.fn(),
    handleUnlinkEntity: vi.fn(),
    mergeDialogOpen: false,
    setMergeDialogOpen: vi.fn(),
    mergeTarget: null,
    setMergeTarget: vi.fn(),
    mergeConfirmOpen: false,
    setMergeConfirmOpen: vi.fn(),
    mergeSearchQuery: "",
    setMergeSearchQuery: vi.fn(),
    mergeSearchResults: [],
    merging: false,
    openMergeDialog: vi.fn(),
    handleMerge: vi.fn(),
    handleOpenSuggestedMerge: vi.fn(),
    deleteConfirmOpen: false,
    setDeleteConfirmOpen: vi.fn(),
    handleDelete: vi.fn(),
    duplicateCandidates: [],
    files: [],
    indexing: false,
    indexFeedback: null,
    handleIndexFiles: vi.fn(),
    handleArchive: vi.fn(),
    handleUnarchive: vi.fn(),
    addingAction: false,
    setAddingAction: vi.fn(),
    newActionTitle: "",
    setNewActionTitle: vi.fn(),
    creatingAction: false,
    handleCreateAction: vi.fn(),
    ...overrides,
  };
}

describe("PersonDetailEditorial", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue([]);
    navigateMock.mockReset();
    useRegisterMagazineShellMock.mockReset();
    projectedRefetchMock.mockReset();
    projectedRefetchMock.mockResolvedValue(undefined);
    usePersonDetailMock.mockReturnValue(personHook());
    useProjectedCompositionMock.mockReturnValue({
      loading: false,
      error: null,
      data: response(projection()),
      renderedProvenance: { surface: "tauri_app", value: { ok: true } },
      refetch: projectedRefetchMock,
    });
  });

  it("renders Person Detail from projected composition and derives shell chapters", () => {
    render(<PersonDetailEditorial />);

    expect(useProjectedCompositionMock).toHaveBeenCalledWith({
      entityType: "person",
      entityId: "person-test-1",
    });
    expect(screen.getByText("Projected Person")).toBeInTheDocument();
    expect(screen.getByText("Relationship dynamic")).toBeInTheDocument();
    expect(screen.getByText("Relationships")).toBeInTheDocument();
    expect(screen.getAllByText("Open threads")).toHaveLength(3);
    expect(screen.getByTestId("person-network-controls")).toBeInTheDocument();
    expect(screen.getByTestId("person-relationship-controls")).toBeInTheDocument();

    for (const renderedBlock of screen.getAllByTestId("composition-block")) {
      expect(renderedBlock).toHaveAttribute("data-entity-id", "person-test-1");
      expect(renderedBlock).toHaveAttribute("data-entity-type", "person");
    }

    const shellConfig = useRegisterMagazineShellMock.mock.calls.at(-1)?.[0] as {
      chapters: Array<{ id: string; label: string }>;
    };
    expect(shellConfig.chapters.map((chapter) => chapter.id)).toEqual([
      "headline",
      "the-dynamic",
      "their-network",
      "open-threads",
    ]);
  });

  it("shows producer errors without falling back to the old template", () => {
    useProjectedCompositionMock.mockReturnValue({
      loading: false,
      error: "person producer unavailable",
      data: null,
      renderedProvenance: null,
      refetch: vi.fn(),
    });

    render(<PersonDetailEditorial />);

    expect(screen.getByTestId("editorial-error")).toHaveTextContent("person producer unavailable");
    expect(screen.queryByTestId("composition-block")).not.toBeInTheDocument();
  });

  it("force-refreshes projected composition after person controls save", async () => {
    const handleSave = vi.fn().mockResolvedValue(undefined);
    usePersonDetailMock.mockReturnValue(personHook({ dirty: true, handleSave }));

    render(<PersonDetailEditorial />);

    fireEvent.click(screen.getByRole("button", { name: /Save/i }));

    await waitFor(() => expect(handleSave).toHaveBeenCalled());
    expect(projectedRefetchMock).toHaveBeenCalledWith({ forceRefresh: true });
  });

  it("force-refreshes projected composition after relationship mutations", async () => {
    const handleLinkEntity = vi.fn().mockResolvedValue(undefined);
    const handleUnlinkEntity = vi.fn().mockResolvedValue(undefined);
    usePersonDetailMock.mockReturnValue(
      personHook({
        handleLinkEntity,
        handleUnlinkEntity,
      }),
    );

    render(<PersonDetailEditorial />);

    fireEvent.click(screen.getByRole("button", { name: "Link entity" }));
    await waitFor(() => expect(handleLinkEntity).toHaveBeenCalledWith("entity-link-test"));

    fireEvent.click(screen.getByRole("button", { name: "Unlink entity" }));
    await waitFor(() => expect(handleUnlinkEntity).toHaveBeenCalledWith("entity-unlink-test"));

    fireEvent.click(screen.getByRole("button", { name: "Relationship changed" }));
    await waitFor(() =>
      expect(projectedRefetchMock).toHaveBeenCalledWith({ forceRefresh: true }),
    );
    expect(projectedRefetchMock).toHaveBeenCalledTimes(3);
  });

  it("shows a composition empty state when no projected sections are renderable", () => {
    useProjectedCompositionMock.mockReturnValue({
      loading: false,
      error: null,
      data: response(projection({ sections: [], blocks: [] })),
      renderedProvenance: { surface: "tauri_app", value: { ok: true } },
      refetch: vi.fn(),
    });

    render(<PersonDetailEditorial />);

    expect(screen.getByTestId("editorial-empty")).toHaveTextContent("No person composition");
  });

  it("shows loading while the first projected composition is pending", () => {
    useProjectedCompositionMock.mockReturnValue({
      loading: true,
      error: null,
      data: null,
      renderedProvenance: null,
      refetch: vi.fn(),
    });

    render(<PersonDetailEditorial />);

    expect(screen.getByTestId("editorial-loading")).toBeInTheDocument();
  });
});
