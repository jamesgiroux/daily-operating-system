import { describe, expect, it } from "vitest";

import { resolveStartupGate } from "./routerStartupGate";

describe("resolveStartupGate", () => {
  it("prioritizes checking config before all other states", () => {
    expect(
      resolveStartupGate({
        checkingConfig: true,
        dbRecoveryRequired: true,
        isLocked: true,
        needsOnboarding: true,
      })
    ).toBe("checking");
  });

  it("blocks app with database recovery when required", () => {
    expect(
      resolveStartupGate({
        checkingConfig: false,
        dbRecoveryRequired: true,
        isLocked: false,
        needsOnboarding: false,
      })
    ).toBe("database-recovery");
  });

  it("prefers lock over onboarding when both are true", () => {
    expect(
      resolveStartupGate({
        checkingConfig: false,
        dbRecoveryRequired: false,
        isLocked: true,
        needsOnboarding: true,
      })
    ).toBe("lock");
  });

  it("returns app only when no startup gate is active", () => {
    expect(
      resolveStartupGate({
        checkingConfig: false,
        dbRecoveryRequired: false,
        isLocked: false,
        needsOnboarding: false,
      })
    ).toBe("app");
  });
});
