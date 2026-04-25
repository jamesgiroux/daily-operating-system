import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type { GleanAuthStatus } from "@/types";
import { useTauriEvent } from "./useTauriEvent";

type GleanAuthPhase = "idle" | "authorizing" | "disconnecting";

// Glean SSO (Okta, Google Workspace) can be slower than direct Google OAuth
const AUTH_TIMEOUT_MS = 150_000; // Backend listener has 120s timeout; give extra margin

interface GleanAuthFailedPayload {
  message: string;
}

function withTimeout<T>(promise: Promise<T>, timeoutMs: number): Promise<T> {
  return Promise.race([
    promise,
    new Promise<never>((_, reject) => {
      setTimeout(() => reject(new Error("Glean authorization timed out after 60s")), timeoutMs);
    }),
  ]);
}

export function useGleanAuth() {
  const [status, setStatus] = useState<GleanAuthStatus>({
    status: "notconfigured",
  });
  const [loading, setLoading] = useState(false);
  const [phase, setPhase] = useState<GleanAuthPhase>("idle");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<GleanAuthStatus>("get_glean_auth_status").then(setStatus).catch((err) => {
      console.error("get_glean_auth_status failed:", err); // Expected: background auth check on mount
    });
  }, []);

  const handleGleanAuthChanged = useCallback((payload: GleanAuthStatus) => {
    setStatus(payload);
  }, []);

  const handleGleanAuthFailed = useCallback((payload: GleanAuthFailedPayload) => {
    const message = payload?.message || "Glean auth failed";
    setError(message);
    setLoading(false);
    setPhase("idle");
  }, []);

  useTauriEvent("glean-auth-changed", handleGleanAuthChanged);
  useTauriEvent("glean-auth-failed", handleGleanAuthFailed);

  const connect = useCallback(
    async (endpoint: string) => {
      if (loading) return;
      setLoading(true);
      setPhase("authorizing");
      setError(null);
      try {
        const result = await withTimeout(
          invoke<GleanAuthStatus>("start_glean_auth", {
            endpoint,
          }),
          AUTH_TIMEOUT_MS,
        );
        setStatus(result);
        toast.success("Glean account connected");
      } catch (err) {
        const message =
          typeof err === "string"
            ? err
            : err instanceof Error
              ? err.message
              : "Glean auth failed";
        setError(message);
        toast.error(message);
      } finally {
        setLoading(false);
        setPhase("idle");
      }
    },
    [loading],
  );

  const disconnect = useCallback(async () => {
    if (loading) return;
    setLoading(true);
    setPhase("disconnecting");
    setError(null);
    try {
      await invoke("disconnect_glean");
      setStatus({ status: "notconfigured" });
      toast.success("Glean account disconnected");
    } catch (err) {
      const message =
        typeof err === "string" ? err : err instanceof Error ? err.message : "Disconnect failed";
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
      setPhase("idle");
    }
  }, [loading]);

  const email = status.status === "authenticated" ? status.email : undefined;
  const name = status.status === "authenticated" ? status.name : undefined;

  const clearError = useCallback(() => setError(null), []);

  return {
    status,
    email,
    name,
    loading,
    phase,
    error,
    connect,
    disconnect,
    clearError,
  };
}
