import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTauriEvent } from "./useTauriEvent";

export function useAppLock() {
  const [isLocked, setIsLocked] = useState(false);

  useEffect(() => {
    // Check initial lock status
    invoke<boolean>("get_lock_status").then(setIsLocked).catch(() => {});
  }, []);

  const handleLocked = useCallback(() => setIsLocked(true), []);
  const handleUnlocked = useCallback(() => setIsLocked(false), []);

  // Listen for lock/unlock events from the backend
  useTauriEvent("app-locked", handleLocked);
  useTauriEvent("app-unlocked", handleUnlocked);

  return { isLocked, setIsLocked };
}
