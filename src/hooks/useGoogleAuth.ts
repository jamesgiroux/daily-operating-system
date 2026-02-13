import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import type { GoogleAuthStatus } from "@/types";

type GoogleAuthPhase = "idle" | "authorizing" | "disconnecting";
const AUTH_TIMEOUT_MS = 30_000;

interface GoogleAuthFailedPayload {
  message: string;
}

function withTimeout<T>(promise: Promise<T>, timeoutMs: number): Promise<T> {
  return Promise.race([
    promise,
    new Promise<never>((_, reject) => {
      setTimeout(() => reject(new Error("Google authorization timed out after 30s")), timeoutMs);
    }),
  ]);
}

export function useGoogleAuth() {
  const [status, setStatus] = useState<GoogleAuthStatus>({
    status: "notconfigured",
  });
  const [loading, setLoading] = useState(false);
  const [phase, setPhase] = useState<GoogleAuthPhase>("idle");
  const [error, setError] = useState<string | null>(null);
  const [justConnected, setJustConnected] = useState(false);

  useEffect(() => {
    invoke<GoogleAuthStatus>("get_google_auth_status").then(setStatus).catch(() => {});

    const unlistenChanged = listen<GoogleAuthStatus>("google-auth-changed", (event) => {
      setStatus(event.payload);
    });
    const unlistenFailed = listen<GoogleAuthFailedPayload>("google-auth-failed", (event) => {
      const message = event.payload?.message || "Google auth failed";
      setError(message);
      setLoading(false);
      setPhase("idle");
    });

    return () => {
      unlistenChanged.then((fn) => fn());
      unlistenFailed.then((fn) => fn());
    };
  }, []);

  const connect = useCallback(async () => {
    if (loading) return;
    setLoading(true);
    setPhase("authorizing");
    setError(null);
    try {
      const result = await withTimeout(
        invoke<GoogleAuthStatus>("start_google_auth"),
        AUTH_TIMEOUT_MS,
      );
      setStatus(result);
      setJustConnected(true);
      setTimeout(() => setJustConnected(false), 1500);
      toast.success("Google account connected");
    } catch (err) {
      const message =
        typeof err === "string" ? err : err instanceof Error ? err.message : "Google auth failed";
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
      setPhase("idle");
    }
  }, [loading]);

  const disconnect = useCallback(async () => {
    if (loading) return;
    setLoading(true);
    setPhase("disconnecting");
    setError(null);
    try {
      await invoke("disconnect_google");
      setStatus({ status: "notconfigured" });
      toast.success("Google account disconnected");
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

  const email =
    status.status === "authenticated" ? status.email : undefined;

  const clearError = useCallback(() => setError(null), []);

  return {
    status,
    email,
    loading,
    phase,
    error,
    justConnected,
    connect,
    disconnect,
    clearError,
  };
}
