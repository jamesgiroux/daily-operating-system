import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import {
  SettingsButton,
  SettingsSectionLabel,
  formRowStyles,
} from "@/components/settings/FormRow";
import s from "./ActivityLogSection.module.css";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface AuditRecord {
  ts: string;
  v: number;
  category: string;
  event: string;
  detail: Record<string, unknown>;
  prev_hash: string | null;
}

type CategoryFilter =
  | "all"
  | "security"
  | "data_access"
  | "ai"
  | "anomaly"
  | "config"
  | "system";

const CATEGORY_FILTERS: { value: CategoryFilter; label: string }[] = [
  { value: "all", label: "All" },
  { value: "security", label: "Security" },
  { value: "data_access", label: "Data" },
  { value: "ai", label: "AI" },
  { value: "anomaly", label: "Anomalies" },
  { value: "config", label: "Config" },
  { value: "system", label: "System" },
];

// ---------------------------------------------------------------------------
// Event name translation (raw snake_case → plain English)
// ---------------------------------------------------------------------------

type EventTranslation = string | ((detail: Record<string, unknown>) => string);

const EVENT_NAMES: Record<string, EventTranslation> = {
  app_started: "App started",
  audit_log_rotated: "Log maintenance",
  db_key_accessed: "Database opened",
  db_key_generated: "Encryption key created",
  db_key_missing: "Database key missing",
  db_migration_started: "Database migration started",
  db_migration_completed: "Database migration completed",
  oauth_connected: "Google account connected",
  oauth_revoked: "Google account disconnected",
  app_unlock_succeeded: "App unlocked",
  app_unlock_failed: "Unlock attempt failed",
  google_calendar_sync: (d) =>
    `Calendar synced (${d.events_fetched ?? 0} events)`,
  gmail_sync: (d) => `Email synced (${d.emails_fetched ?? 0} emails)`,
  clay_enrichment: "Contact updated (Clay)",
  gravatar_lookup: "Avatar lookup (Gravatar)",
  entity_enrichment_completed: "Context updated",
  entity_enrichment_failed: "Context update failed",
  email_enrichment_batch: "Email batch processed",
  meeting_prep_generated: "Meeting briefing generated",
  injection_tag_escape_detected: "Injection attempt detected and blocked",
  injection_instruction_in_output: "Suspicious output detected",
  schema_validation_failed: "AI output dismissed (unexpected format)",
  workspace_path_changed: "Workspace path changed",
  ai_provider_changed: "AI provider changed",
  context_mode_changed: (d) =>
    `Context source changed (${d.from ?? "?"} → ${d.to ?? "?"})`,
  glean_context_gathered: "Glean context gathered",
  glean_connection_failed: "Glean connection failed",
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatTimestamp(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleTimeString(undefined, {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  } catch {
    return iso;
  }
}

function formatDate(iso: string): string {
  try {
    const d = new Date(iso);
    const today = new Date();
    const yesterday = new Date();
    yesterday.setDate(yesterday.getDate() - 1);

    if (d.toDateString() === today.toDateString()) return "Today";
    if (d.toDateString() === yesterday.toDateString()) return "Yesterday";

    return d.toLocaleDateString(undefined, {
      weekday: "long",
      month: "long",
      day: "numeric",
    });
  } catch {
    return iso;
  }
}

function getCategoryClass(category: string): string {
  switch (category) {
    case "security":
      return s.categorySecurity;
    case "data_access":
      return s.categoryDataAccess;
    case "ai":
      return s.categoryAi;
    case "anomaly":
      return s.categoryAnomaly;
    case "config":
      return s.categoryConfig;
    default:
      return s.categorySystem;
  }
}

function groupByDay(
  records: AuditRecord[]
): { date: string; records: AuditRecord[] }[] {
  const groups: Map<string, AuditRecord[]> = new Map();
  // Records come in chronological order — reverse for most-recent-first
  for (const record of [...records].reverse()) {
    const dateKey = record.ts.slice(0, 10);
    const group = groups.get(dateKey) ?? [];
    group.push(record);
    groups.set(dateKey, group);
  }
  return Array.from(groups.entries()).map(([date, recs]) => ({
    date: recs[0]?.ts ?? date,
    records: recs,
  }));
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const PAGE_SIZE = 50;

export default function ActivityLogSection() {
  const [records, setRecords] = useState<AuditRecord[]>([]);
  const [filter, setFilter] = useState<CategoryFilter>("all");
  const [expandedIdx, setExpandedIdx] = useState<string | null>(null);
  const [visibleCount, setVisibleCount] = useState(PAGE_SIZE);
  const [integrityResult, setIntegrityResult] = useState<{
    ok: boolean;
    message: string;
  } | null>(null);

  const loadRecords = useCallback(async () => {
    try {
      const data = await invoke<AuditRecord[]>("get_audit_log_records", {
        limit: PAGE_SIZE,
        categoryFilter: filter === "all" ? null : filter,
      });
      setRecords(data);
      setVisibleCount(PAGE_SIZE);
    } catch (e) {
      console.warn("Failed to load audit records:", e);
    }
  }, [filter]);

  useEffect(() => {
    loadRecords();
  }, [loadRecords]);

  const handleExport = async () => {
    const path = await save({
      defaultPath: "audit-log.jsonl",
      filters: [{ name: "JSON Lines", extensions: ["jsonl"] }],
    });
    if (!path) return;

    try {
      await invoke("export_audit_log", { destPath: path });
      toast.success("Audit log exported");
    } catch (e) {
      toast.error(`Export failed: ${e}`);
    }
  };

  const handleVerify = async () => {
    try {
      const result = await invoke<string>("verify_audit_log_integrity");
      setIntegrityResult({ ok: true, message: result });
    } catch (e) {
      setIntegrityResult({ ok: false, message: String(e) });
    }
  };

  // Paginate: records arrive chronological; take the most recent N from the tail
  const totalCount = records.length;
  const visibleRecords = records.slice(Math.max(0, totalCount - visibleCount));
  const grouped = groupByDay(visibleRecords);
  const hasMore = visibleCount < totalCount;

  return (
    <div className={s.container}>
      <SettingsSectionLabel as="h3">Activity Log</SettingsSectionLabel>
      <p className={formRowStyles.descriptionLead}>
        A tamper-evident record of what the app did and when.
      </p>

      {/* Category filter */}
      <div className={s.filterBar}>
        {CATEGORY_FILTERS.map((f) => (
          <button
            key={f.value}
            className={`${s.filterChip} ${filter === f.value ? s.filterChipActive : ""}`}
            onClick={() => {
              setFilter(f.value);
              setExpandedIdx(null);
            }}
          >
            {f.label}
          </button>
        ))}
      </div>

      {/* Records grouped by day */}
      {grouped.length === 0 ? (
        <div className={s.emptyState}>No activity recorded yet.</div>
      ) : (
        grouped.map((group) => (
          <div key={group.date} className={s.dayGroup}>
            <h4 className={s.dayLabel}>{formatDate(group.date)}</h4>
            {group.records.map((record, idx) => {
              const key = `${record.ts}-${idx}`;
              const isExpanded = expandedIdx === key;
              const isAnomaly = record.category === "anomaly";

              return (
                <div key={key}>
                  <div
                    className={`${s.record} ${isAnomaly ? s.recordAnomaly : ""}`}
                    onClick={() => setExpandedIdx(isExpanded ? null : key)}
                  >
                    <span className={s.timestamp}>
                      {formatTimestamp(record.ts)}
                    </span>
                    <span
                      className={`${s.categoryChip} ${getCategoryClass(record.category)}`}
                    >
                      {record.category.replace("_", " ")}
                    </span>
                    <span className={s.eventName}>
                      {(() => {
                        const t = EVENT_NAMES[record.event];
                        if (typeof t === "function") return t(record.detail);
                        return t ?? record.event;
                      })()}
                    </span>
                  </div>
                  {isExpanded && (
                    <div className={s.detail}>
                      {JSON.stringify(record.detail, null, 2)}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        ))
      )}

      {/* Pagination */}
      {totalCount > 0 && (
        <div className={s.pagination}>
          <span className={s.paginationCount}>
            Showing {Math.min(visibleCount, totalCount)} of {totalCount} entries
          </span>
          {hasMore && (
            <button
              className={s.loadMoreButton}
              onClick={() => setVisibleCount((prev) => prev + PAGE_SIZE)}
            >
              Load more
            </button>
          )}
        </div>
      )}

      {/* Actions */}
      <div className={s.actions}>
        <SettingsButton tone="ghost" onClick={handleExport}>
          Export Log
        </SettingsButton>
        <SettingsButton tone="primary" onClick={handleVerify}>
          Verify Integrity
        </SettingsButton>
      </div>

      {integrityResult && (
        <div
          className={`${s.integrityResult} ${
            integrityResult.ok ? s.integritySuccess : s.integrityFailure
          }`}
        >
          {integrityResult.message}
        </div>
      )}
    </div>
  );
}
