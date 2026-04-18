import { describe, expect, it } from "vitest";
import { buildWorkChapters } from "./account-detail-utils";

/**
 * DOS-13 Work tab: chapter pills MUST match the section ids rendered by
 * AccountDetailPage.renderWorkView, or the folio nav island dead-links.
 *
 * This snapshot is the authoritative contract for those ids.
 */
const EXPECTED_WORK_SECTION_IDS = [
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
  it("returns 8 chapters in section order", () => {
    const chapters = buildWorkChapters();
    expect(chapters).toHaveLength(EXPECTED_WORK_SECTION_IDS.length);
    expect(chapters.map((c) => c.id)).toEqual(EXPECTED_WORK_SECTION_IDS);
  });

  it("every chapter has a non-empty label and icon element", () => {
    for (const chapter of buildWorkChapters()) {
      expect(chapter.label).toBeTruthy();
      expect(typeof chapter.label).toBe("string");
      expect(chapter.icon).toBeTruthy();
    }
  });

  it("chapter ids are unique", () => {
    const ids = buildWorkChapters().map((c) => c.id);
    expect(new Set(ids).size).toBe(ids.length);
  });
});
