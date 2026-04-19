/** @vitest-environment jsdom */

import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SentimentHero } from "./SentimentHero";
import {
  DEFAULT_SENTIMENT_LABELS,
  type SentimentView,
} from "@/hooks/useAccountDetail";
import type { HealthSparklinePoint, SentimentJournalEntry } from "@/types";

function makeView(overrides: Partial<SentimentView> = {}): SentimentView {
  return {
    current: "concerning",
    note: "Block editor issues piling up. Concerned we're losing the architect conversation.",
    setAt: new Date(Date.now() - 12 * 24 * 60 * 60 * 1000).toISOString(),
    history: [
      {
        sentiment: "concerning",
        note: "Block editor issues piling up.",
        setAt: new Date(Date.now() - 12 * 24 * 60 * 60 * 1000).toISOString(),
      } as SentimentJournalEntry,
    ],
    sparkline: makeSparkline(),
    divergence: null,
    isStale: false,
    presetLabels: DEFAULT_SENTIMENT_LABELS,
    ...overrides,
  };
}

function makeSparkline(): HealthSparklinePoint[] {
  const out: HealthSparklinePoint[] = [];
  const bands = ["green", "green", "yellow", "green", "green", "yellow", "yellow"];
  for (let day = 0; day < 90; day++) {
    const bucket = Math.min(6, Math.floor(day / (90 / 7)));
    out.push({
      day: new Date(Date.now() - (89 - day) * 86400000)
        .toISOString()
        .slice(0, 10),
      score: 50 + (day % 30),
      band: bands[bucket]!,
    });
  }
  return out;
}

describe("SentimentHero", () => {
  it("renders the Your Assessment label and current sentiment pill", () => {
    render(
      <SentimentHero
        view={makeView()}
        onSetSentiment={vi.fn().mockResolvedValue(undefined)}
        onAcknowledgeStale={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    expect(screen.getByText("Your Assessment")).toBeTruthy();
    expect(screen.getByText("Concerning")).toBeTruthy();
  });

  it("renders exactly 7 sparkline bars for a 90-day dataset", () => {
    const { container } = render(
      <SentimentHero
        view={makeView()}
        onSetSentiment={vi.fn().mockResolvedValue(undefined)}
        onAcknowledgeStale={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    const spark = container.querySelector(
      "[aria-label='Computed health over last 90 days']",
    );
    expect(spark).toBeTruthy();
    expect(spark!.children.length).toBe(7);
  });

  it("renders the meta line with set-days and note count", () => {
    render(
      <SentimentHero
        view={makeView()}
        onSetSentiment={vi.fn().mockResolvedValue(undefined)}
        onAcknowledgeStale={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    expect(screen.getByText(/Set 12 days ago/)).toBeTruthy();
    expect(screen.getByText(/1 note/)).toBeTruthy();
  });

  it("renders the Still accurate? button unconditionally in the meta line", () => {
    // Per the mockup (lines 622 / 467 of .docs/mockups/account-health-*.html),
    // "Still accurate?" is always present alongside the set-date and note
    // count — it's a zero-pressure prompt, not a staleness escalation.
    const { rerender } = render(
      <SentimentHero
        view={makeView({ isStale: false })}
        onSetSentiment={vi.fn().mockResolvedValue(undefined)}
        onAcknowledgeStale={vi.fn().mockResolvedValue(undefined)}
      />,
    );
    expect(screen.getByText("Still accurate?")).toBeTruthy();

    rerender(
      <SentimentHero
        view={makeView({ isStale: true })}
        onSetSentiment={vi.fn().mockResolvedValue(undefined)}
        onAcknowledgeStale={vi.fn().mockResolvedValue(undefined)}
      />,
    );
    expect(screen.getByText("Still accurate?")).toBeTruthy();
  });

  it("calls onAcknowledgeStale when Still accurate? is clicked", () => {
    const onAck = vi.fn().mockResolvedValue(undefined);
    render(
      <SentimentHero
        view={makeView({ isStale: true })}
        onSetSentiment={vi.fn().mockResolvedValue(undefined)}
        onAcknowledgeStale={onAck}
      />,
    );

    fireEvent.click(screen.getByText("Still accurate?"));
    expect(onAck).toHaveBeenCalledTimes(1);
  });

  it("renders the pull quote with attribution", () => {
    render(
      <SentimentHero
        view={makeView()}
        onSetSentiment={vi.fn().mockResolvedValue(undefined)}
        onAcknowledgeStale={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    expect(screen.getByText(/Your note,/)).toBeTruthy();
  });

  it("renders the divergence flag only when view.divergence is set", () => {
    const { rerender } = render(
      <SentimentHero
        view={makeView({ divergence: null })}
        onSetSentiment={vi.fn().mockResolvedValue(undefined)}
        onAcknowledgeStale={vi.fn().mockResolvedValue(undefined)}
      />,
    );
    expect(screen.queryByText("Updates currently disagree")).toBeNull();

    rerender(
      <SentimentHero
        view={makeView({
          divergence: { severity: "minor", computedBand: "green", delta: 2 },
        })}
        onSetSentiment={vi.fn().mockResolvedValue(undefined)}
        onAcknowledgeStale={vi.fn().mockResolvedValue(undefined)}
      />,
    );
    expect(screen.getByText("Updates currently disagree")).toBeTruthy();
    expect(screen.getByText(/Add more detail/)).toBeTruthy();
  });

  it("opens the editor when Update is clicked", () => {
    render(
      <SentimentHero
        view={makeView()}
        onSetSentiment={vi.fn().mockResolvedValue(undefined)}
        onAcknowledgeStale={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    fireEvent.click(screen.getByText("Update"));
    expect(screen.getByPlaceholderText(/Add a journal note/)).toBeTruthy();
  });

  it("renders the unset-state prompt when view.current is null", () => {
    render(
      <SentimentHero
        view={makeView({ current: null })}
        onSetSentiment={vi.fn().mockResolvedValue(undefined)}
        onAcknowledgeStale={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    expect(
      screen.getByText(/strong, on track, concerning, at risk, or critical/),
    ).toBeTruthy();
  });
});
