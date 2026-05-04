import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { toast } from "sonner";
import { Download, Trash2, AlertTriangle, Loader2 } from "lucide-react";
import {
  SettingsButton,
  SettingsInput,
  SettingsRule,
  SettingsSectionLabel,
  formRowStyles,
} from "@/components/settings/FormRow";
import type { DataSummary, ExportReport, ClearReport } from "@/types";

// ---------------------------------------------------------------------------
// DataPrivacySection
// ---------------------------------------------------------------------------

export default function DataPrivacySection() {
  const [summary, setSummary] = useState<DataSummary | null>(null);
  const [exporting, setExporting] = useState(false);
  const [clearing, setClearing] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [confirmText, setConfirmText] = useState("");
  const [showClearConfirm, setShowClearConfirm] = useState(false);

  useEffect(() => {
    invoke<DataSummary>("get_data_summary")
      .then(setSummary)
      .catch((e) => console.error("Failed to load data summary:", e)); // Expected: background data fetch on mount
  }, []);

  // ── Export ──────────────────────────────────────────────────────────────

  async function handleExport() {
    const dest = await save({
      defaultPath: `dailyos-export-${new Date().toISOString().slice(0, 10)}.zip`,
      filters: [{ name: "ZIP", extensions: ["zip"] }],
    });
    if (!dest) return;

    setExporting(true);
    try {
      const report = await invoke<ExportReport>("export_all_data", {
        destPath: dest,
      });
      const total =
        report.counts.accounts +
        report.counts.people +
        report.counts.projects +
        report.counts.meetings +
        report.counts.actions +
        report.counts.signals +
        report.counts.intelligence;
      toast.success(`Exported ${total} records to ${report.path}`);
    } catch (e) {
      toast.error(`Export failed: ${e}`);
    } finally {
      setExporting(false);
    }
  }

  // ── Clear Insights ─────────────────────────────────────────────────────

  async function handleClearIntelligence() {
    setClearing(true);
    try {
      const report = await invoke<ClearReport>("clear_intelligence");
      const total =
        report.assessmentsDeleted +
        report.feedbackDeleted +
        report.signalsDeleted +
        report.summariesCleared;
      toast.success(`Cleared ${total} insight records`);
      setShowClearConfirm(false);
      // Refresh counts
      const fresh = await invoke<DataSummary>("get_data_summary");
      setSummary(fresh);
    } catch (e) {
      toast.error(`Failed to clear insights: ${e}`);
    } finally {
      setClearing(false);
    }
  }

  // ── Delete Everything ──────────────────────────────────────────────────

  async function handleDeleteAll() {
    setDeleting(true);
    try {
      await invoke("delete_all_data");
      toast.success("All data deleted. Restarting...");
      setTimeout(() => relaunch(), 1500);
    } catch (e) {
      toast.error(`Delete failed: ${e}`);
      setDeleting(false);
    }
  }

  // ── Render ─────────────────────────────────────────────────────────────

  const countLine = (label: string, count: number | undefined) => (
    <span
      style={{
        fontFamily: "var(--font-mono)",
        fontSize: 12,
        color: "var(--color-text-secondary)",
      }}
    >
      {count ?? 0} {label}
    </span>
  );

  return (
    <div>
      {/* ── Section: What DailyOS stores ───────────────────────────── */}
      <SettingsSectionLabel>Your data</SettingsSectionLabel>

      <p className={formRowStyles.descriptionLead}>
        Everything is stored locally on your Mac. Nothing leaves your device
        unless you explicitly connect an external service.
      </p>
      <div style={{ marginBottom: 24 }}>
        <p className={formRowStyles.descriptionSmallBottom8}>
          Stored locally: calendar events, email metadata and summaries, contacts,
          meeting transcripts, and AI-generated intelligence.
        </p>
        <p className={formRowStyles.descriptionFlush}>
          Not stored permanently: full email bodies, Glean result payloads after disconnect,
          or connector credentials outside the macOS Keychain.
        </p>
      </div>

      {summary && (
        <div
          style={{
            display: "flex",
            flexWrap: "wrap",
            gap: "8px 24px",
            marginBottom: 24,
          }}
        >
          {countLine("accounts", summary.accounts)}
          {countLine("contacts", summary.people)}
          {countLine("projects", summary.projects)}
          {countLine("meetings", summary.meetings)}
          {countLine("actions", summary.actions)}
          {countLine("insights", summary.insights)}
          {countLine("emails", summary.emails)}
        </div>
      )}

      <SettingsRule />

      {/* ── Section: Export ─────────────────────────────────────────── */}
      <div style={{ padding: "16px 0" }}>
        <p className={formRowStyles.descriptionTight}>
          Download a ZIP containing all your data as human-readable JSON files.
        </p>
        <SettingsButton
          tone="primary"
          onClick={handleExport}
          disabled={exporting}
          muted={exporting}
        >
          {exporting ? (
            <Loader2 size={12} className="animate-spin" />
          ) : (
            <Download size={12} />
          )}
          {exporting ? "Exporting..." : "Export all data"}
        </SettingsButton>
      </div>

      <SettingsRule />

      {/* ── Section: Clear Insights ────────────────────────────────── */}
      <div style={{ padding: "16px 0" }}>
        <p className={formRowStyles.descriptionTight}>
          Remove all AI-generated analysis while keeping your accounts,
          contacts, and meetings intact. Insights will be regenerated
          automatically over time.
        </p>

        {!showClearConfirm ? (
          <SettingsButton
            tone="danger"
            onClick={() => setShowClearConfirm(true)}
          >
            <Trash2 size={12} />
            Clear insights
          </SettingsButton>
        ) : (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 12,
            }}
          >
            <span
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 13,
                color: "var(--color-spice-terracotta)",
              }}
            >
              Are you sure?
            </span>
            <SettingsButton
              tone="danger"
              onClick={handleClearIntelligence}
              disabled={clearing}
              muted={clearing}
            >
              {clearing ? (
                <Loader2 size={12} className="animate-spin" />
              ) : (
                <Trash2 size={12} />
              )}
              {clearing ? "Clearing..." : "Yes, clear insights"}
            </SettingsButton>
            <SettingsButton
              tone="ghost"
              onClick={() => setShowClearConfirm(false)}
            >
              Cancel
            </SettingsButton>
          </div>
        )}
      </div>

      <SettingsRule />

      {/* ── Section: Delete Everything ─────────────────────────────── */}
      <div
        style={{
          padding: 16,
          marginTop: 8,
          border: "1px solid var(--color-spice-terracotta)",
          borderRadius: 6,
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            marginBottom: 8,
          }}
        >
          <AlertTriangle
            size={14}
            style={{ color: "var(--color-spice-terracotta)" }}
          />
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              letterSpacing: "0.06em",
              textTransform: "uppercase" as const,
              color: "var(--color-spice-terracotta)",
            }}
          >
            Danger zone
          </span>
        </div>
        <p className={formRowStyles.descriptionTight}>
          Permanently delete all data including your database, workspace files,
          and configuration. This cannot be undone. DailyOS will restart with a
          clean slate.
        </p>

        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <SettingsInput
            width={200}
            className={formRowStyles.inputDanger}
            type="text"
            placeholder='Type "DELETE" to confirm'
            value={confirmText}
            onChange={(e) => setConfirmText(e.target.value)}
          />
          <SettingsButton
            tone="danger"
            onClick={handleDeleteAll}
            disabled={confirmText !== "DELETE" || deleting}
          >
            {deleting ? (
              <Loader2 size={12} className="animate-spin" />
            ) : (
              <Trash2 size={12} />
            )}
            {deleting ? "Deleting..." : "Delete everything"}
          </SettingsButton>
        </div>
      </div>
    </div>
  );
}
