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
// Phase 2a chapter order: Commitments is the opener, Programs sits
// between Suggestions and the optional Shared pill, The Record is the
// terminal timeline. Nudges + Focus chapters were removed.
const EXPECTED_WORK_SECTION_IDS_NO_SHARED = [
  "commitments",
  "suggestions",
  "programs",
  "recently-landed",
  "outputs",
  "the-record",
];

const EXPECTED_WORK_SECTION_IDS_WITH_SHARED = [
  "commitments",
  "suggestions",
  "programs",
  "shared",
  "recently-landed",
  "outputs",
  "the-record",
];

const EXPECTED_WORK_SECTION_IDS_WITH_FILES = [
  "commitments",
  "suggestions",
  "programs",
  "shared",
  "recently-landed",
  "outputs",
  "the-record",
  "files",
];

const ALL_CONTENT_FLAGS = {
  hasCommitments: true,
  hasSuggestions: true,
  hasPrograms: true,
  hasRecentlyLanded: true,
  hasOutputs: true,
};

describe("buildWorkChapters", () => {
  it("omitting all flags returns only the always-on chapter", () => {
    const chapters = buildWorkChapters();
    expect(chapters.map((c) => c.id)).toEqual(["the-record"]);
  });

  it("omits the 'shared' pill when hasSharedData is false", () => {
    const chapters = buildWorkChapters({
      ...ALL_CONTENT_FLAGS,
      hasSharedData: false,
    });
    expect(chapters.map((c) => c.id)).toEqual(EXPECTED_WORK_SECTION_IDS_NO_SHARED);
  });

  it("includes the 'shared' pill when hasSharedData is true", () => {
    const chapters = buildWorkChapters({
      ...ALL_CONTENT_FLAGS,
      hasSharedData: true,
    });
    expect(chapters.map((c) => c.id)).toEqual(EXPECTED_WORK_SECTION_IDS_WITH_SHARED);
  });

  it("includes the 'files' pill after the record when hasFiles is true", () => {
    const chapters = buildWorkChapters({
      ...ALL_CONTENT_FLAGS,
      hasSharedData: true,
      hasFiles: true,
    });
    expect(chapters.map((c) => c.id)).toEqual(EXPECTED_WORK_SECTION_IDS_WITH_FILES);
  });

  it("every chapter has a non-empty label and icon element", () => {
    for (const chapter of buildWorkChapters({
      ...ALL_CONTENT_FLAGS,
      hasSharedData: true,
      hasFiles: true,
    })) {
      expect(chapter.label).toBeTruthy();
      expect(typeof chapter.label).toBe("string");
      expect(chapter.icon).toBeTruthy();
    }
  });

  it("chapter ids are unique in both modes", () => {
    for (const hasShared of [false, true]) {
      const ids = buildWorkChapters({
        ...ALL_CONTENT_FLAGS,
        hasSharedData: hasShared,
      }).map((c) => c.id);
      expect(new Set(ids).size).toBe(ids.length);
    }
  });
});
