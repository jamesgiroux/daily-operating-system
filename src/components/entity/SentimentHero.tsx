/**
 * SentimentHero — DOS-27 journal-entry sentiment treatment for Health view.
 *
 * Renders the current sentiment value, a 90-day computed-health sparkline
 * (bucketed to 7 bars to match the editorial mockup), the latest journal
 * pull quote, a "Still accurate?" prompt after 30 days of inactivity, and a
 * divergence flag when user sentiment and computed health disagree.
 *
 * Canonical design: .docs/mockups/account-health-outlook-globex.html
 * lines 599-635.
 */
import { useState } from "react";
import type { SentimentValue, HealthSparklinePoint } from "@/types";
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

/** Number of bars in the sparkline — fixed by the editorial mockup. */
const SPARKLINE_BUCKETS = 7;

function valueClass(value: SentimentValue): string {
  switch (value) {
    case "strong":
      return css.valueStrong;
    case "on_track":
      return css.valueOnTrack;
    case "concerning":
      return css.valueConcerning;
    case "at_risk":
      return css.valueAtRisk;
    case "critical":
      return css.valueCritical;
  }
}

function relativeDays(iso: string | null): string {
  if (!iso) return "";
  const days = Math.floor(
    (Date.now() - new Date(iso).getTime()) / (24 * 60 * 60 * 1000),
  );
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

/**
 * Maps a sentiment value to the band used for tinting the sparkline bar that
 * sits beside the sentiment pill. The raw band from a computed sparkline
 * point wins when present; this is the fallback tier when we only have the
 * sentiment value.
 */
function sentimentBand(value: SentimentValue): "good" | "mid" | "bad" {
  switch (value) {
    case "strong":
    case "on_track":
      return "good";
    case "concerning":
      return "mid";
    case "at_risk":
    case "critical":
      return "bad";
  }
}

function sparkBarClass(band: string): string {
  switch (band.toLowerCase()) {
    case "green":
    case "good":
      return css.sparkGood;
    case "yellow":
    case "mid":
      return css.sparkMid;
    case "red":
    case "bad":
    case "warn":
      return css.sparkWarn;
    default:
      return css.sparkMid;
  }
}

interface SparkBar {
  score: number;
  band: string;
}

/**
 * Bucket up to ~90 days of points into exactly 7 bars (matching the mockup).
 * Each bucket averages the underlying scores and picks the worst band in the
 * window, so a single bad day still colors the bar. Returns an empty array
 * when no data is available.
 */
function bucketSparkline(points: HealthSparklinePoint[]): SparkBar[] {
  if (!points.length) return [];
  const window = points.slice(-90);
  const bucketSize = window.length / SPARKLINE_BUCKETS;
  const bars: SparkBar[] = [];
  for (let i = 0; i < SPARKLINE_BUCKETS; i++) {
    const start = Math.floor(i * bucketSize);
    const end = Math.max(start + 1, Math.floor((i + 1) * bucketSize));
    const slice = window.slice(start, end);
    if (!slice.length) continue;
    const avg =
      slice.reduce((sum, p) => sum + (p.score ?? 0), 0) / slice.length;
    const band = worstBand(slice.map((p) => p.band));
    bars.push({ score: avg, band });
  }
  return bars;
}

/** Pick the worst (red > yellow > green) band in a window. */
function worstBand(bands: string[]): string {
  const order: Record<string, number> = {
    red: 3,
    bad: 3,
    warn: 3,
    yellow: 2,
    mid: 2,
    green: 1,
    good: 1,
  };
  let worst = "green";
  let worstRank = 0;
  for (const b of bands) {
    const rank = order[b.toLowerCase()] ?? 0;
    if (rank > worstRank) {
      worstRank = rank;
      worst = b;
    }
  }
  return worst;
}

/**
 * Map a 0-100 score to one of the mockup's discrete heights. The mockup uses
 * h4/h8/h12/h16/h20, and we mirror that quantization so the bars feel
 * deliberate rather than jittery.
 */
function sparkHeightClass(score: number): string {
  const clamped = Math.max(0, Math.min(100, score));
  if (clamped >= 80) return css.sparkBarH20;
  if (clamped >= 60) return css.sparkBarH16;
  if (clamped >= 40) return css.sparkBarH12;
  if (clamped >= 20) return css.sparkBarH8;
  return css.sparkBarH4;
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
  const [draftValue, setDraftValue] = useState<SentimentValue | null>(
    view.current,
  );
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

  const sparkBars = bucketSparkline(view.sparkline);
  const noteCount = view.history.length;
  const latestEntry = view.history[0];
  const latestNoteDate = latestEntry?.setAt ?? view.setAt ?? null;
  const currentLabel = labels[view.current];
  const currentBandFallback = sentimentBand(view.current);

  return (
    <section className={css.hero}>
      <div className={css.label}>Your Assessment</div>

      <div className={css.setRow}>
        <span className={`${css.value} ${valueClass(view.current)}`}>
          {currentLabel}
        </span>

        {sparkBars.length > 0 ? (
          <>
            <div
              className={css.sparkline}
              aria-label="Computed health over last 90 days"
            >
              {sparkBars.map((bar, i) => (
                <div
                  key={i}
                  className={`${css.sparkBar} ${sparkBarClass(bar.band || currentBandFallback)} ${sparkHeightClass(bar.score)}`}
                />
              ))}
            </div>
            <span className={css.sparklineLabel}>90d</span>
          </>
        ) : (
          // TODO(DOS-27): Empty-state sparkline placeholder when no computed
          // history is available yet — reserves the layout slot without
          // fabricating data.
          <span className={css.sparklineEmpty} aria-hidden="true" />
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
          {noteCount > 0 && (
            <>
              {" · "}
              {noteCount} note{noteCount === 1 ? "" : "s"}
            </>
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
        <blockquote className={css.pullquote}>
          &ldquo;{view.note}&rdquo;
          {latestNoteDate && (
            <span className={css.pullquoteAttr}>
              &mdash; Your note, {formatNoteDate(latestNoteDate)}
            </span>
          )}
        </blockquote>
      )}

      {view.divergence && !editing && (
        <div className={css.divergenceFlag}>
          <strong>Updates currently disagree</strong>
          Computed health is{" "}
          <span className={css.divergenceComputed}>
            {capitalize(view.divergence.computedBand)}
          </span>{" "}
          and your read is {currentLabel.toLowerCase()} &mdash; a{" "}
          {view.divergence.severity} divergence ({view.divergence.delta} band
          {view.divergence.delta === 1 ? "" : "s"} apart). The note you add
          next is the signal that trains the system.{" "}
          <button
            type="button"
            className={css.divergenceAction}
            onClick={openEditor}
          >
            Add more detail &rarr;
          </button>
        </div>
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

function capitalize(s: string): string {
  if (!s) return s;
  return s.charAt(0).toUpperCase() + s.slice(1);
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
