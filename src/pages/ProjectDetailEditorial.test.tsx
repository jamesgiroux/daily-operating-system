/** @vitest-environment jsdom */

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ProjectDetailEditorial from "@/pages/ProjectDetailEditorial";
import type {
  ProjectedBlock,
  ProjectedComposition,
  ProjectedCompositionCommandResponse,
} from "@/services/composition/contracts";

const {
  invokeMock,
  navigateMock,
  useProjectDetailMock,
  useProjectedCompositionMock,
  projectedRefetchMock,
  useRegisterMagazineShellMock,
} = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  navigateMock: vi.fn(),
  useProjectDetailMock: vi.fn(),
  useProjectedCompositionMock: vi.fn(),
  projectedRefetchMock: vi.fn(),
  useRegisterMagazineShellMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tanstack/react-router", () => ({
  useParams: () => ({ projectId: "project-test-1" }),
  useNavigate: () => navigateMock,
}));

vi.mock("@/hooks/useProjectDetail", () => ({
  useProjectDetail: () => useProjectDetailMock(),
}));

vi.mock("@/hooks/useProjectedComposition", () => ({
  useProjectedComposition: (subject: unknown) => useProjectedCompositionMock(subject),
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

vi.mock("@/components/entity/LinearIssuesChapter", () => ({
  LinearIssuesChapter: () => <section id="linear-issues" data-testid="linear-issues" />,
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
      project: { display_name: "Projected Project" },
      title: "Projected Project",
    }),
    block("trajectory-block", 1, "claim_summary", { text: "Trajectory signal" }),
    block("work-block", 2, "action_list", { title: "Actions" }),
  ];
  return {
    composition_id: "dailyos/project-overview:project:project-test-1",
    composition_version: 3,
    fallback_policy_version: 3,
    sections: [
      {
        section_id: "headline",
        section_index: 0,
        label: "Headline",
        layout: "stacked",
        salience: { weight: 0.95, band: "critical", reason: "project masthead" },
        block_ids: ["headline-block"],
        block_indexes: [0],
      },
      {
        section_id: "trajectory",
        section_index: 1,
        label: "Trajectory",
        layout: "stacked",
        salience: { weight: 0.8, band: "important", reason: "project trajectory" },
        block_ids: ["trajectory-block"],
        block_indexes: [1],
      },
      {
        section_id: "the-work",
        section_index: 2,
        label: "The work",
        layout: "stacked",
        salience: { weight: 0.5, band: "background", reason: "project work" },
        block_ids: ["work-block"],
        block_indexes: [2],
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

function projectHook(overrides: Record<string, unknown> = {}) {
  return {
    detail: {
      id: "project-test-1",
      name: "Detail Project",
      status: "active",
      milestone: "Beta",
      owner: "Owner",
      targetDate: "",
      openActionCount: 0,
      archived: false,
      childCount: 1,
      isParent: true,
      milestones: [],
      openActions: [],
      recentMeetings: [],
      linkedPeople: [],
      recentCaptures: [],
      children: [{ id: "project-child-1", name: "Child Project", status: "active", openActionCount: 0 }],
    },
    intelligence: null,
    loading: false,
    error: null,
    files: [],
    load: vi.fn(),
    silentRefresh: vi.fn(),
    editName: "Detail Project",
    setEditName: vi.fn(),
    editStatus: "active",
    setEditStatus: vi.fn(),
    editMilestone: "Beta",
    setEditMilestone: vi.fn(),
    editOwner: "Owner",
    setEditOwner: vi.fn(),
    editTargetDate: "",
    setEditTargetDate: vi.fn(),
    dirty: false,
    setDirty: vi.fn(),
    saving: false,
    handleSave: vi.fn(),
    saveField: vi.fn(),
    handleCancelEdit: vi.fn(),
    enriching: false,
    enrichSeconds: 0,
    handleEnrich: vi.fn(),
    handleArchive: vi.fn(),
    handleUnarchive: vi.fn(),
    addingAction: false,
    setAddingAction: vi.fn(),
    newActionTitle: "",
    setNewActionTitle: vi.fn(),
    creatingAction: false,
    handleCreateAction: vi.fn(),
    createChildOpen: false,
    setCreateChildOpen: vi.fn(),
    childName: "",
    setChildName: vi.fn(),
    childDescription: "",
    setChildDescription: vi.fn(),
    creatingChild: false,
    handleCreateChild: vi.fn(),
    indexing: false,
    indexFeedback: null,
    handleIndexFiles: vi.fn(),
    ...overrides,
  };
}

describe("ProjectDetailEditorial", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue([]);
    navigateMock.mockReset();
    useRegisterMagazineShellMock.mockReset();
    projectedRefetchMock.mockReset();
    projectedRefetchMock.mockResolvedValue(undefined);
    useProjectDetailMock.mockReturnValue(projectHook());
    useProjectedCompositionMock.mockReturnValue({
      loading: false,
      error: null,
      data: response(projection()),
      renderedProvenance: { surface: "tauri_app", value: { ok: true } },
      refetch: projectedRefetchMock,
    });
  });

  it("renders Project Detail from the projected composition and derives shell chapters", () => {
    render(<ProjectDetailEditorial />);

    expect(useProjectedCompositionMock).toHaveBeenCalledWith({
      entityType: "project",
      entityId: "project-test-1",
    });
    expect(screen.getByText("Projected Project")).toBeInTheDocument();
    expect(screen.getByText("Trajectory signal")).toBeInTheDocument();
    expect(screen.getByText("Actions")).toBeInTheDocument();
    expect(screen.getByTestId("linear-issues")).toBeInTheDocument();

    for (const renderedBlock of screen.getAllByTestId("composition-block")) {
      expect(renderedBlock).toHaveAttribute("data-entity-id", "project-test-1");
      expect(renderedBlock).toHaveAttribute("data-entity-type", "project");
    }

    const shellConfig = useRegisterMagazineShellMock.mock.calls.at(-1)?.[0] as {
      chapters: Array<{ id: string; label: string }>;
    };
    expect(shellConfig.chapters.map((chapter) => chapter.id)).toEqual([
      "headline",
      "trajectory",
      "the-work",
      "linear-issues",
    ]);
  });

  it("shows producer errors without falling back to the old template", () => {
    useProjectedCompositionMock.mockReturnValue({
      loading: false,
      error: "project producer unavailable",
      data: null,
      renderedProvenance: null,
      refetch: vi.fn(),
    });

    render(<ProjectDetailEditorial />);

    expect(screen.getByTestId("editorial-error")).toHaveTextContent("project producer unavailable");
    expect(screen.queryByTestId("composition-block")).not.toBeInTheDocument();
  });

  it("force-refreshes projected composition after project controls save", async () => {
    const handleSave = vi.fn().mockResolvedValue(undefined);
    useProjectDetailMock.mockReturnValue(projectHook({ dirty: true, handleSave }));

    render(<ProjectDetailEditorial />);

    fireEvent.click(screen.getByRole("button", { name: /Save/i }));

    await waitFor(() => expect(handleSave).toHaveBeenCalled());
    expect(projectedRefetchMock).toHaveBeenCalledWith({ forceRefresh: true });
  });

  it("shows a composition empty state when no projected sections are renderable", () => {
    useProjectedCompositionMock.mockReturnValue({
      loading: false,
      error: null,
      data: response(projection({ sections: [], blocks: [] })),
      renderedProvenance: { surface: "tauri_app", value: { ok: true } },
      refetch: vi.fn(),
    });

    render(<ProjectDetailEditorial />);

    expect(screen.getByTestId("editorial-empty")).toHaveTextContent("No project composition");
  });

  it("shows loading while the first projected composition is pending", () => {
    useProjectedCompositionMock.mockReturnValue({
      loading: true,
      error: null,
      data: null,
      renderedProvenance: null,
      refetch: vi.fn(),
    });

    render(<ProjectDetailEditorial />);

    expect(screen.getByTestId("editorial-loading")).toBeInTheDocument();
  });
});
