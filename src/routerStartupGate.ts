export type StartupGate =
  | "checking"
  | "encryption-recovery"
  | "database-recovery"
  | "lock"
  | "onboarding"
  | "app";

export interface StartupGateInput {
  checkingConfig: boolean;
  encryptionKeyMissing: boolean;
  dbRecoveryRequired: boolean;
  isLocked: boolean;
  needsOnboarding: boolean;
}

export function resolveStartupGate(input: StartupGateInput): StartupGate {
  if (input.checkingConfig) return "checking";
  if (input.encryptionKeyMissing) return "encryption-recovery";
  if (input.dbRecoveryRequired) return "database-recovery";
  if (input.isLocked) return "lock";
  if (input.needsOnboarding) return "onboarding";
  return "app";
}
