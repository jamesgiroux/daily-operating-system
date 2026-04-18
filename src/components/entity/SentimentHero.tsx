/**
 * SentimentHero — DOS-27 journal-entry sentiment treatment for Health view.
 *
 * Renders the current sentiment value, a 90-day computed-health sparkline,
 * the latest journal note, a "Still accurate?" prompt after 30 days of
 * inactivity, and a divergence flag when user sentiment and computed health
 * disagree.
 */
import { useState } from "react";
import type { SentimentValue } from "@/types";
import type { SentimentView } from "@/hooks/useAccountDetail";
import { DEFAULT_SENTIMENT_LABELS } from "@/hooks/useAccountDetail";
import css from "./SentimentHero.module.css";

interface SentimentHeroProps {
  view: SentimentView;
  onSetSentiment: (value: SentimentValue, note?: string) => Promise<void>;
  onAcknowledgeStale: () => Promise<void>;
  /** Optional override for preset-specific labels. */
  labels?: Partial<Record<SentimentValue, string>>;
}

const SENTIMENT_ORDER: SentimentValue[] = [
  "strong",
  "on_track",
  "concerning",
  "at_risk",
  "critical",
];

function valueClass(value: SentimentValue): string {
  switch (value) {
    case "strong": return css.valueStrong;
    case "on_track": return css.valueOnTrack;
    case "concerning": return css.valueConcerning;
    case "at_risk": return css.valueAtRisk;
    case "critical": return css.valueCritical;
  }
}

function relativeDays(iso: string | null): string {
  if (!iso) return "";
  const days = Math.floor((Date.now() - new Date(iso).getTime()) / (24 * 60 * 60 * 1000));
  if (days <= 0) return "today";
  if (days === 1) return "1 day ago";
  return `${days} days ago`;
}

function formatNoteDate(iso: string): string {
  try {
    return new Date(iso).toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
    });
  } catch {
    return iso;
  }
}

function sparkBarClass(band: string): string {
  switch (band.toLowerCase()) {
    case "green": return css.sparkGood;
    case "yellow": return css.sparkMid;
    case "red": return css.sparkWarn;
    default: return css.sparkMid;
  }
}

function sparkHeight(score: number): number {
  // 100 → 20px, 0 → 5px
  const clamped = Math.max(0, Math.min(100, score));
  return Math.round(5 + (clamped / 100) * 15);
}

export function SentimentHero({
  view,
  onSetSentiment,
  onAcknowledgeStale,
  labels: labelOverrides,
}: SentimentHeroProps) {
  const labels: Record<SentimentValue, string> = {
    ...DEFAULT_SENTIMENT_LABELS,
    ...view.presetLabels,
    ...(labelOverrides ?? {}),
  };
  const [editing, setEditing] = useState(false);
  const [draftValue, setDraftValue] = useState<SentimentValue | null>(view.current);
  const [draftNote, setDraftNote] = useState("");
  const [saving, setSaving] = useState(false);

  function openEditor() {
    setDraftValue(view.current);
    setDraftNote("");
    setEditing(true);
  }

  function cancelEditor() {
    setEditing(false);
    setDraftNote("");
  }

  async function saveEditor() {
    if (!draftValue) return;
    setSaving(true);
    try {
      await onSetSentiment(draftValue, draftNote);
      setEditing(false);
      setDraftNote("");
    } finally {
      setSaving(false);
    }
  }

  // Unset state — invite the user to set
  if (!view.current) {
    return (
      <section className={css.hero}>
        <div className={css.label}>Your Assessment</div>
        {editing ? (
          <SentimentEditor
            labels={labels}
            draftValue={draftValue}
            draftNote={draftNote}
            onDraftValueChange={setDraftValue}
            onDraftNoteChange={setDraftNote}
            onCancel={cancelEditor}
            onSave={saveEditor}
            saving={saving}
          />
        ) : (
          <div className={css.unsetRow}>
            Is this relationship{" "}
            <button
              type="button"
              className={css.unsetButton}
              onClick={openEditor}
            >
              strong, on track, concerning, at risk, or critical?
            </button>
          </div>
        )}
      </section>
    );
  }

  // Sparkline uses up to the last 24 daily points so it fits the hero line.
  const sparkPoints = view.sparkline.slice(-24);

  return (
    <section className={css.hero}>
      <div className={css.label}>Your Assessment</div>

      <div className={css.setRow}>
        <span className={`${css.value} ${valueClass(view.current)}`}>
          {labels[view.current]}
        </span>

        {sparkPoints.length > 0 && (
          <>
            <div
              className={css.sparkline}
              aria-label="Computed health over last 90 days"
            >
              {sparkPoints.map((p, i) => (
                <div
                  key={i}
                  className={`${css.sparkBar} ${sparkBarClass(p.band)}`}
                  style={{ height: `${sparkHeight(p.score)}px` }}
                />
              ))}
            </div>
            <span className={css.sparklineLabel}>90d</span>
          </>
        )}

        <button
          type="button"
          className={css.updateButton}
          onClick={openEditor}
        >
          Update
        </button>
      </div>

      {view.setAt && (
        <div className={css.meta}>
          Set {relativeDays(view.setAt)}
          {view.history.length > 0 && (
            <> · {view.history.length} note{view.history.length === 1 ? "" : "s"}</>
          )}
          {view.isStale && (
            <>
              {" · "}
              <button
                type="button"
                className={css.metaLink}
                onClick={onAcknowledgeStale}
                title="Tap to confirm this still reflects the account"
              >
                Still accurate?
              </button>
            </>
          )}
        </div>
      )}

      {view.note && !editing && (
        <blockquote className={css.note}>
          “{view.note}”
          {view.history[0]?.setAt && (
            <div className={css.noteDate}>
              — Your note, {formatNoteDate(view.history[0].setAt)}
            </div>
          )}
        </blockquote>
      )}

      {view.divergence && !editing && (
        <button
          type="button"
          className={css.divergenceFlag}
          onClick={() => {
            // Click surfaces detail; feedback is captured on the next sentiment
            // update (Bayesian signal is the act of adding more detail).
            openEditor();
          }}
        >
          <strong>Updates currently disagree</strong>
          Computed health is{" "}
          <span style={{ textTransform: "capitalize" }}>
            {view.divergence.computedBand}
          </span>{" "}
          and your read is {labels[view.current].toLowerCase()} — a{" "}
          {view.divergence.severity}{" "}
          divergence ({view.divergence.delta} band{view.divergence.delta === 1 ? "" : "s"}{" "}
          apart). The note you add next is the signal that trains the system.
        </button>
      )}

      {editing && (
        <SentimentEditor
          labels={labels}
          draftValue={draftValue}
          draftNote={draftNote}
          onDraftValueChange={setDraftValue}
          onDraftNoteChange={setDraftNote}
          onCancel={cancelEditor}
          onSave={saveEditor}
          saving={saving}
        />
      )}
    </section>
  );
}

interface SentimentEditorProps {
  labels: Record<SentimentValue, string>;
  draftValue: SentimentValue | null;
  draftNote: string;
  onDraftValueChange: (v: SentimentValue) => void;
  onDraftNoteChange: (note: string) => void;
  onCancel: () => void;
  onSave: () => void;
  saving: boolean;
}

function SentimentEditor({
  labels,
  draftValue,
  draftNote,
  onDraftValueChange,
  onDraftNoteChange,
  onCancel,
  onSave,
  saving,
}: SentimentEditorProps) {
  return (
    <div className={css.editor}>
      <div className={css.editorOptions}>
        {SENTIMENT_ORDER.map((v) => (
          <button
            key={v}
            type="button"
            className={`${css.editorOption} ${
              draftValue === v ? css.editorOptionActive : ""
            }`}
            onClick={() => onDraftValueChange(v)}
          >
            {labels[v]}
          </button>
        ))}
      </div>
      <textarea
        className={css.editorTextarea}
        placeholder="Add a journal note — what's driving this read?"
        value={draftNote}
        onChange={(e) => onDraftNoteChange(e.target.value)}
      />
      <div className={css.editorActions}>
        <button type="button" className={css.editorBtn} onClick={onCancel}>
          Cancel
        </button>
        <button
          type="button"
          className={`${css.editorBtn} ${css.editorBtnPrimary}`}
          onClick={onSave}
          disabled={!draftValue || saving}
        >
          {saving ? "Saving…" : "Save"}
        </button>
      </div>
    </div>
  );
}
