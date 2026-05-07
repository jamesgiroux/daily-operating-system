import { invoke } from "@tauri-apps/api/core";
import clsx from "clsx";
import { Eye, LockKeyhole } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import type { RenderableClaimText } from "@/types";
import { Button } from "./button";

export interface ClaimTextRendererProps {
  value?: RenderableClaimText | string | null;
  className?: string;
  surface?: string;
  reveal?: (
    claimId: string,
    revealActionId: string,
    surface: string | undefined,
  ) => Promise<RenderableClaimText>;
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
  revealActionId: string,
  surface?: string,
): Promise<RenderableClaimText> {
  return invoke<RenderableClaimText>("reveal_sensitive_claim_text", {
    claimId,
    revealActionId,
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
  const revealInFlightRef = useRef(false);
  const revealActionIdRef = useRef<string | null>(null);
  const carrier = isRenderableClaimText(value) ? value : null;
  const cacheSurface = surface ?? carrier?.policy.surface;
  const prevCarrierRef = useRef<RenderableClaimText | null>(null);
  const prevSurfaceRef = useRef<string | undefined>(undefined);
  const carrierChanged = carrier !== prevCarrierRef.current;
  const surfaceChanged = cacheSurface !== prevSurfaceRef.current;

  useEffect(() => {
    if (carrier === prevCarrierRef.current && cacheSurface === prevSurfaceRef.current) {
      return;
    }
    prevCarrierRef.current = carrier;
    prevSurfaceRef.current = cacheSurface;
    setRevealed(null);
    setError(null);
    setIsRevealing(false);
    revealInFlightRef.current = false;
    revealActionIdRef.current = null;
  }, [carrier, cacheSurface]);

  if (!value) {
    return null;
  }

  if (!isRenderableClaimText(value)) {
    return <span className={className}>{value}</span>;
  }

  const cachedReveal = carrierChanged || surfaceChanged ? null : revealed;
  const current = cachedReveal ?? value;
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
    if (!claimId || revealInFlightRef.current) {
      return;
    }
    const revealCarrier = carrier;
    const revealSurface = surface ?? current.policy.surface;
    const revealActionId = revealActionIdRef.current ?? crypto.randomUUID();
    revealActionIdRef.current = revealActionId;
    revealInFlightRef.current = true;
    setError(null);
    setIsRevealing(true);
    try {
      const rendered = await reveal(
        claimId,
        revealActionId,
        revealSurface,
      );
      if (prevCarrierRef.current === revealCarrier && prevSurfaceRef.current === revealSurface) {
        setRevealed(rendered);
      }
    } catch {
      if (prevCarrierRef.current === revealCarrier && prevSurfaceRef.current === revealSurface) {
        setError("Unable to reveal.");
      }
    } finally {
      if (prevCarrierRef.current === revealCarrier && prevSurfaceRef.current === revealSurface) {
        revealInFlightRef.current = false;
        setIsRevealing(false);
        revealActionIdRef.current = null;
      }
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
