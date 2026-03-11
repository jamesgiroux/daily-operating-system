import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export function ICloudWarningModal() {
  const [warningPath, setWarningPath] = useState<string | null>(null);

  useEffect(() => {
    invoke<string | null>("check_icloud_warning").then((path) => {
      if (path) setWarningPath(path);
    });
  }, []);

  if (!warningPath) return null;

  const handleDismiss = async () => {
    await invoke("dismiss_icloud_warning");
    setWarningPath(null);
  };

  return (
    <div className="icloud-warning-overlay">
      <div className="icloud-warning-modal">
        <h2>Workspace in iCloud</h2>
        <p>Your workspace is in an iCloud-synced folder:</p>
        <code className="icloud-warning-path">{warningPath}</code>
        <p>
          iCloud sync can cause data corruption with local databases. Consider
          moving your workspace to a folder outside of iCloud sync, such as{" "}
          <code>~/.dailyos/</code>.
        </p>
        <div className="icloud-warning-actions">
          <button onClick={handleDismiss} className="icloud-warning-dismiss">
            I Understand — Don't Show Again
          </button>
        </div>
      </div>
      <style>{`
        .icloud-warning-overlay {
          position: fixed;
          inset: 0;
          background: var(--color-overlay-medium);
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 900;
        }
        .icloud-warning-modal {
          background: var(--color-paper-warm-white, #faf8f6);
          border-radius: 12px;
          padding: 32px;
          max-width: 480px;
          margin: 16px;
          box-shadow: var(--shadow-2xl);
        }
        .icloud-warning-modal h2 {
          font-family: var(--font-serif, 'Newsreader', Georgia, serif);
          color: var(--color-text-primary, #1e2530);
          margin: 0 0 16px;
          font-size: 1.5rem;
          font-weight: 500;
        }
        .icloud-warning-modal p {
          color: var(--color-text-secondary, #5a6370);
          font-family: var(--font-sans, 'DM Sans', sans-serif);
          font-size: 0.9375rem;
          line-height: 1.6;
          margin: 0 0 12px;
        }
        .icloud-warning-modal p code {
          background: var(--color-paper-linen, #e8e2d9);
          padding: 2px 6px;
          border-radius: 4px;
          font-family: var(--font-mono, 'JetBrains Mono', monospace);
          font-size: 0.8125rem;
        }
        .icloud-warning-path {
          display: block;
          padding: 8px 12px;
          background: var(--color-paper-linen, #e8e2d9);
          border-radius: 6px;
          font-family: var(--font-mono, 'JetBrains Mono', monospace);
          font-size: 0.8125rem;
          color: var(--color-text-primary, #1e2530);
          margin: 8px 0 16px;
          word-break: break-all;
        }
        .icloud-warning-actions {
          display: flex;
          justify-content: flex-end;
          margin-top: 24px;
        }
        .icloud-warning-dismiss {
          padding: 8px 20px;
          border-radius: 6px;
          border: none;
          background: var(--color-spice-terracotta, #c4654a);
          color: white;
          cursor: pointer;
          font-family: var(--font-sans, 'DM Sans', sans-serif);
          font-size: 0.875rem;
          font-weight: 500;
          transition: opacity 0.15s ease;
        }
        .icloud-warning-dismiss:hover {
          opacity: 0.9;
        }
      `}</style>
    </div>
  );
}
