import { useId, useMemo, useState } from "react";
import { ClaimTextRenderer } from "@/components/ui/ClaimTextRenderer";
import { TrustBandIndicator } from "@/components/ui/TrustBandIndicator";
import {
  partitionTrustEvidence,
  readShowAllEvidenceState,
  writeShowAllEvidenceState,
} from "@/lib/trust-band";
import { formatShortDate } from "@/lib/utils";
import type { RenderableClaimText, TrustAnnotated } from "@/types";
import s from "./ContextEntryList.module.css";

type ContextEntryListEntry = TrustAnnotated<{
  id: string;
  title: string | RenderableClaimText;
  content: string | RenderableClaimText;
  createdAt: string;
}>;

interface ContextEntryListProps {
  entries: ContextEntryListEntry[];
  onUpdate: (id: string, title: string, content: string) => void;
  onDelete: (id: string) => void;
  onCreate: (title: string, content: string) => void;
  addLabel?: string;
  placeholders?: { title?: string; content?: string };
  surfaceId?: string;
}

export function ContextEntryList({
  entries,
  onUpdate,
  onDelete,
  onCreate,
  addLabel = "+ Add context entry",
  placeholders,
  surfaceId = "context-entry-list",
}: ContextEntryListProps) {
  const needsVerificationId = useId();
  const [adding, setAdding] = useState(false);
  const [newTitle, setNewTitle] = useState("");
  const [newContent, setNewContent] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editTitle, setEditTitle] = useState("");
  const [editContent, setEditContent] = useState("");
  const [showAllEvidence, setShowAllEvidence] = useState(() =>
    readShowAllEvidenceState(surfaceId),
  );

  const partition = useMemo(
    () => partitionTrustEvidence(entries, { showAllEvidence }),
    [entries, showAllEvidence],
  );

  const newestEvidenceDate = useMemo(
    () => newestEntryEvidenceDate(entries),
    [entries],
  );

  const handleCreate = () => {
    if (!newTitle.trim() || !newContent.trim()) return;
    onCreate(newTitle.trim(), newContent.trim());
    setNewTitle("");
    setNewContent("");
    setAdding(false);
  };

  const startEdit = (entry: ContextEntryListEntry) => {
    setEditingId(entry.id);
    setEditTitle(claimTextToEditableString(entry.title));
    setEditContent(claimTextToEditableString(entry.content));
  };

  const commitEdit = () => {
    if (editingId && editTitle.trim() && editContent.trim()) {
      onUpdate(editingId, editTitle.trim(), editContent.trim());
    }
    setEditingId(null);
  };

  const setShowAllEvidenceForSurface = (next: boolean) => {
    writeShowAllEvidenceState(surfaceId, next);
    setShowAllEvidence(next);
  };

  const renderEntry = (entry: ContextEntryListEntry) =>
    editingId === entry.id ? (
      <div key={entry.id} className={s.editEntryForm}>
        <input
          className={s.addEntryInput}
          value={editTitle}
          onChange={(e) => setEditTitle(e.target.value)}
          placeholder="Title"
          autoFocus
        />
        <textarea
          className={s.addEntryTextarea}
          value={editContent}
          onChange={(e) => setEditContent(e.target.value)}
          placeholder="Content"
          rows={3}
        />
        <div className={s.addEntryActions}>
          <button className={s.addEntryCancel} onClick={() => setEditingId(null)}>
            Cancel
          </button>
          <button className={s.addEntrySave} onClick={commitEdit}>
            Save
          </button>
        </div>
      </div>
    ) : (
      <div key={entry.id} className={s.entryItem}>
        <div className={s.entryHeader}>
          <ClaimTextRenderer
            value={entry.title}
            className={s.entryTitle}
            surface="tauri_entity_detail"
          />
          <span className={s.entryDate}>{formatShortDate(entry.createdAt)}</span>
        </div>
        <div className={s.entryTrust}>
          <TrustBandIndicator band={entry.trustBand ?? "unscored"} />
        </div>
        <div className={s.entryContent}>
          <ClaimTextRenderer value={entry.content} surface="tauri_entity_detail" />
        </div>
        <div className={s.entryActions}>
          <button className={s.entryActionBtn} onClick={() => startEdit(entry)}>
            Edit
          </button>
          <button className={s.entryActionBtn} onClick={() => onDelete(entry.id)}>
            Delete
          </button>
        </div>
      </div>
    );

  return (
    <div>
      <div className={s.entryList}>
        {partition.current.map(renderEntry)}
      </div>

      {partition.current.length === 0 && partition.totalCount > 0 && (
        <p className={s.trustEmptyState}>
          No high-confidence current-state evidence
          {newestEvidenceDate ? ` since ${formatShortDate(newestEvidenceDate)}` : ""}.
        </p>
      )}

      {partition.caution.length > 0 && (
        <details className={s.backgroundDetails}>
          <summary className={s.backgroundSummary}>
            Background
            <span className={s.backgroundCount}>{partition.caution.length}</span>
          </summary>
          <div className={s.backgroundList}>
            {partition.caution.map(renderEntry)}
          </div>
        </details>
      )}

      {partition.needsVerification.length > 0 && (
        <div className={s.showAllEvidence}>
          <button
            type="button"
            className={s.showAllButton}
            aria-pressed={showAllEvidence}
            aria-controls={needsVerificationId}
            onClick={() => setShowAllEvidenceForSurface(!showAllEvidence)}
          >
            {showAllEvidence ? "Hide low-confidence evidence" : "Show all evidence"}
            <span className={s.backgroundCount}>{partition.needsVerification.length}</span>
          </button>
          <span role="status" aria-live="polite" className={s.showAllStatus}>
            {showAllEvidence ? "Showing low-confidence evidence" : "Hiding low-confidence evidence"}
          </span>
          <div id={needsVerificationId} hidden={!showAllEvidence} className={s.backgroundList}>
            {partition.revealedNeedsVerification.map(renderEntry)}
          </div>
        </div>
      )}

      {adding ? (
        <div className={s.addEntryForm}>
          <input
            className={s.addEntryInput}
            value={newTitle}
            onChange={(e) => setNewTitle(e.target.value)}
            placeholder={placeholders?.title ?? "e.g., 'Infrastructure scaling philosophy' or 'Customer success metrics'"}
            autoFocus
          />
          <textarea
            className={s.addEntryTextarea}
            value={newContent}
            onChange={(e) => setNewContent(e.target.value)}
            placeholder={placeholders?.content ?? "Write 1\u20133 paragraphs about your approach, methodology, or key insight."}
            rows={3}
          />
          <div className={s.addEntryActions}>
            <button
              className={s.addEntryCancel}
              onClick={() => {
                setAdding(false);
                setNewTitle("");
                setNewContent("");
              }}
            >
              Cancel
            </button>
            <button className={s.addEntrySave} onClick={handleCreate}>
              Save
            </button>
          </div>
        </div>
      ) : (
        <button className={s.addBtn} onClick={() => setAdding(true)}>
          {addLabel}
        </button>
      )}
    </div>
  );
}

function newestEntryEvidenceDate(entries: ContextEntryListEntry[]): string | null {
  const newest = entries
    .map((entry) => entry.trustSourceDate ?? entry.createdAt)
    .map((date) => ({ raw: date, time: Date.parse(date) }))
    .filter((date) => Number.isFinite(date.time))
    .sort((a, b) => b.time - a.time)[0];
  return newest?.raw ?? null;
}

function claimTextToEditableString(value: string | RenderableClaimText): string {
  return typeof value === "string" ? value : value.text;
}
