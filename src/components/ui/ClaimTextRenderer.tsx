import { invoke } from "@tauri-apps/api/core";
import clsx from "clsx";
import { Eye, LockKeyhole } from "lucide-react";
import { useState } from "react";

import type { RenderableClaimText } from "@/types";
import { Button } from "./button";

export interface ClaimTextRendererProps {
  value?: RenderableClaimText | string | null;
  className?: string;
  surface?: string;
  reveal?: (claimId: string, surface?: string) => Promise<RenderableClaimText>;
}

function isRenderableClaimText(
  value: ClaimTextRendererProps["value"],
): value is RenderableClaimText {
  return Boolean(
    value
      && typeof value === "object"
      && "text" in value
      && "policy" in value,
  );
}

function revealClaim(
  claimId: string,
  surface?: string,
): Promise<RenderableClaimText> {
  return invoke<RenderableClaimText>("reveal_sensitive_claim_text", {
    claimId,
    surface,
  });
}

function affordanceClaimId(value: RenderableClaimText): string | undefined {
  const affordance = value.policy.affordance;
  if (affordance?.kind !== "confidential_click_to_reveal") {
    return value.policy.claimId;
  }
  return affordance.claimId ?? affordance.claim_id ?? value.policy.claimId;
}

export function ClaimTextRenderer({
  value,
  className,
  surface,
  reveal = revealClaim,
}: ClaimTextRendererProps) {
  const [revealed, setRevealed] = useState<RenderableClaimText | null>(null);
  const [isRevealing, setIsRevealing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!value) {
    return null;
  }

  if (!isRenderableClaimText(value)) {
    return <span className={className}>{value}</span>;
  }

  const current = revealed ?? value;
  if (current.policy.kind === "drop") {
    return null;
  }

  if (current.policy.kind === "render") {
    return <span className={className}>{current.text}</span>;
  }

  const affordance = current.policy.affordance;
  const claimId = affordanceClaimId(current);
  const canReveal = affordance?.kind === "confidential_click_to_reveal" && Boolean(claimId);
  const label = affordance?.label ?? current.text;

  async function handleReveal() {
    if (!claimId || isRevealing) {
      return;
    }
    setError(null);
    setIsRevealing(true);
    try {
      setRevealed(await reveal(claimId, surface ?? current.policy.surface));
    } catch {
      setError("Unable to reveal.");
    } finally {
      setIsRevealing(false);
    }
  }

  return (
    <span
      className={clsx(
        "inline-flex max-w-full flex-wrap items-center gap-2 align-baseline",
        className,
      )}
      data-render-policy={current.policy.kind}
      data-sensitivity={current.policy.sensitivity}
    >
      <span className="inline-flex items-center gap-1.5 rounded-md border border-dashed border-stone-300 bg-stone-50 px-2 py-1 text-xs font-medium text-stone-700">
        <LockKeyhole aria-hidden="true" className="size-3.5" />
        {label}
      </span>
      {canReveal ? (
        <Button
          type="button"
          variant="ghost"
          size="xs"
          onClick={handleReveal}
          disabled={isRevealing}
          aria-label="Reveal confidential claim"
        >
          <Eye aria-hidden="true" />
          {isRevealing ? "Revealing" : "Reveal"}
        </Button>
      ) : null}
      {error ? (
        <span role="status" className="text-xs text-red-700">
          {error}
        </span>
      ) : null}
    </span>
  );
}

