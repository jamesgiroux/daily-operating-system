/** @vitest-environment jsdom */

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ActionDetailPage from "@/pages/ActionDetailPage";
import type {
  ProjectedBlock,
  ProjectedComposition,
  ProjectedCompositionCommandResponse,
} from "@/services/composition/contracts";
import type { ActionDetail } from "@/types";

const {
  invokeMock,
  navigateMock,
  projectedRefetchMock,
  useChapterLayoutMock,
  useProjectedCompositionMock,
  useRegisterMagazineShellMock,
} = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  navigateMock: vi.fn(),
  projectedRefetchMock: vi.fn(),
  useChapterLayoutMock: vi.fn(),
  useProjectedCompositionMock: vi.fn(),
  useRegisterMagazineShellMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tanstack/react-router", () => ({
  useParams: () => ({ actionId: "action-test-1" }),
  useNavigate: () => navigateMock,
  Link: ({ children, to, className }: { children: ReactNode; to: string; className?: string }) => (
    <a href={to} className={className}>
      {children}
    </a>
  ),
}));

vi.mock("sonner", () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
  },
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
  useChapterLayout: (args: { projection: ProjectedComposition | null; entityType: string }) => {
    useChapterLayoutMock(args);
    const blockLabel = (block: ProjectedBlock | undefined) => {
      const value = block?.payload.title ?? block?.payload.text;
      return typeof value === "string" ? value : block?.block_id ?? "";
    };
    const sections =
      args.projection?.sections.map((section) => ({
        section,
        label: section.label ?? section.section_id,
        coreLocked: section.section_id === "headline",
        blocks: section.block_indexes.map((index) => {
          const block = args.projection?.blocks[index] as ProjectedBlock;
          return {
            block,
            sectionId: section.section_id,
            label: blockLabel(block),
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
      {typeof block.payload.title === "string"
        ? block.payload.title
        : typeof block.payload.text === "string"
          ? block.payload.text
          : block.block_id}
    </div>
  ),
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

vi.mock("@/components/ui/EditableText", () => ({
  EditableText: ({ value, onChange }: { value: string; onChange: (value: string) => void }) => (
    <button type="button" data-testid="editable-title" onClick={() => onChange("Renamed action")}>
      {value}
    </button>
  ),
}));

vi.mock("@/components/ui/editable-textarea", () => ({
  EditableTextarea: ({
    value,
    onSave,
    placeholder,
  }: {
    value: string;
    onSave: (value: string) => void;
    placeholder?: string;
  }) => (
    <button type="button" data-testid="editable-context" onClick={() => onSave("Updated context")}>
      {value || placeholder}
    </button>
  ),
}));

vi.mock("@/components/ui/editable-date", () => ({
  EditableDate: ({ value, onSave }: { value: string; onSave: (value: string) => void }) => (
    <button type="button" data-testid="editable-due" onClick={() => onSave("2026-06-12")}>
      {value || "Add due date"}
    </button>
  ),
}));

vi.mock("@/components/ui/editable-inline", () => ({
  EditableInline: ({
    value,
    onSave,
    placeholder,
  }: {
    value: string;
    onSave: (value: string) => void;
    placeholder?: string;
  }) => (
    <button type="button" data-testid="editable-source" onClick={() => onSave("Manual source")}>
      {value || placeholder}
    </button>
  ),
}));

vi.mock("@/components/ui/entity-picker", () => ({
  EntityPicker: ({
    onChange,
    placeholder,
  }: {
    onChange: (id: string | null, name?: string, entityType?: "account" | "project") => void;
    placeholder?: string;
  }) => (
    <button type="button" data-testid="account-picker" onClick={() => onChange("acct-new", "New Account", "account")}>
      {placeholder}
    </button>
  ),
}));

vi.mock("@/components/ui/priority-picker", () => ({
  PriorityPicker: ({ onChange }: { value: number; onChange: (priority: number) => void }) => (
    <button type="button" data-testid="priority-picker" onClick={() => onChange(2)}>
      Set high priority
    </button>
  ),
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
      action: { title: "Projected Action" },
      title: "Projected Action",
    }),
    block("status-block", 1, "claim_summary", { text: "Open and owned" }),
    block("priority-block", 2, "risk_callout", { title: "High priority" }),
    block("context-block", 3, "claim_summary", { text: "Customer context" }),
    block("reference-block", 4, "evidence_list", { title: "References" }),
    block("linear-block", 5, "evidence_list", { title: "Linear issue" }),
    block("action-bar-block", 6, "action_list", { title: "Next action" }),
  ];
  return {
    composition_id: "dailyos/action-detail:action:action-test-1",
    composition_version: 1,
    fallback_policy_version: 3,
    sections: [
      {
        section_id: "headline",
        section_index: 0,
        label: "Headline",
        layout: "stacked",
        salience: { weight: 0.95, band: "critical", reason: "action masthead" },
        block_ids: ["headline-block"],
        block_indexes: [0],
      },
      {
        section_id: "status",
        section_index: 1,
        label: "Status",
        layout: "stacked",
        salience: { weight: 0.86, band: "important", reason: "action status" },
        block_ids: ["status-block"],
        block_indexes: [1],
      },
      {
        section_id: "priority",
        section_index: 2,
        label: "Priority",
        layout: "stacked",
        salience: { weight: 0.78, band: "important", reason: "action priority" },
        block_ids: ["priority-block"],
        block_indexes: [2],
      },
      {
        section_id: "context",
        section_index: 3,
        label: "Context",
        layout: "stacked",
        salience: { weight: 0.7, band: "contextual", reason: "action context" },
        block_ids: ["context-block"],
        block_indexes: [3],
      },
      {
        section_id: "reference",
        section_index: 4,
        label: "Reference",
        layout: "stacked",
        salience: { weight: 0.6, band: "contextual", reason: "action reference" },
        block_ids: ["reference-block"],
        block_indexes: [4],
      },
      {
        section_id: "linear",
        section_index: 5,
        label: "Linear",
        layout: "stacked",
        salience: { weight: 0.55, band: "background", reason: "linear link" },
        block_ids: ["linear-block"],
        block_indexes: [5],
      },
      {
        section_id: "action-bar",
        section_index: 6,
        label: "Action bar",
        layout: "stacked",
        salience: { weight: 0.75, band: "important", reason: "action controls" },
        block_ids: ["action-bar-block"],
        block_indexes: [6],
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

function actionDetail(overrides: Partial<ActionDetail> = {}): ActionDetail {
  return {
    id: "action-test-1",
    title: "Detail Action",
    priority: 3,
    status: "unstarted",
    createdAt: "2026-06-01T14:00:00Z",
    dueDate: undefined,
    completedAt: undefined,
    accountId: undefined,
    accountName: undefined,
    projectId: undefined,
    sourceType: "manual",
    sourceId: undefined,
    sourceLabel: undefined,
    actionKind: "task",
    commitmentId: undefined,
    ownerRaw: undefined,
    ownerEntityId: undefined,
    ownerConfidence: undefined,
    ownerSource: undefined,
    trustScore: undefined,
    trustBand: undefined,
    commitmentSourceCount: undefined,
    context: "Existing context",
    waitingOn: undefined,
    updatedAt: "2026-06-01T14:00:00Z",
    personId: undefined,
    nextMeetingTitle: undefined,
    nextMeetingStart: undefined,
    needsDecision: undefined,
    decisionOwner: undefined,
    decisionStakes: undefined,
    linearIdentifier: undefined,
    linearUrl: undefined,
    sourceMeetingTitle: undefined,
    ...overrides,
  };
}

describe("ActionDetailPage", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    navigateMock.mockReset();
    projectedRefetchMock.mockReset();
    projectedRefetchMock.mockResolvedValue(undefined);
    useChapterLayoutMock.mockReset();
    useProjectedCompositionMock.mockReset();
    useRegisterMagazineShellMock.mockReset();
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_action_detail") return Promise.resolve(actionDetail());
      if (command === "get_linear_status") return Promise.resolve({ enabled: true, apiKeySet: true });
      if (command === "get_linear_teams") return Promise.resolve([{ id: "team-1", name: "Team One" }]);
      if (command === "push_action_to_linear") {
        return Promise.resolve({ identifier: "DOS-1", url: "https://linear.example/DOS-1" });
      }
      return Promise.resolve(null);
    });
    useProjectedCompositionMock.mockReturnValue({
      loading: false,
      error: null,
      data: response(projection()),
      renderedProvenance: { surface: "tauri_app", value: { ok: true } },
      refetch: projectedRefetchMock,
    });
  });

  it("renders Action Detail from the projected composition and derives shell chapters", async () => {
    render(<ActionDetailPage />);

    expect(useProjectedCompositionMock).toHaveBeenCalledWith({
      entityType: "action",
      entityId: "action-test-1",
    });
    expect(useChapterLayoutMock).toHaveBeenCalledWith(
      expect.objectContaining({ entityType: "action" }),
    );
    expect(screen.getByText("Projected Action")).toBeInTheDocument();
    expect(screen.getByText("Open and owned")).toBeInTheDocument();
    expect(screen.getByText("High priority")).toBeInTheDocument();
    expect(screen.getByText("Customer context")).toBeInTheDocument();
    expect(await screen.findByTestId("editable-title")).toHaveTextContent("Detail Action");

    for (const renderedBlock of screen.getAllByTestId("composition-block")) {
      expect(renderedBlock).toHaveAttribute("data-entity-id", "action-test-1");
      expect(renderedBlock).toHaveAttribute("data-entity-type", "action");
    }

    const shellConfig = useRegisterMagazineShellMock.mock.calls.at(-1)?.[0] as {
      chapters: Array<{ id: string; label: string }>;
    };
    expect(shellConfig.chapters.map((chapter) => chapter.id)).toEqual([
      "headline",
      "status",
      "priority",
      "context",
      "reference",
      "linear",
      "action-bar",
    ]);
  });

  it("preserves existing action edit controls through their Tauri commands", async () => {
    render(<ActionDetailPage />);

    fireEvent.click(await screen.findByTestId("editable-title"));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("update_action", {
        request: { id: "action-test-1", title: "Renamed action" },
      }),
    );

    fireEvent.click(screen.getByTestId("priority-picker"));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("update_action", {
        request: { id: "action-test-1", priority: 2 },
      }),
    );

    fireEvent.click(screen.getByTestId("editable-context"));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("update_action", {
        request: { id: "action-test-1", context: "Updated context" },
      }),
    );

    fireEvent.click(screen.getByTestId("editable-due"));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("update_action", {
        request: { id: "action-test-1", dueDate: "2026-06-12" },
      }),
    );

    fireEvent.click(screen.getByTestId("account-picker"));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("update_action", {
        request: { id: "action-test-1", accountId: "acct-new" },
      }),
    );

    fireEvent.click(screen.getByTestId("editable-source"));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("update_action", {
        request: { id: "action-test-1", sourceLabel: "Manual source" },
      }),
    );

    fireEvent.change(await screen.findByRole("combobox"), { target: { value: "team-1" } });
    fireEvent.click(screen.getByRole("button", { name: "Create Linear Issue" }));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("push_action_to_linear", {
        actionId: "action-test-1",
        teamId: "team-1",
        title: "Detail Action",
      }),
    );

    fireEvent.click(screen.getByRole("button", { name: "Mark Complete" }));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("complete_action", { id: "action-test-1" }),
    );
    expect(projectedRefetchMock).toHaveBeenCalledWith({ forceRefresh: true });
  });

  it("shows producer errors without falling back to the old template", () => {
    useProjectedCompositionMock.mockReturnValue({
      loading: false,
      error: "action producer unavailable",
      data: null,
      renderedProvenance: null,
      refetch: vi.fn(),
    });

    render(<ActionDetailPage />);

    expect(screen.getByTestId("editorial-error")).toHaveTextContent("action producer unavailable");
    expect(screen.queryByTestId("composition-block")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Action controls")).not.toBeInTheDocument();
  });

  it("shows a composition empty state when no projected sections are renderable", () => {
    useProjectedCompositionMock.mockReturnValue({
      loading: false,
      error: null,
      data: response(projection({ sections: [], blocks: [] })),
      renderedProvenance: { surface: "tauri_app", value: { ok: true } },
      refetch: vi.fn(),
    });

    render(<ActionDetailPage />);

    expect(screen.getByTestId("editorial-empty")).toHaveTextContent("No action composition");
  });

  it("shows loading while the first projected composition is pending", () => {
    useProjectedCompositionMock.mockReturnValue({
      loading: true,
      error: null,
      data: null,
      renderedProvenance: null,
      refetch: vi.fn(),
    });

    render(<ActionDetailPage />);

    expect(screen.getByTestId("editorial-loading")).toBeInTheDocument();
  });
});
