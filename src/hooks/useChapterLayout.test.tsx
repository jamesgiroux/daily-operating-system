/** @vitest-environment jsdom */

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  applyOverlayToComposition,
  useChapterLayout,
} from "@/hooks/useChapterLayout";
import type { ProjectedBlock, ProjectedComposition, ProjectedSection } from "@/services/composition/contracts";
import type { CompositionLayoutOverlay, LayoutOverlayResponse } from "@/services/composition/layoutOverlay";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((innerResolve, innerReject) => {
    resolve = innerResolve;
    reject = innerReject;
  });
  return { promise, resolve, reject };
}

function section(sectionId: string, blockIndexes: number[], label = sectionId): ProjectedSection {
  return {
    section_id: sectionId,
    section_index: blockIndexes[0] ?? 0,
    label,
    layout: "stacked",
    salience: { weight: 0.5, band: "contextual", reason: "Fixture salience" },
    block_ids: blockIndexes.map((index) => `block-${index}`),
    block_indexes: blockIndexes,
  };
}

function block(blockId: string, title: string, typeId = "claim_summary"): ProjectedBlock {
  return {
    block_id: blockId,
    block_index: 0,
    original_type_id: typeId,
    selected_known_type_id: typeId,
    payload: { title, text: `${title} body` },
    banner: null,
    trust_band: "likely_current",
    claim_refs: [],
    provenance: [],
    edit_routes: [],
    diagnostics: [],
  };
}

function projection(blocks: ProjectedBlock[], sections: ProjectedSection[]): ProjectedComposition {
  return {
    composition_id: "dailyos/account-overview:account:fixture",
    composition_version: 4,
    fallback_policy_version: 3,
    sections,
    blocks,
    diagnostics: [],
    unknown_block_count: 0,
    unknown_block_cap: 5,
    dropped_unknown_block_count: 0,
  };
}

function overlay(overrides: Partial<CompositionLayoutOverlay> = {}): CompositionLayoutOverlay {
  return {
    schemaVersion: 1,
    sectionOrder: [],
    blockOrder: {},
    hiddenSectionIds: [],
    hiddenBlockIds: [],
    blockVariants: {},
    sectionLabelOverrides: {},
    ...overrides,
  };
}

function overlayResponse(
  responseOverlay: CompositionLayoutOverlay | null,
  layoutRevision = 1,
): LayoutOverlayResponse {
  return {
    entityType: "account",
    surfaceKey: "entity_page",
    overlaySchemaVersion: 1,
    layoutRevision,
    overlay: responseOverlay,
    updatedAt: "2026-06-02T10:00:00Z",
  };
}

function accountProjection(suffix = "a"): ProjectedComposition {
  const blocks = [
    block(`masthead-${suffix}`, "Account overview", "account_overview"),
    block(`signal-${suffix}`, "Signal"),
    block(`risk-${suffix}`, "Risk"),
  ];
  blocks.forEach((item, index) => {
    item.block_index = index;
  });
  return projection(blocks, [
    section("headline", [0], "Lead"),
    section("outlook", [1, 2], "Outlook"),
  ]);
}

describe("applyOverlayToComposition", () => {
  it("keeps core headline content visible and in producer order despite overlay preferences", () => {
    const blocks = [
      block("core-a", "Core A", "account_overview"),
      block("core-b", "Core B", "claim_summary"),
      block("signal", "Signal"),
    ];
    blocks.forEach((item, index) => {
      item.block_index = index;
    });
    const view = applyOverlayToComposition(
      projection(blocks, [
        section("headline", [0, 1], "Lead"),
        section("outlook", [2], "Outlook"),
      ]),
      overlay({
        blockOrder: { headline: ["core-b", "core-a"] },
        hiddenSectionIds: ["headline"],
        hiddenBlockIds: ["core-a"],
      }),
    );

    expect(view.sections[0].section.section_id).toBe("headline");
    expect(view.sections[0].coreLocked).toBe(true);
    expect(view.sections[0].blocks.map((item) => item.block.block_id)).toEqual(["core-a", "core-b"]);
    expect(view.hiddenItems).toEqual([]);
  });

  it("applies non-core order, visibility, variants, and presentation labels while ignoring stale ids", () => {
    const blocks = [
      block("masthead", "Account overview", "account_overview"),
      block("signal", "Signal"),
      block("risk", "Risk"),
      block("watch", "Watch"),
    ];
    blocks.forEach((item, index) => {
      item.block_index = index;
    });
    const view = applyOverlayToComposition(
      projection(blocks, [
        section("headline", [0], "Lead"),
        section("outlook", [1, 2], "Outlook"),
        section("watch-list", [3], "Watch list"),
      ]),
      overlay({
        sectionOrder: ["missing-section", "watch-list", "outlook"],
        blockOrder: { outlook: ["missing-block", "risk", "signal"] },
        hiddenBlockIds: ["risk", "missing-block"],
        blockVariants: { signal: "spotlight", risk: "compact", "missing-block": "compact" },
        sectionLabelOverrides: { outlook: "Field notes" },
      }),
    );

    expect(view.sections.map((item) => item.section.section_id)).toEqual([
      "headline",
      "watch-list",
      "outlook",
    ]);
    const outlook = view.sections.find((item) => item.section.section_id === "outlook");
    expect(outlook?.label).toBe("Field notes");
    expect(outlook?.blocks.map((item) => item.block.block_id)).toEqual(["signal"]);
    expect(outlook?.blocks[0].variant).toBe("spotlight");
    expect(outlook?.hiddenBlocks.map((item) => item.block.block_id)).toEqual(["risk"]);
    expect(view.hiddenItems).toEqual([
      { id: "risk", label: "Risk", kind: "block", sectionId: "outlook" },
    ]);
  });

  it("reports the empty non-core state when user visibility leaves only core content visible", () => {
    const view = applyOverlayToComposition(
      accountProjection(),
      overlay({ hiddenBlockIds: ["signal-a", "risk-a"] }),
    );

    expect(view.sections.map((item) => item.section.section_id)).toEqual(["headline", "outlook"]);
    expect(view.hasVisibleNonCore).toBe(false);
    expect(view.hiddenItems.map((item) => item.id)).toEqual(["signal-a", "risk-a"]);
  });
});

describe("useChapterLayout", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("loads the per-type overlay and applies it to the open projection", async () => {
    invokeMock.mockResolvedValueOnce(
      overlayResponse(overlay({ hiddenBlockIds: ["risk-a"], blockVariants: { "signal-a": "compact" } }), 8),
    );

    const { result } = renderHook(() =>
      useChapterLayout({ projection: accountProjection(), entityType: "account" }),
    );

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(invokeMock).toHaveBeenCalledWith("get_composition_layout_overlay", {
      key: { entityType: "account", surfaceKey: "entity_page" },
    });
    expect(result.current.layoutRevision).toBe(8);
    expect(result.current.view.hiddenItems.map((item) => item.id)).toEqual(["risk-a"]);
    expect(result.current.view.sections[1].blocks[0].variant).toBe("compact");
  });

  it("optimistically saves block visibility and accepts the saved revision", async () => {
    const save = deferred<LayoutOverlayResponse>();
    invokeMock
      .mockResolvedValueOnce(overlayResponse(null, 0))
      .mockReturnValueOnce(save.promise);

    const { result } = renderHook(() =>
      useChapterLayout({ projection: accountProjection(), entityType: "account" }),
    );
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => {
      result.current.setBlockHidden("risk-a", true);
    });

    expect(result.current.saving).toBe(true);
    expect(result.current.overlay.hiddenBlockIds).toEqual(["risk-a"]);
    expect(result.current.view.hiddenItems.map((item) => item.id)).toEqual(["risk-a"]);

    await act(async () => {
      save.resolve(overlayResponse(overlay({ hiddenBlockIds: ["risk-a"] }), 2));
      await save.promise;
    });

    await waitFor(() => expect(result.current.saving).toBe(false));
    expect(result.current.layoutRevision).toBe(2);
    expect(result.current.overlay.hiddenBlockIds).toEqual(["risk-a"]);
  });

  it("rolls back a failed latest save", async () => {
    const save = deferred<LayoutOverlayResponse>();
    invokeMock
      .mockResolvedValueOnce(overlayResponse(overlay({ hiddenBlockIds: ["signal-a"] }), 1))
      .mockReturnValueOnce(save.promise);

    const { result } = renderHook(() =>
      useChapterLayout({ projection: accountProjection(), entityType: "account" }),
    );
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => {
      result.current.setBlockHidden("risk-a", true);
    });
    expect(result.current.overlay.hiddenBlockIds).toEqual(["signal-a", "risk-a"]);

    await act(async () => {
      save.reject(new Error("layout save failed"));
      await save.promise.catch(() => undefined);
    });

    await waitFor(() => expect(result.current.saving).toBe(false));
    expect(result.current.overlay.hiddenBlockIds).toEqual(["signal-a"]);
    expect(result.current.error).toBe("layout save failed");
  });

  it("does not let a stale initial overlay load overwrite a local edit", async () => {
    const load = deferred<LayoutOverlayResponse>();
    const save = deferred<LayoutOverlayResponse>();
    invokeMock.mockReturnValueOnce(load.promise).mockReturnValueOnce(save.promise);

    const { result } = renderHook(() =>
      useChapterLayout({ projection: accountProjection(), entityType: "account" }),
    );

    expect(result.current.loading).toBe(true);
    act(() => {
      result.current.setBlockHidden("risk-a", true);
    });
    expect(result.current.overlay.hiddenBlockIds).toEqual(["risk-a"]);

    await act(async () => {
      load.resolve(overlayResponse(overlay({ hiddenBlockIds: ["signal-a"] }), 1));
      await load.promise;
    });

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.layoutRevision).toBe(0);
    expect(result.current.overlay.hiddenBlockIds).toEqual(["risk-a"]);

    await act(async () => {
      save.resolve(overlayResponse(overlay({ hiddenBlockIds: ["risk-a"] }), 2));
      await save.promise;
    });

    await waitFor(() => expect(result.current.saving).toBe(false));
    expect(result.current.layoutRevision).toBe(2);
    expect(result.current.overlay.hiddenBlockIds).toEqual(["risk-a"]);
  });

  it("does not let a failed older save rollback a newer mutation", async () => {
    const firstSave = deferred<LayoutOverlayResponse>();
    const secondSave = deferred<LayoutOverlayResponse>();
    invokeMock
      .mockResolvedValueOnce(overlayResponse(null, 0))
      .mockReturnValueOnce(firstSave.promise)
      .mockReturnValueOnce(secondSave.promise);

    const { result } = renderHook(() =>
      useChapterLayout({ projection: accountProjection(), entityType: "account" }),
    );
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => {
      result.current.setBlockHidden("risk-a", true);
      result.current.setBlockVariant("signal-a", "spotlight");
    });

    expect(result.current.overlay.hiddenBlockIds).toEqual(["risk-a"]);
    expect(result.current.overlay.blockVariants["signal-a"]).toBe("spotlight");

    await act(async () => {
      secondSave.resolve(
        overlayResponse(
          overlay({ hiddenBlockIds: ["risk-a"], blockVariants: { "signal-a": "spotlight" } }),
          3,
        ),
      );
      await secondSave.promise;
    });
    await waitFor(() => expect(result.current.saving).toBe(false));

    await act(async () => {
      firstSave.reject(new Error("older save failed"));
      await firstSave.promise.catch(() => undefined);
    });

    expect(result.current.error).toBeNull();
    expect(result.current.layoutRevision).toBe(3);
    expect(result.current.overlay.hiddenBlockIds).toEqual(["risk-a"]);
    expect(result.current.overlay.blockVariants["signal-a"]).toBe("spotlight");
  });

  it("rolls back overlapping failed saves to the last persisted overlay", async () => {
    const firstSave = deferred<LayoutOverlayResponse>();
    const secondSave = deferred<LayoutOverlayResponse>();
    invokeMock
      .mockResolvedValueOnce(overlayResponse(null, 0))
      .mockReturnValueOnce(firstSave.promise)
      .mockReturnValueOnce(secondSave.promise);

    const { result } = renderHook(() =>
      useChapterLayout({ projection: accountProjection(), entityType: "account" }),
    );
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => {
      result.current.setBlockHidden("risk-a", true);
      result.current.setBlockVariant("signal-a", "spotlight");
    });

    expect(result.current.overlay.hiddenBlockIds).toEqual(["risk-a"]);
    expect(result.current.overlay.blockVariants["signal-a"]).toBe("spotlight");

    await act(async () => {
      firstSave.reject(new Error("older save failed"));
      await firstSave.promise.catch(() => undefined);
    });

    expect(result.current.overlay.hiddenBlockIds).toEqual(["risk-a"]);
    expect(result.current.overlay.blockVariants["signal-a"]).toBe("spotlight");

    await act(async () => {
      secondSave.reject(new Error("latest save failed"));
      await secondSave.promise.catch(() => undefined);
    });

    await waitFor(() => expect(result.current.saving).toBe(false));
    expect(result.current.overlay.hiddenBlockIds).toEqual([]);
    expect(result.current.overlay.blockVariants["signal-a"]).toBeUndefined();
    expect(result.current.error).toBe("latest save failed");
  });

  it("does not let an older reset response overwrite a newer save", async () => {
    const reset = deferred<LayoutOverlayResponse>();
    const save = deferred<LayoutOverlayResponse>();
    invokeMock
      .mockResolvedValueOnce(overlayResponse(overlay({ hiddenBlockIds: ["risk-a"] }), 1))
      .mockReturnValueOnce(reset.promise)
      .mockReturnValueOnce(save.promise);

    const { result } = renderHook(() =>
      useChapterLayout({ projection: accountProjection(), entityType: "account" }),
    );
    await waitFor(() => expect(result.current.loading).toBe(false));

    void act(() => {
      void result.current.resetLayout();
    });
    expect(result.current.overlay.hiddenBlockIds).toEqual([]);

    act(() => {
      result.current.setBlockVariant("signal-a", "spotlight");
    });

    await act(async () => {
      save.resolve(overlayResponse(overlay({ blockVariants: { "signal-a": "spotlight" } }), 4));
      await save.promise;
    });
    await waitFor(() => expect(result.current.saving).toBe(false));

    await act(async () => {
      reset.resolve(overlayResponse(null, 2));
      await reset.promise;
    });

    expect(result.current.layoutRevision).toBe(4);
    expect(result.current.overlay.blockVariants["signal-a"]).toBe("spotlight");
  });

  it("keeps same-type Account layout while route projections change and stale ids disappear from the view", async () => {
    const save = deferred<LayoutOverlayResponse>();
    invokeMock
      .mockResolvedValueOnce(overlayResponse(null, 0))
      .mockReturnValueOnce(save.promise);

    const { result, rerender } = renderHook(
      ({ currentProjection }: { currentProjection: ProjectedComposition }) =>
        useChapterLayout({ projection: currentProjection, entityType: "account" }),
      { initialProps: { currentProjection: accountProjection("a") } },
    );
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => {
      result.current.setBlockHidden("risk-a", true);
    });
    rerender({ currentProjection: accountProjection("b") });

    expect(invokeMock).toHaveBeenCalledTimes(2);
    expect(result.current.view.hiddenItems).toEqual([]);
    expect(result.current.view.sections[1].blocks.map((item) => item.block.block_id)).toEqual([
      "signal-b",
      "risk-b",
    ]);

    await act(async () => {
      save.resolve(overlayResponse(overlay({ hiddenBlockIds: ["risk-a"] }), 2));
      await save.promise;
    });

    expect(result.current.view.hiddenItems).toEqual([]);
    expect(result.current.view.sections[1].blocks.map((item) => item.block.block_id)).toEqual([
      "signal-b",
      "risk-b",
    ]);
  });
});
