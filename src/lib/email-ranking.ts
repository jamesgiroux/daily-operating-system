import type { Email } from "@/types";

type RankableEmail = Pick<Email, "pinnedAt" | "relevanceScore">;

export function compareEmailRank(a: RankableEmail, b: RankableEmail) {
  const aPinned = !!a.pinnedAt;
  const bPinned = !!b.pinnedAt;
  if (aPinned && !bPinned) return -1;
  if (!aPinned && bPinned) return 1;
  return (b.relevanceScore ?? -1) - (a.relevanceScore ?? -1);
}
