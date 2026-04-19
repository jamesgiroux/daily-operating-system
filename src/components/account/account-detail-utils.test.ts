import { describe, expect, it } from "vitest";
import { buildWorkChapters } from "./account-detail-utils";

/**
 * DOS-13 Work tab: chapter pills MUST match the section ids rendered by
 * AccountDetailPage.renderWorkView, or the folio nav island dead-links.
 *
 * This snapshot is the authoritative contract for those ids.
 *
 * Wave 0g Finding 2: "shared" is conditional on hasSharedData (real
 * tracker provenance, arrives in v1.2.2 / DOS-75).
 */
const EXPECTED_WORK_SECTION_IDS_NO_SHARED = [
  "focus",
  "programs",
  "commitments",
  "suggestions",
  "recently-landed",
  "outputs",
  "nudges",
];

const EXPECTED_WORK_SECTION_IDS_WITH_SHARED = [
  "focus",
  "programs",
  "commitments",
  "suggestions",
  "shared",
  "recently-landed",
  "outputs",
  "nudges",
];

describe("buildWorkChapters", () => {
  it("omits the 'shared' pill by default (honest degradation)", () => {
    const chapters = buildWorkChapters();
    expect(chapters.map((c) => c.id)).toEqual(EXPECTED_WORK_SECTION_IDS_NO_SHARED);
    expect(chapters.some((c) => c.id === "shared")).toBe(false);
  });

  it("omits the 'shared' pill when hasSharedData is false", () => {
    const chapters = buildWorkChapters(false);
    expect(chapters.map((c) => c.id)).toEqual(EXPECTED_WORK_SECTION_IDS_NO_SHARED);
  });

  it("includes the 'shared' pill when hasSharedData is true", () => {
    const chapters = buildWorkChapters(true);
    expect(chapters.map((c) => c.id)).toEqual(EXPECTED_WORK_SECTION_IDS_WITH_SHARED);
  });

  it("every chapter has a non-empty label and icon element", () => {
    for (const chapter of buildWorkChapters(true)) {
      expect(chapter.label).toBeTruthy();
      expect(typeof chapter.label).toBe("string");
      expect(chapter.icon).toBeTruthy();
    }
  });

  it("chapter ids are unique in both modes", () => {
    for (const hasShared of [false, true]) {
      const ids = buildWorkChapters(hasShared).map((c) => c.id);
      expect(new Set(ids).size).toBe(ids.length);
    }
  });
});
