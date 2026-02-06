import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import type { GoogleAuthStatus } from "@/types";

export function useGoogleAuth() {
  const [status, setStatus] = useState<GoogleAuthStatus>({
    status: "notconfigured",
  });
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    invoke<GoogleAuthStatus>("get_google_auth_status").then(setStatus).catch(() => {});

    const unlisten = listen<GoogleAuthStatus>("google-auth-changed", (event) => {
      setStatus(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const connect = useCallback(async () => {
    setLoading(true);
    try {
      const result = await invoke<GoogleAuthStatus>("start_google_auth");
      setStatus(result);
      toast.success("Google account connected");
    } catch (err) {
      const message = typeof err === "string" ? err : "Google auth failed";
      toast.error(message);
    } finally {
      setLoading(false);
    }
  }, []);

  const disconnect = useCallback(async () => {
    setLoading(true);
    try {
      await invoke("disconnect_google");
      setStatus({ status: "notconfigured" });
      toast.success("Google account disconnected");
    } catch (err) {
      const message = typeof err === "string" ? err : "Disconnect failed";
      toast.error(message);
    } finally {
      setLoading(false);
    }
  }, []);

  const email =
    status.status === "authenticated" ? status.email : undefined;

  return { status, email, loading, connect, disconnect };
}
