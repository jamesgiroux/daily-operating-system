import { describe, it, expect } from "vitest";
import {
  computeMeetingDisplayState,
  type DisplayStateContext,
} from "./MeetingCard";
import type { Meeting } from "@/types";

function makeMeeting(overrides: Partial<Meeting> = {}): Meeting {
  return {
    id: "test-1",
    time: "09:00 AM",
    endTime: "10:00 AM",
    title: "Test Meeting",
    type: "internal",
    hasPrep: false,
    ...overrides,
  };
}

const defaultCtx: DisplayStateContext = {
  isPast: false,
  outcomesStatus: "none",
  isLive: false,
  hasInlinePrep: false,
  hasEnrichedPrep: false,
};

function ctx(overrides: Partial<DisplayStateContext> = {}): DisplayStateContext {
  return { ...defaultCtx, ...overrides };
}

describe("computeMeetingDisplayState", () => {
  describe("cancelled", () => {
    it("gates out all actions and badges except cancelled", () => {
      const meeting = makeMeeting({
        overlayStatus: "cancelled",
        type: "customer",
        hasPrep: true,
        prepFile: "prep.md",
      });
      const state = computeMeetingDisplayState(meeting, ctx({ isPast: true, outcomesStatus: "loaded" }));

      expect(state.primaryStatus).toBe("cancelled");
      expect(state.badges).toHaveLength(1);
      expect(state.badges[0].key).toBe("cancelled");
      expect(state.actions).toHaveLength(0);
      expect(state.showExpander).toBe(false);
      expect(state.title.lineThrough).toBe(true);
      expect(state.card.hoverEnabled).toBe(false);
    });

    it("cancelled + live → no gold ring", () => {
      const meeting = makeMeeting({ overlayStatus: "cancelled" });
      const state = computeMeetingDisplayState(meeting, ctx({ isLive: true }));

      expect(state.primaryStatus).toBe("cancelled");
      expect(state.dot.ringClass).toBe("");
      expect(state.dot.animate).toBe(false);
    });
  });

  describe("past meetings", () => {
    it("past + loaded outcomes → processed badge, no action buttons", () => {
      const meeting = makeMeeting();
      const state = computeMeetingDisplayState(meeting, ctx({ isPast: true, outcomesStatus: "loaded" }));

      expect(state.primaryStatus).toBe("processed");
      expect(state.badges).toHaveLength(1);
      expect(state.badges[0].key).toBe("processed");
      expect(state.badges[0].icon).toBe("check");
      expect(state.actions).toHaveLength(0);
      expect(state.showExpander).toBe(true);
    });

    it("past + loading → neither badge nor buttons (prevents flash)", () => {
      const meeting = makeMeeting();
      const state = computeMeetingDisplayState(meeting, ctx({ isPast: true, outcomesStatus: "loading" }));

      expect(state.primaryStatus).toBe("past-loading");
      expect(state.badges).toHaveLength(0);
      expect(state.actions).toHaveLength(0);
    });

    it("past + no outcomes → attach and outcomes buttons", () => {
      const meeting = makeMeeting();
      const state = computeMeetingDisplayState(meeting, ctx({ isPast: true, outcomesStatus: "none" }));

      expect(state.primaryStatus).toBe("past-unprocessed");
      expect(state.actions).toHaveLength(2);
      expect(state.actions.map(a => a.key)).toEqual(["attach-transcript", "capture-outcomes"]);
      expect(state.badges).toHaveLength(0);
    });
  });

  describe("live meetings", () => {
    it("adds gold ring without suppressing prep button", () => {
      const meeting = makeMeeting({ type: "customer", hasPrep: true, prepFile: "prep.md" });
      const state = computeMeetingDisplayState(meeting, ctx({ isLive: true }));

      expect(state.primaryStatus).toBe("has-prep");
      expect(state.card.className).toContain("ring-2 ring-primary/50");
      expect(state.actions).toHaveLength(1);
      expect(state.actions[0].key).toBe("view-prep");
    });

    it("live-only meeting gets live primaryStatus", () => {
      const meeting = makeMeeting({ type: "internal" });
      const state = computeMeetingDisplayState(meeting, ctx({ isLive: true }));

      expect(state.primaryStatus).toBe("live");
      expect(state.card.className).toContain("ring-2 ring-primary/50");
    });

    it("live dot has ring and animate", () => {
      const meeting = makeMeeting();
      const state = computeMeetingDisplayState(meeting, ctx({ isLive: true }));

      expect(state.dot.ringClass).toContain("ring-2");
      expect(state.dot.animate).toBe(true);
    });
  });

  describe("new meetings", () => {
    it("new → 'No prep available' badge", () => {
      const meeting = makeMeeting({ overlayStatus: "new", type: "customer" });
      const state = computeMeetingDisplayState(meeting, ctx());

      expect(state.primaryStatus).toBe("new");
      expect(state.badges).toHaveLength(1);
      expect(state.badges[0].label).toBe("No prep available");
      expect(state.actions).toHaveLength(0);
    });
  });

  describe("prep state", () => {
    it("has prep file → View Prep action with linkTo", () => {
      const meeting = makeMeeting({ type: "customer", hasPrep: true, prepFile: "01-customer-acme.md" });
      const state = computeMeetingDisplayState(meeting, ctx());

      expect(state.primaryStatus).toBe("has-prep");
      expect(state.actions).toHaveLength(1);
      expect(state.actions[0].key).toBe("view-prep");
      expect(state.actions[0].linkTo).toBe("01-customer-acme.md");
    });

    it("has prep file without enrichment → 'Limited prep' badge", () => {
      const meeting = makeMeeting({ type: "customer", hasPrep: true, prepFile: "prep.md" });
      const state = computeMeetingDisplayState(meeting, ctx({ hasEnrichedPrep: false }));

      expect(state.primaryStatus).toBe("has-prep");
      expect(state.badges.some(b => b.key === "limited-prep")).toBe(true);
      expect(state.badges.find(b => b.key === "limited-prep")?.label).toBe("Limited prep");
    });

    it("has prep file with enrichment → no 'Limited prep' badge", () => {
      const meeting = makeMeeting({ type: "customer", hasPrep: true, prepFile: "prep.md" });
      const state = computeMeetingDisplayState(meeting, ctx({ hasEnrichedPrep: true }));

      expect(state.primaryStatus).toBe("has-prep");
      expect(state.badges.some(b => b.key === "limited-prep")).toBe(false);
    });

    it("customer without prep → 'No prep' badge", () => {
      const meeting = makeMeeting({ type: "customer", hasPrep: false });
      const state = computeMeetingDisplayState(meeting, ctx());

      expect(state.primaryStatus).toBe("no-prep");
      expect(state.badges).toHaveLength(1);
      expect(state.badges[0].label).toBe("No prep");
    });

    it("internal without prep → default (no 'No prep' badge)", () => {
      const meeting = makeMeeting({ type: "internal", hasPrep: false });
      const state = computeMeetingDisplayState(meeting, ctx());

      expect(state.primaryStatus).toBe("default");
      expect(state.badges).toHaveLength(0);
      expect(state.actions).toHaveLength(0);
    });
  });

  describe("default", () => {
    it("default meeting → empty badges, empty actions", () => {
      const meeting = makeMeeting();
      const state = computeMeetingDisplayState(meeting, ctx());

      expect(state.primaryStatus).toBe("default");
      expect(state.badges).toHaveLength(0);
      expect(state.actions).toHaveLength(0);
      expect(state.title.lineThrough).toBe(false);
      expect(state.card.hoverEnabled).toBe(true);
    });
  });

  describe("dot styling", () => {
    it("customer type → primary dot color", () => {
      const meeting = makeMeeting({ type: "customer" });
      const state = computeMeetingDisplayState(meeting, ctx());
      expect(state.dot.bgClass).toBe("bg-primary");
    });

    it("personal type → success dot color", () => {
      const meeting = makeMeeting({ type: "personal" });
      const state = computeMeetingDisplayState(meeting, ctx());
      expect(state.dot.bgClass).toBe("bg-success");
    });

    it("internal type → muted fallback dot color", () => {
      const meeting = makeMeeting({ type: "internal" });
      const state = computeMeetingDisplayState(meeting, ctx());
      expect(state.dot.bgClass).toBe("bg-muted-foreground/50");
    });

    it("cancelled → muted dot, no ring", () => {
      const meeting = makeMeeting({ overlayStatus: "cancelled", type: "customer" });
      const state = computeMeetingDisplayState(meeting, ctx());
      expect(state.dot.bgClass).toBe("bg-muted-foreground/30");
      expect(state.dot.ringClass).toBe("");
    });
  });

  describe("showExpander", () => {
    it("true when has inline prep", () => {
      const meeting = makeMeeting();
      const state = computeMeetingDisplayState(meeting, ctx({ hasInlinePrep: true }));
      expect(state.showExpander).toBe(true);
    });

    it("true when outcomes loaded", () => {
      const meeting = makeMeeting();
      const state = computeMeetingDisplayState(meeting, ctx({ isPast: true, outcomesStatus: "loaded" }));
      expect(state.showExpander).toBe(true);
    });

    it("false when no prep and no outcomes", () => {
      const meeting = makeMeeting();
      const state = computeMeetingDisplayState(meeting, ctx());
      expect(state.showExpander).toBe(false);
    });

    it("false when cancelled (even with prep)", () => {
      const meeting = makeMeeting({ overlayStatus: "cancelled" });
      const state = computeMeetingDisplayState(meeting, ctx({ hasInlinePrep: true }));
      expect(state.showExpander).toBe(false);
    });
  });

  describe("priority chain ordering", () => {
    it("past+outcomes takes priority over hasPrepFile", () => {
      const meeting = makeMeeting({ type: "customer", hasPrep: true, prepFile: "prep.md" });
      const state = computeMeetingDisplayState(meeting, ctx({ isPast: true, outcomesStatus: "loaded" }));

      expect(state.primaryStatus).toBe("processed");
      // Should NOT have view-prep action when processed
      expect(state.actions).toHaveLength(0);
    });

    it("cancelled takes priority over everything", () => {
      const meeting = makeMeeting({
        overlayStatus: "cancelled",
        type: "customer",
        hasPrep: true,
        prepFile: "prep.md",
      });
      const state = computeMeetingDisplayState(meeting, ctx({
        isPast: true,
        outcomesStatus: "loaded",
        isLive: true,
        hasInlinePrep: true,
      }));

      expect(state.primaryStatus).toBe("cancelled");
      expect(state.actions).toHaveLength(0);
      expect(state.showExpander).toBe(false);
    });
  });
});
