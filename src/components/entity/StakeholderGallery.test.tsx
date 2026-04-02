/** @vitest-environment jsdom */

import { render, screen, fireEvent } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { StakeholderGallery } from "./StakeholderGallery";
import type { StakeholderInsight, Person, AccountTeamMember, EntityIntelligence, StakeholderFull } from "@/types";

// ── Mocks ──────────────────────────────────────────────────────────────────────

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tanstack/react-router", () => ({
  Link: ({ children, ...props }: Record<string, unknown>) => (
    <a href={String(props.to ?? "#")}>{children as React.ReactNode}</a>
  ),
  useNavigate: () => vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: { error: vi.fn(), success: vi.fn() },
}));

vi.mock("@/components/editorial/ChapterHeading", () => ({
  ChapterHeading: ({ title, epigraph }: { title: string; epigraph?: string }) => (
    <div data-testid="chapter-heading">
      <h2>{title}</h2>
      {epigraph && <p>{epigraph}</p>}
    </div>
  ),
}));

vi.mock("@/components/ui/EditableText", () => ({
  EditableText: ({ value }: { value: string }) => <span>{value}</span>,
}));

vi.mock("@/components/ui/Avatar", () => ({
  Avatar: ({ name }: { name: string }) => <span data-testid="avatar">{name}</span>,
}));

vi.mock("./EngagementSelector", () => ({
  EngagementSelector: ({ value, onChange }: { value: string; onChange: (v: string) => void }) => (
    <select data-testid="engagement-selector" value={value} onChange={(e) => onChange(e.target.value)}>
      <option value="active">Active</option>
      <option value="passive">Passive</option>
    </select>
  ),
  getEngagementDisplay: () => ({ background: "#eee", color: "#333" }),
  getEngagementLabel: (v: string) => v,
}));

vi.mock("./TeamRoleSelector", () => ({
  TeamRoleSelector: ({ value, onChange }: { value: string; onChange: (v: string) => void }) => (
    <select data-testid="team-role-selector" value={value} onChange={(e) => onChange(e.target.value)}>
      <option value="associated">Associated</option>
      <option value="csm">CSM</option>
    </select>
  ),
  getTeamRoleDisplay: (v: string) => v,
}));

// ── Test Data ──────────────────────────────────────────────────────────────────

function makeStakeholder(overrides: Partial<StakeholderInsight> = {}): StakeholderInsight {
  return {
    name: "Jane Champion",
    role: "VP Engineering",
    assessment: "Strong champion who drives adoption across the org.",
    engagement: "active",
    ...overrides,
  };
}

function makeStakeholderFull(overrides: Partial<StakeholderFull> = {}): StakeholderFull {
  return {
    personId: "p-1",
    personName: "Jane Champion",
    personRole: "VP Engineering",
    stakeholderRole: "champion",
    roles: [],
    dataSource: "ai",
    engagement: "active",
    createdAt: "2026-03-20T00:00:00Z",
    assessment: "Strong champion who drives adoption across the org.",
    ...overrides,
  };
}

function makeMinimalIntelligence(overrides: Partial<EntityIntelligence> = {}): EntityIntelligence {
  return {
    version: 1,
    entityId: "acct-1",
    entityType: "account",
    enrichedAt: "2026-03-20T00:00:00Z",
    sourceFileCount: 3,
    sourceManifest: [],
    risks: [],
    recentWins: [],
    stakeholderInsights: [],
    ...overrides,
  };
}

const baseProps = {
  intelligence: null as EntityIntelligence | null,
  linkedPeople: [] as Person[],
};

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("StakeholderGallery", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("renders without crashing with minimal props — returns null when no data", () => {
    const { container } = render(
      <StakeholderGallery {...baseProps} />,
    );
    expect(container.innerHTML).toBe("");
  });

  it("renders stakeholder cards from stakeholdersFull", () => {
    const stakeholdersFull = [
      makeStakeholderFull({ personId: "p-1", personName: "Jane Champion" }),
      makeStakeholderFull({ personId: "p-2", personName: "Bob Exec", personRole: "CTO" }),
    ];

    render(
      <StakeholderGallery
        {...baseProps}
        stakeholdersFull={stakeholdersFull}
      />,
    );

    expect(screen.getAllByText("Jane Champion").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Bob Exec").length).toBeGreaterThan(0);
    expect(screen.getByText(/Two stakeholders shape/)).toBeInTheDocument();
  });

  it("renders role text for stakeholders", () => {
    const stakeholdersFull = [
      makeStakeholderFull({ personId: "p-1", personName: "Jane", personRole: "VP Engineering" }),
    ];

    render(
      <StakeholderGallery {...baseProps} stakeholdersFull={stakeholdersFull} />,
    );

    expect(screen.getByText("VP Engineering")).toBeInTheDocument();
  });

  it("renders empty state with add button when canEdit", () => {
    render(
      <StakeholderGallery
        {...baseProps}
        entityId="acct-1"
        entityType="account"
      />,
    );

    expect(screen.getByText("Add Stakeholder")).toBeInTheDocument();
  });

  it("shows engagement selector when onUpdateEngagement and engagement exists", () => {
    const stakeholdersFull = [
      makeStakeholderFull({ personId: "p-1", engagement: "active" }),
    ];

    render(
      <StakeholderGallery
        {...baseProps}
        stakeholdersFull={stakeholdersFull}
        entityId="acct-1"
        entityType="account"
        onUpdateEngagement={vi.fn()}
      />,
    );

    const selector = screen.getByTestId("engagement-selector");
    expect(selector).toBeInTheDocument();
  });

  it("renders Your Team section when accountTeam is provided", () => {
    const team: AccountTeamMember[] = [
      {
        accountId: "acct-1",
        personId: "person-10",
        personName: "CSM Sarah",
        personEmail: "sarah@internal.com",
        role: "csm",
        createdAt: "2026-01-01T00:00:00Z",
      },
    ];

    render(
      <StakeholderGallery
        {...baseProps}
        accountTeam={team}
        onRemoveTeamMember={vi.fn()}
      />,
    );

    expect(screen.getByText("Your Team")).toBeInTheDocument();
    expect(screen.getByText("CSM Sarah")).toBeInTheDocument();
  });

  it("shows coverage analysis strip", () => {
    const stakeholdersFull = [
      makeStakeholderFull({ personId: "p-1", engagement: "active", roles: [{ role: "champion", dataSource: "user" }] }),
      makeStakeholderFull({ personId: "p-2", personName: "Person 2", engagement: "unknown", roles: [] }),
    ];

    render(
      <StakeholderGallery {...baseProps} stakeholdersFull={stakeholdersFull} />,
    );

    expect(screen.getByText("1 of 2")).toBeInTheDocument();
    expect(screen.getByText("stakeholders with defined roles")).toBeInTheDocument();
  });

  it("renders linked people fallback when no intelligence stakeholders exist", () => {
    const linked: Person[] = [
      {
        id: "p-fallback",
        name: "Linked Lee",
        email: "lee@example.com",
        relationship: "external",
        meetingCount: 3,
        updatedAt: "2026-03-01T00:00:00Z",
        archived: false,
        role: "Director",
        organization: "Acme",
      },
    ];

    render(
      <StakeholderGallery {...baseProps} linkedPeople={linked} />,
    );

    expect(screen.getByText("Linked Lee")).toBeInTheDocument();
  });

  it("shows 'Show N more' button when more than 6 stakeholders", () => {
    const stakeholdersFull = Array.from({ length: 8 }, (_, i) =>
      makeStakeholderFull({ personId: `p-${i}`, personName: `Person ${i}` }),
    );

    render(
      <StakeholderGallery {...baseProps} stakeholdersFull={stakeholdersFull} />,
    );

    expect(screen.getByText("Show 2 more")).toBeInTheDocument();

    fireEvent.click(screen.getByText("Show 2 more"));
    // After expanding, all 8 should be visible
    expect(screen.getAllByText("Person 7").length).toBeGreaterThan(0);
  });

  it("renders assessment text with truncation", () => {
    const longAssessment = "A".repeat(200);
    const stakeholdersFull = [
      makeStakeholderFull({ personId: "p-1", assessment: longAssessment }),
    ];

    render(
      <StakeholderGallery {...baseProps} stakeholdersFull={stakeholdersFull} />,
    );

    expect(screen.getByText("Read more")).toBeInTheDocument();
  });
});
