import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { connections } from "./connections/registry";
import ConnectionDetail from "./ConnectionDetail";
import { styles } from "./styles";

interface ConnectionStatus {
  id: string;
  connected: boolean;
  label: string;
}

function resolveStatus(id: string, result: unknown): { connected: boolean; label: string } {
  if (!result || typeof result !== "object") {
    return { connected: false, label: "Unknown" };
  }

  const r = result as Record<string, unknown>;

  // Google has a nested status field
  if (id === "google") {
    const authStatus = (r as { status?: string }).status;
    if (authStatus === "authenticated") {
      return { connected: true, label: (r as { email?: string }).email ?? "Connected" };
    }
    if (authStatus === "tokenexpired") {
      return { connected: false, label: "Session expired" };
    }
    return { connected: false, label: "Not connected" };
  }

  // Claude Desktop uses success boolean
  if (id === "claude-desktop") {
    return {
      connected: !!r.success,
      label: typeof r.message === "string" ? r.message : r.success ? "Configured" : "Not configured",
    };
  }

  // Standard pattern: enabled + some indicator
  const enabled = !!r.enabled;
  if (!enabled) return { connected: false, label: "Disabled" };

  if (id === "quill") {
    return { connected: !!r.bridgeExists, label: r.bridgeExists ? "Bridge active" : "Bridge not found" };
  }
  if (id === "granola") {
    return { connected: !!r.cacheExists, label: r.cacheExists ? `${r.documentCount} documents` : "Cache not found" };
  }
  if (id === "gravatar") {
    return { connected: true, label: `${r.cachedCount} cached` };
  }
  if (id === "clay") {
    return { connected: true, label: `${r.enrichedCount} enriched` };
  }
  if (id === "linear") {
    return { connected: true, label: `${r.issueCount} issues` };
  }

  return { connected: enabled, label: "Enabled" };
}

export default function ConnectionsGrid() {
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [statuses, setStatuses] = useState<Record<string, ConnectionStatus>>({});

  useEffect(() => {
    for (const conn of connections) {
      invoke(conn.statusCommand)
        .then((result) => {
          const resolved = resolveStatus(conn.id, result);
          setStatuses((prev) => ({
            ...prev,
            [conn.id]: { id: conn.id, ...resolved },
          }));
        })
        .catch(() => {
          setStatuses((prev) => ({
            ...prev,
            [conn.id]: { id: conn.id, connected: false, label: "Error" },
          }));
        });
    }
  }, []);

  function handleToggle(id: string) {
    setExpandedId((prev) => (prev === id ? null : id));
  }

  return (
    <div>
      {connections.map((conn) => {
        const status = statuses[conn.id];
        const isExpanded = expandedId === conn.id;
        const dotColor = !status
          ? "var(--color-text-tertiary)"
          : status.connected
            ? "var(--color-garden-sage)"
            : "var(--color-text-tertiary)";

        return (
          <div key={conn.id}>
            <button
              onClick={() => handleToggle(conn.id)}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 12,
                width: "100%",
                padding: "14px 0",
                background: "none",
                border: "none",
                borderBottom: isExpanded ? "none" : "1px solid var(--color-rule-light)",
                cursor: "pointer",
                textAlign: "left",
              }}
            >
              <div style={styles.statusDot(dotColor)} />
              <span
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 14,
                  fontWeight: 500,
                  color: "var(--color-text-primary)",
                  flex: 1,
                }}
              >
                {conn.name}
              </span>
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  color: "var(--color-text-tertiary)",
                  letterSpacing: "0.02em",
                }}
              >
                {status?.label ?? "Loading..."}
              </span>
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                style={{
                  width: 14,
                  height: 14,
                  color: "var(--color-text-tertiary)",
                  transform: isExpanded ? "rotate(180deg)" : "none",
                  transition: "transform 0.2s ease",
                  flexShrink: 0,
                }}
              >
                <polyline points="6 9 12 15 18 9" />
              </svg>
            </button>

            {isExpanded && (
              <ConnectionDetail
                component={conn.component}
                onClose={() => setExpandedId(null)}
              />
            )}
          </div>
        );
      })}
    </div>
  );
}
