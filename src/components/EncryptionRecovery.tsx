import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

/**
 * Recovery screen shown when the encryption key is missing from the
 * macOS Keychain but an encrypted database exists.
 *
 * Renders INSTEAD of the app — no data is accessible without the key.
 */
export function EncryptionRecovery() {
  return (
    <div className="encryption-recovery">
      <div className="encryption-recovery-content">
        <div className="encryption-recovery-icon">*</div>
        <h1>Encryption Key Not Found</h1>
        <p>
          DailyOS found an encrypted database but the decryption key is missing
          from your macOS Keychain. This can happen if the Keychain entry was
          deleted or if the app was moved to a different machine.
        </p>
        <p>
          Your data is still on disk but cannot be read without the original key.
        </p>
        <div className="encryption-recovery-options">
          <h3>Options</h3>
          <ul>
            <li>
              <strong>Restore from Keychain backup</strong> — If you have a Time Machine
              or iCloud Keychain backup, restore the Keychain entry for
              &ldquo;com.dailyos.desktop.db&rdquo; and relaunch the app.
            </li>
            <li>
              <strong>Start fresh</strong> — Delete <code>~/.dailyos/dailyos.db</code> and
              relaunch. A new database and key will be created. Your workspace
              files (accounts, projects) are preserved and will be re-imported.
            </li>
          </ul>
        </div>
      </div>
      <style>{`
        .encryption-recovery {
          position: fixed;
          inset: 0;
          background: var(--color-paper-cream, #f5f0e8);
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 1100;
          padding: 32px;
        }
        .encryption-recovery-content {
          max-width: 520px;
          text-align: left;
        }
        .encryption-recovery-icon {
          font-family: var(--font-mark, 'Newsreader', serif);
          font-size: 48px;
          color: var(--color-spice-terracotta, #c97d60);
          margin-bottom: 8px;
        }
        .encryption-recovery h1 {
          font-family: var(--font-serif, 'Newsreader', serif);
          font-size: 24px;
          font-weight: 500;
          margin: 0 0 16px;
        }
        .encryption-recovery p {
          font-family: var(--font-sans, 'DM Sans', sans-serif);
          font-size: 14px;
          line-height: 1.6;
          color: var(--color-ink-secondary, #555);
          margin: 0 0 12px;
        }
        .encryption-recovery h3 {
          font-family: var(--font-sans, 'DM Sans', sans-serif);
          font-size: 14px;
          font-weight: 600;
          margin: 24px 0 8px;
        }
        .encryption-recovery ul {
          padding-left: 20px;
          font-family: var(--font-sans, 'DM Sans', sans-serif);
          font-size: 13px;
          line-height: 1.6;
          color: var(--color-ink-secondary, #555);
        }
        .encryption-recovery li {
          margin-bottom: 12px;
        }
        .encryption-recovery code {
          font-family: var(--font-mono, 'JetBrains Mono', monospace);
          font-size: 12px;
          background: var(--color-surface-secondary, #ede8df);
          padding: 2px 6px;
          border-radius: 3px;
        }
      `}</style>
    </div>
  );
}

/**
 * Hook to check if the encryption key is missing.
 */
export function useEncryptionStatus() {
  const [keyMissing, setKeyMissing] = useState(false);

  useEffect(() => {
    invoke<boolean>('get_encryption_key_status').then(setKeyMissing).catch(() => {});
  }, []);

  return keyMissing;
}
