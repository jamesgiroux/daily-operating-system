import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./LockOverlay.css";

interface LockOverlayProps {
  onUnlock: () => void;
}

export function LockOverlay({ onUnlock }: LockOverlayProps) {
  const [error, setError] = useState<string | null>(null);
  const [authenticating, setAuthenticating] = useState(false);

  const handleUnlock = async () => {
    setAuthenticating(true);
    setError(null);
    try {
      await invoke("unlock_app");
      onUnlock();
    } catch (e) {
      setError(String(e));
    } finally {
      setAuthenticating(false);
    }
  };

  return (
    <div className="lock-overlay">
      <div className="lock-content">
        <div className="lock-icon">*</div>
        <h1 className="lock-title">DailyOS</h1>
        <p className="lock-subtitle">Locked</p>
        <button
          className="lock-unlock-btn"
          onClick={handleUnlock}
          disabled={authenticating}
        >
          {authenticating ? "Authenticating..." : "Unlock with Touch ID"}
        </button>
        {error && <p className="lock-error">{error}</p>}
      </div>
    </div>
  );
}
