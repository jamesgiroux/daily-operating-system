export type WorkCommitmentOwnerPatch = {
  ownerRaw?: string;
  clearOwner?: boolean;
};

export function workCommitmentOwnerPatch(owner: string): WorkCommitmentOwnerPatch {
  const trimmed = owner.trim();
  if (trimmed.length === 0) {
    return { clearOwner: true };
  }
  return { ownerRaw: trimmed };
}
