import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import type { GoogleAuthStatus } from "@/types";

const AUTH_TIMEOUT_MS = 30_000;

function errorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  return "Google auth failed";
}

export function useGoogleAuth() {
  const [status, setStatus] = useState<GoogleAuthStatus>({
    status: "notconfigured",
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<GoogleAuthStatus>("get_google_auth_status").then(setStatus).catch(() => {});

    const unlisten = listen<GoogleAuthStatus>("google-auth-changed", (event) => {
      setStatus(event.payload);
    });
    const unlistenFailed = listen<string>("google-auth-failed", (event) => {
      setLoading(false);
      const message = event.payload || "Google auth failed";
      setError(message);
      toast.error(message);
    });

    return () => {
      unlisten.then((fn) => fn());
      unlistenFailed.then((fn) => fn());
    };
  }, []);

  const connect = useCallback(async () => {
    setLoading(true);
    setError(null);
    let timeoutHandle: ReturnType<typeof setTimeout> | undefined;
    try {
      const timeoutPromise = new Promise<never>((_, reject) => {
        timeoutHandle = setTimeout(() => {
          reject(
            new Error(
              "Authorization timed out after 30 seconds. Please try again.",
            ),
          );
        }, AUTH_TIMEOUT_MS);
      });
      const result = await Promise.race([
        invoke<GoogleAuthStatus>("start_google_auth"),
        timeoutPromise,
      ]);
      setStatus(result);
      setError(null);
      toast.success("Google account connected");
    } catch (err) {
      const message = errorMessage(err);
      setError(message);
      toast.error(message);
    } finally {
      if (timeoutHandle) clearTimeout(timeoutHandle);
      setLoading(false);
    }
  }, []);

  const disconnect = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      await invoke("disconnect_google");
      setStatus({ status: "notconfigured" });
      toast.success("Google account disconnected");
    } catch (err) {
      const message = errorMessage(err);
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  }, []);

  const email =
    status.status === "authenticated" ? status.email : undefined;

  return { status, email, loading, error, connect, disconnect };
}
