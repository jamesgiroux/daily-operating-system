import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type { GranolaAuthStatus } from "@/types";
import { withTimeout } from "@/lib/async-utils";
import { useTauriEvent } from "./useTauriEvent";

type GranolaAuthPhase = "idle" | "authorizing" | "disconnecting";

const AUTH_TIMEOUT_MS = 150_000;
const AUTH_TIMEOUT_MESSAGE = "Granola authorization timed out after 120s";

interface GranolaAuthFailedPayload {
  message: string;
}

export function useGranolaAuth() {
  const [status, setStatus] = useState<GranolaAuthStatus>({
    status: "notconfigured",
  });
  const [loading, setLoading] = useState(false);
  const [phase, setPhase] = useState<GranolaAuthPhase>("idle");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<GranolaAuthStatus>("get_granola_oauth_status").then(setStatus).catch((err) => {
      console.error("get_granola_oauth_status failed:", err);
    });
  }, []);

  const handleGranolaAuthChanged = useCallback((payload: GranolaAuthStatus) => {
    setStatus(payload);
  }, []);

  const handleGranolaAuthFailed = useCallback((payload: GranolaAuthFailedPayload) => {
    const message = payload?.message || "Granola auth failed";
    setError(message);
    setLoading(false);
    setPhase("idle");
  }, []);

  useTauriEvent("granola-auth-changed", handleGranolaAuthChanged);
  useTauriEvent("granola-auth-failed", handleGranolaAuthFailed);

  const connect = useCallback(
    async (endpoint: string) => {
      if (loading) return;
      setLoading(true);
      setPhase("authorizing");
      setError(null);
      try {
        const result = await withTimeout(
          invoke<GranolaAuthStatus>("start_granola_oauth", {
            endpoint,
          }),
          AUTH_TIMEOUT_MS,
          AUTH_TIMEOUT_MESSAGE,
        );
        setStatus(result);
        toast.success("Granola account connected");
      } catch (err) {
        const message =
          typeof err === "string"
            ? err
            : err instanceof Error
              ? err.message
              : "Granola auth failed";
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
      await invoke("disconnect_granola_oauth");
      setStatus({ status: "notconfigured" });
      toast.success("Granola account disconnected");
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
