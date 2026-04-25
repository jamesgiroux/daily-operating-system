/** @vitest-environment jsdom */

import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

// ── Hoisted mocks ──────────────────────────────────────────────────────────────

const { invokeMock, useAccountDetailMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  useAccountDetailMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("@tanstack/react-router", () => ({
  useParams: () => ({ accountId: "acct-test-1" }),
  useNavigate: () => vi.fn(),
  Link: ({ children, ...props }: Record<string, unknown>) => (
    <a href={String(props.to ?? "#")}>{children as React.ReactNode}</a>
  ),
}));

vi.mock("sonner", () => ({
  toast: { error: vi.fn(), success: vi.fn(), warning: vi.fn() },
}));

vi.mock("@/hooks/useAccountDetail", () => ({
  useAccountDetail: () => useAccountDetailMock(),
}));

vi.mock("@/hooks/useActivePreset", () => ({
  useActivePreset: () => ({
    preset: null,
    vitals: { account: [] },
    metadata: { account: [] },
  }),
}));

vi.mock("@/hooks/useIntelligenceFieldUpdate", () => ({
  useIntelligenceFieldUpdate: () => ({
    updateField: vi.fn(),
    saveStatus: "idle",
    setSaveStatus: vi.fn(),
  }),
}));

vi.mock("@/hooks/useRevealObserver", () => ({
  useRevealObserver: vi.fn(),
}));

vi.mock("@/hooks/useMagazineShell", () => ({
  useRegisterMagazineShell: vi.fn(),
  useUpdateFolioVolatile: vi.fn(),
}));

vi.mock("@/hooks/useIntelligenceFeedback", () => ({
  useIntelligenceFeedback: () => ({
    getFeedback: vi.fn(),
    submitFeedback: vi.fn(),
  }),
}));

vi.mock("@/hooks/useAccountFieldSave", () => ({
  useAccountFieldSave: () => ({
    saveMetadata: vi.fn(),
    saveAccountField: vi.fn(),
    conflictsForStrip: new Map(),
  }),
}));

vi.mock("@/hooks/useEntityContextEntries", () => ({
  useEntityContextEntries: () => ({
    entries: [],
    loading: false,
  }),
}));

vi.mock("@/hooks/useTauriEvent", () => ({
  useTauriEvent: () => {},
}));

vi.mock("@/lib/report-config", () => ({
  getAccountReports: () => [],
}));

// Stub out heavy child components to isolate page-level logic
vi.mock("@/components/account/AccountHero", () => ({
  AccountHero: ({ detail }: { detail: { name: string } }) => (
    <div data-testid="account-hero">{detail?.name}</div>
  ),
}));

vi.mock("@/components/entity/VitalsStrip", () => ({
  VitalsStrip: () => <div data-testid="vitals-strip" />,
}));

vi.mock("@/components/entity/EditableVitalsStrip", () => ({
  EditableVitalsStrip: () => <div data-testid="editable-vitals-strip" />,
}));

vi.mock("@/components/entity/StateOfPlay", () => ({
  StateOfPlay: () => <div data-testid="state-of-play" />,
}));

vi.mock("@/components/entity/StakeholderGallery", () => ({
  StakeholderGallery: () => <div data-testid="stakeholder-gallery" />,
}));

vi.mock("@/components/entity/WatchList", () => ({
  WatchList: () => <div data-testid="watch-list" />,
}));

vi.mock("@/components/entity/UnifiedTimeline", () => ({
  UnifiedTimeline: () => <div data-testid="unified-timeline" />,
}));

vi.mock("@/components/entity/TheWork", () => ({
  TheWork: () => <div data-testid="the-work" />,
}));

vi.mock("@/components/entity/ValueCommitments", () => ({
  ValueCommitments: () => <div data-testid="value-commitments" />,
}));

vi.mock("@/components/entity/StrategicLandscape", () => ({
  StrategicLandscape: () => <div data-testid="strategic-landscape" />,
}));

vi.mock("@/components/entity/AccountOutlook", () => ({
  AccountOutlook: () => <div data-testid="account-outlook" />,
}));

vi.mock("@/components/editorial/ChapterHeading", () => ({
  ChapterHeading: ({ title }: { title: string }) => <h2>{title}</h2>,
}));

vi.mock("@/components/editorial/FinisMarker", () => ({
  FinisMarker: () => <div data-testid="finis-marker" />,
}));

vi.mock("@/components/editorial/MarginSection", () => ({
  MarginSection: ({ children }: { children: React.ReactNode }) => <div data-testid="margin-section">{children}</div>,
}));

vi.mock("@/components/editorial/EditorialLoading", () => ({
  EditorialLoading: () => <div data-testid="editorial-loading">Loading...</div>,
}));

vi.mock("@/components/editorial/EditorialError", () => ({
  EditorialError: ({ message, onRetry }: { message: string; onRetry?: () => void }) => (
    <div data-testid="editorial-error">
      <span>{message}</span>
      {onRetry && <button onClick={onRetry}>Retry</button>}
    </div>
  ),
}));

vi.mock("@/components/entity/AddToRecord", () => ({
  AddToRecord: () => null,
}));

vi.mock("@/components/entity/FileListSection", () => ({
  FileListSection: () => null,
}));

vi.mock("@/components/account/AccountMergeDialog", () => ({
  AccountMergeDialog: () => null,
}));

vi.mock("@/components/account/WatchListPrograms", () => ({
  WatchListPrograms: () => null,
}));

vi.mock("@/components/account/AccountBreadcrumbs", () => ({
  AccountBreadcrumbs: () => null,
}));

vi.mock("@/components/account/AccountRolloverPrompt", () => ({
  AccountRolloverPrompt: () => null,
}));

vi.mock("@/components/account/AccountProductsSection", () => ({
  AccountProductsSection: () => <div data-testid="products-section" />,
}));

vi.mock("@/components/account/AccountPortfolioSection", () => ({
  AccountPortfolioSection: () => <div data-testid="portfolio-section" />,
}));

vi.mock("@/components/account/AccountHealthSection", () => ({
  AccountHealthSection: () => <div data-testid="health-section" />,
}));

vi.mock("@/components/account/AccountPullQuote", () => ({
  AccountPullQuote: () => null,
}));

vi.mock("@/components/account/AccountTechnicalFootprint", () => ({
  AccountTechnicalFootprint: () => null,
}));

vi.mock("@/components/account/AccountReportsSection", () => ({
  AccountReportsSection: () => <div data-testid="reports-section" />,
}));

vi.mock("@/components/account/AccountDialogs", () => ({
  AccountDialogs: () => null,
}));

vi.mock("@/components/shared/DimensionBar", () => ({
  DimensionBar: () => null,
}));

vi.mock("@/components/ui/ProvenanceLabel", () => ({
  formatProvenanceSource: (s: string) => s,
}));

vi.mock("@/components/ui/IntelligenceFeedback", () => ({
  IntelligenceFeedback: () => null,
}));

vi.mock("@/components/ui/folio-refresh-button", () => ({
  FolioRefreshButton: () => null,
}));

vi.mock("@/components/folio/FolioReportsDropdown", () => ({
  FolioReportsDropdown: () => null,
}));

vi.mock("@/components/folio/FolioToolsDropdown", () => ({
  FolioToolsDropdown: () => null,
}));

// ── Mock helper ────────────────────────────────────────────────────────────────

/**
 * Build a complete useAccountDetail mock return value.
 * The hook exposes many fields; this helper ensures all are present.
 */
function makeAccountHookMock(overrides: Record<string, unknown> = {}) {
  return {
    loading: false,
    error: null,
    detail: null,
    intelligence: null,
    events: [],
    files: [],
    load: vi.fn(),
    silentRefresh: vi.fn(),
    handleEnrich: vi.fn(),
    enriching: false,
    enrichSeconds: 0,
    enrichmentPercentage: null,
    // Child account creation
    createChildOpen: false,
    setCreateChildOpen: vi.fn(),
    childName: "",
    setChildName: vi.fn(),
    childDescription: "",
    setChildDescription: vi.fn(),
    childOwnerId: "",
    setChildOwnerId: vi.fn(),
    creatingChild: false,
    handleCreateChild: vi.fn(),
    // Action creation
    addingAction: false,
    setAddingAction: vi.fn(),
    newActionTitle: "",
    setNewActionTitle: vi.fn(),
    creatingAction: false,
    handleCreateAction: vi.fn(),
    // Indexing
    indexing: false,
    newFileCount: 0,
    bannerDismissed: false,
    setBannerDismissed: vi.fn(),
    indexFeedback: null,
    handleIndexFiles: vi.fn(),
    // Editing
    editName: "",
    setEditName: vi.fn(),
    editHealth: "",
    setEditHealth: vi.fn(),
    editLifecycle: "",
    setEditLifecycle: vi.fn(),
    dirty: false,
    setDirty: vi.fn(),
    handleSave: vi.fn(),
    // Team
    teamSearchQuery: "",
    setTeamSearchQuery: vi.fn(),
    teamSearchResults: [],
    handleRemoveTeamMember: vi.fn(),
    changeTeamMemberRole: vi.fn(),
    addTeamMemberDirect: vi.fn(),
    createTeamMemberDirect: vi.fn(),
    // Programs
    programs: [],
    handleAddProgram: vi.fn(),
    handleProgramUpdate: vi.fn(),
    handleProgramDelete: vi.fn(),
    // Events
    handleRecordEvent: vi.fn(),
    setNewEventType: vi.fn(),
    setNewEventDate: vi.fn(),
    // Stakeholders
    suggestions: [],
    acceptSuggestion: vi.fn(),
    dismissSuggestion: vi.fn(),
    updateStakeholderEngagement: vi.fn(),
    updateStakeholderAssessment: vi.fn(),
    addStakeholderRole: vi.fn(),
    removeStakeholderRole: vi.fn(),
    // Archive
    handleArchive: vi.fn(),
    handleUnarchive: vi.fn(),
    ...overrides,
  };
}

// ── Setup ──────────────────────────────────────────────────────────────────────

beforeEach(async () => {
  invokeMock.mockImplementation(async (command: string) => {
    switch (command) {
      case "get_entity_metadata":
        return "{}";
      case "get_account_products":
        return [];
      case "get_tracked_field_changes":
        return [];
      default:
        return null;
    }
  });
});

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("AccountDetailEditorial", () => {
  it("renders loading state", async () => {
    useAccountDetailMock.mockReturnValue(makeAccountHookMock({ loading: true }));

    const mod = await import("./AccountDetailEditorial");
    render(<mod.default />);

    expect(screen.getByTestId("editorial-loading")).toBeInTheDocument();
  });

  it("renders error state with message", async () => {
    useAccountDetailMock.mockReturnValue(
      makeAccountHookMock({ error: "Network error" }),
    );

    const mod = await import("./AccountDetailEditorial");
    render(<mod.default />);

    expect(screen.getByTestId("editorial-error")).toBeInTheDocument();
    expect(screen.getByText("Network error")).toBeInTheDocument();
  });

  it("renders account detail when data is loaded", async () => {
    useAccountDetailMock.mockReturnValue(
      makeAccountHookMock({
        detail: {
          id: "acct-test-1",
          name: "Acme Corporation",
          accountType: "customer",
          health: "green",
          arr: 250000,
          lifecycle: "growth",
          isParent: false,
          archived: false,
          linkedPeople: [],
          strategicPrograms: [],
          upcomingMeetings: [],
          recentMeetings: [],
          events: [],
          actions: [],
          accountTeam: [],
        },
      }),
    );

    const mod = await import("./AccountDetailEditorial");
    render(<mod.default />);

    expect(screen.getByTestId("account-hero")).toBeInTheDocument();
    expect(screen.getByText("Acme Corporation")).toBeInTheDocument();
  });

  it("renders 'Account not found' when detail is null without error", async () => {
    useAccountDetailMock.mockReturnValue(makeAccountHookMock());

    const mod = await import("./AccountDetailEditorial");
    render(<mod.default />);

    expect(screen.getByTestId("editorial-error")).toBeInTheDocument();
    expect(screen.getByText("Account not found")).toBeInTheDocument();
  });
});
