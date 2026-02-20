import { Loader2 } from "lucide-react";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";
import { styles } from "../styles";

export default function GoogleConnection() {
  const {
    status,
    email,
    loading,
    phase,
    error,
    justConnected,
    connect,
    disconnect,
    clearError,
  } = useGoogleAuth();

  return (
    <div>
      <p style={styles.subsectionLabel}>Google Account</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        {status.status === "authenticated"
          ? "Calendar and meeting features active"
          : "Connect Google for calendar awareness and meeting features"}
      </p>

      {error && (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "10px 0",
            borderBottom: "1px solid var(--color-spice-terracotta)",
            marginBottom: 12,
          }}
        >
          <span
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 12,
              color: "var(--color-spice-terracotta)",
            }}
          >
            {error}
          </span>
          <button
            style={{
              ...styles.btn,
              fontSize: 10,
              padding: "2px 8px",
              color: "var(--color-spice-terracotta)",
              border: "none",
            }}
            onClick={clearError}
          >
            Dismiss
          </button>
        </div>
      )}

      {status.status === "authenticated" ? (
        <div style={styles.settingRow}>
          <div>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <div style={styles.statusDot("var(--color-garden-sage)")} />
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
                {email}
              </span>
            </div>
            {justConnected && (
              <p
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 12,
                  color: "var(--color-garden-sage)",
                  marginTop: 4,
                }}
              >
                Connected successfully.
              </p>
            )}
          </div>
          <button
            style={{ ...styles.btn, ...styles.btnGhost, opacity: loading || phase === "authorizing" ? 0.5 : 1 }}
            onClick={disconnect}
            disabled={loading || phase === "authorizing"}
          >
            {loading ? (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                <Loader2 size={12} className="animate-spin" /> ...
              </span>
            ) : (
              "Disconnect"
            )}
          </button>
        </div>
      ) : status.status === "tokenexpired" ? (
        <div style={styles.settingRow}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <div style={styles.statusDot("var(--color-spice-terracotta)")} />
            <span style={styles.description}>Session expired</span>
          </div>
          <button
            style={{ ...styles.btn, ...styles.btnDanger, opacity: loading ? 0.5 : 1 }}
            onClick={connect}
            disabled={loading}
          >
            {loading ? (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                <Loader2 size={12} className="animate-spin" /> ...
              </span>
            ) : phase === "authorizing" ? (
              "Waiting..."
            ) : (
              "Reconnect"
            )}
          </button>
        </div>
      ) : (
        <div style={styles.settingRow}>
          <span style={styles.description}>Not connected</span>
          <button
            style={{ ...styles.btn, ...styles.btnPrimary, opacity: loading ? 0.5 : 1 }}
            onClick={connect}
            disabled={loading}
          >
            {loading ? (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                <Loader2 size={12} className="animate-spin" /> ...
              </span>
            ) : phase === "authorizing" ? (
              "Waiting for authorization..."
            ) : (
              "Connect"
            )}
          </button>
        </div>
      )}
    </div>
  );
}
