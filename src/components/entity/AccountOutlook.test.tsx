/** @vitest-environment jsdom */

import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AccountOutlook } from "./AccountOutlook";
import type { EntityIntelligence } from "@/types";

// ── Mocks ──────────────────────────────────────────────────────────────────────

vi.mock("@/components/ui/EditableText", () => ({
  EditableText: ({ value }: { value: string }) => <span>{value}</span>,
}));

vi.mock("@/components/ui/IntelligenceFeedback", () => ({
  IntelligenceFeedback: ({ onFeedback }: { value: string | null; onFeedback: (type: string) => void }) => (
    <div data-testid="intelligence-feedback">
      <button onClick={() => onFeedback("positive")}>thumbs up</button>
      <button onClick={() => onFeedback("negative")}>thumbs down</button>
    </div>
  ),
}));

vi.mock("@/components/ui/ProvenanceTag", () => ({
  ProvenanceTag: () => <span data-testid="provenance-tag" />,
}));

// ── Test Data ──────────────────────────────────────────────────────────────────

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

const renewalIntelligence = makeMinimalIntelligence({
  renewalOutlook: {
    confidence: "high",
    riskFactors: ["Executive sponsor departure", "Budget freeze in Q4"],
    expansionPotential: "moderate",
    recommendedStart: "2026-08-01",
  },
});

const expansionIntelligence = makeMinimalIntelligence({
  expansionSignals: [
    {
      opportunity: "Analytics add-on module shows strong interest from 3 departments",
      stage: "evaluating",
      arrImpact: 45000,
    },
    {
      opportunity: "Enterprise tier upgrade being discussed at executive level",
      stage: "exploring",
      arrImpact: 80000,
    },
  ],
});

const contractIntelligence = makeMinimalIntelligence({
  contractContext: {
    contractType: "Annual",
    autoRenew: true,
    renewalDate: "2027-06-15",
    currentArr: 125000,
  },
});

const fullIntelligence = makeMinimalIntelligence({
  renewalOutlook: renewalIntelligence.renewalOutlook,
  expansionSignals: expansionIntelligence.expansionSignals,
  contractContext: contractIntelligence.contractContext,
});

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("AccountOutlook", () => {
  it("returns null when intelligence has no outlook data", () => {
    const { container } = render(
      <AccountOutlook intelligence={makeMinimalIntelligence()} />,
    );
    expect(container.innerHTML).toBe("");
  });

  it("renders renewal section with confidence statement", () => {
    render(<AccountOutlook intelligence={renewalIntelligence} />);

    expect(screen.getByText(/Renewal confidence is/)).toBeInTheDocument();
    expect(screen.getByText("high")).toBeInTheDocument();
  });

  it("renders risk factors as list items", () => {
    render(<AccountOutlook intelligence={renewalIntelligence} />);

    expect(screen.getByText("Executive sponsor departure")).toBeInTheDocument();
    expect(screen.getByText("Budget freeze in Q4")).toBeInTheDocument();
  });

  it("renders expansion potential in the heading", () => {
    render(<AccountOutlook intelligence={renewalIntelligence} />);

    // expansionPotential appended as "— moderate"
    expect(screen.getByText(/moderate/)).toBeInTheDocument();
  });

  it("renders recommended start date", () => {
    render(<AccountOutlook intelligence={renewalIntelligence} />);

    // Date formatting depends on timezone; just verify the element exists
    const startEl = screen.getByText(/Start the conversation by/);
    expect(startEl).toBeInTheDocument();
    // Should contain a date string (July 31 or August 1 depending on timezone)
    expect(startEl.textContent).toMatch(/2026/);
  });

  it("renders expansion signals section", () => {
    render(<AccountOutlook intelligence={expansionIntelligence} />);

    expect(screen.getByText("Expansion Signals")).toBeInTheDocument();
    expect(screen.getByText(/Analytics add-on module/)).toBeInTheDocument();
    expect(screen.getByText(/Enterprise tier upgrade/)).toBeInTheDocument();
  });

  it("renders stage badges for expansion signals", () => {
    render(<AccountOutlook intelligence={expansionIntelligence} />);

    expect(screen.getByText("Evaluating")).toBeInTheDocument();
    expect(screen.getByText("Exploring")).toBeInTheDocument();
  });

  it("renders ARR impact for expansion signals", () => {
    render(<AccountOutlook intelligence={expansionIntelligence} />);

    expect(screen.getByText(/\+\$45K ARR/)).toBeInTheDocument();
    expect(screen.getByText(/\+\$80K ARR/)).toBeInTheDocument();
  });

  it("renders contract strip with details", () => {
    render(<AccountOutlook intelligence={contractIntelligence} />);

    expect(screen.getByText("Annual")).toBeInTheDocument();
    expect(screen.getByText("Yes")).toBeInTheDocument(); // auto-renew
    // Date formatting depends on timezone; verify a date cell exists with 2027
    const renewalCell = screen.getByText(/2027/);
    expect(renewalCell).toBeInTheDocument();
    expect(screen.getByText(/\$125K/)).toBeInTheDocument();
  });

  it("renders all three sections when all data present", () => {
    render(<AccountOutlook intelligence={fullIntelligence} />);

    expect(screen.getByText(/Renewal confidence is/)).toBeInTheDocument();
    expect(screen.getByText("Expansion Signals")).toBeInTheDocument();
    expect(screen.getByText("Annual")).toBeInTheDocument();
  });

  it("renders dismiss button on expansion signals when onUpdateField provided", () => {
    const onUpdateField = vi.fn();

    render(
      <AccountOutlook
        intelligence={expansionIntelligence}
        onUpdateField={onUpdateField}
      />,
    );

    const dismissButtons = screen.getAllByTitle("Dismiss");
    expect(dismissButtons.length).toBe(2);

    fireEvent.click(dismissButtons[0]);
    expect(onUpdateField).toHaveBeenCalledWith("expansionSignals[0].opportunity", "");
  });

  it("renders feedback controls when onItemFeedback provided", () => {
    const onItemFeedback = vi.fn();

    render(
      <AccountOutlook
        intelligence={renewalIntelligence}
        onUpdateField={vi.fn()}
        onItemFeedback={onItemFeedback}
        getItemFeedback={() => null}
      />,
    );

    const feedbackElements = screen.getAllByTestId("intelligence-feedback");
    expect(feedbackElements.length).toBeGreaterThan(0);
  });

  it("filters out expansion signals with empty opportunity text", () => {
    const intel = makeMinimalIntelligence({
      expansionSignals: [
        { opportunity: "", stage: "evaluating" },
        { opportunity: "Valid opportunity", stage: "exploring" },
      ],
    });

    render(<AccountOutlook intelligence={intel} />);

    expect(screen.getByText("Valid opportunity")).toBeInTheDocument();
    // Only one signal should render
    expect(screen.getAllByText("Exploring").length).toBe(1);
    expect(screen.queryByText("Evaluating")).not.toBeInTheDocument();
  });
});
