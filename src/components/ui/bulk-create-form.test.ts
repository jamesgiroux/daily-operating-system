import { describe, expect, it } from "vitest";
import {
  parseBulkCreateInput,
  shouldSubmitBulkCreateKey,
} from "./bulk-create-form";

describe("bulk-create-form helpers", () => {
  it("parses and trims non-empty lines", () => {
    expect(
      parseBulkCreateInput(" Acme  \n\n  Globex\n   \nInitech  "),
    ).toEqual(["Acme", "Globex", "Initech"]);
  });

  it("submits on Cmd/Ctrl+Enter only", () => {
    expect(shouldSubmitBulkCreateKey("Enter", true, false)).toBe(true);
    expect(shouldSubmitBulkCreateKey("Enter", false, true)).toBe(true);
    expect(shouldSubmitBulkCreateKey("Enter", false, false)).toBe(false);
    expect(shouldSubmitBulkCreateKey("Escape", true, false)).toBe(false);
  });
});
