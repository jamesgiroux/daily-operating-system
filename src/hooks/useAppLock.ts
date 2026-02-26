import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export function useAppLock() {
  const [isLocked, setIsLocked] = useState(false);

  useEffect(() => {
    // Check initial lock status
    invoke<boolean>("get_lock_status").then(setIsLocked).catch(() => {});

    // Listen for lock/unlock events from the backend
    const unlistenLocked = listen("app-locked", () => setIsLocked(true));
    const unlistenUnlocked = listen("app-unlocked", () => setIsLocked(false));

    return () => {
      unlistenLocked.then((fn) => fn());
      unlistenUnlocked.then((fn) => fn());
    };
  }, []);

  return { isLocked, setIsLocked };
}
