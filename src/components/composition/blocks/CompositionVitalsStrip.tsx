/**
 * CompositionVitalsStrip — the curated dot-separated vitals strip (reusing
 * VitalsStrip's editorial CSS) for the claim-backed composition runtime.
 *
 * Display-formatted, edit-raw: each cell shows the producer's `display_value`
 * ($185,400 / Nov 24, 2026) but seeds its inline editor with the raw `value`
 * so the user edits 185400, not "$185,400". Commits route through
 * `onSave(field, rawValue)` — the snapshot-field correction path
 * (update_account_field), never the bespoke account hook. A vital is editable
 * only when a column mapping is known and a save handler is supplied;
 * otherwise it renders read-only.
 */
import { useState } from "react";
import css from "@/components/entity/VitalsStrip.module.css";
import editCss from "./CompositionVitalsStrip.module.css";

export interface CompositionVitalSpec {
  /** Human label, e.g. "ARR" (used for the accessible name + edit affordance). */
  label: string | null;
  /** Formatted value for display, e.g. "$185,400". */
  display: string;
  /** Raw value for editing, e.g. "185400". */
  raw: string;
  /** Account column to correct, e.g. "arr". Null = not editable. */
  field: string | null;
}

interface CompositionVitalsStripProps {
  vitals: CompositionVitalSpec[];
  onSave?: (field: string, value: string) => Promise<void> | void;
}

export function CompositionVitalsStrip({ vitals, onSave }: CompositionVitalsStripProps) {
  if (vitals.length === 0) return null;
  return (
    <div className={css.strip}>
      <div className={css.items}>
        {vitals.map((vital, index) => (
          <span key={`${vital.label ?? "vital"}-${index}`} className={css.item}>
            {index > 0 && <span className={css.separatorDot} />}
            <VitalCell vital={vital} onSave={onSave} />
          </span>
        ))}
      </div>
    </div>
  );
}

function VitalCell({
  vital,
  onSave,
}: {
  vital: CompositionVitalSpec;
  onSave?: (field: string, value: string) => Promise<void> | void;
}) {
  const editable = Boolean(onSave && vital.field);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(vital.raw);

  if (editing && editable) {
    const commit = () => {
      setEditing(false);
      const next = draft.trim();
      if (next !== vital.raw && vital.field) void onSave?.(vital.field, next);
    };
    return (
      <input
        className={editCss.input}
        autoFocus
        value={draft}
        aria-label={vital.label ?? "vital"}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            commit();
          } else if (event.key === "Escape") {
            setDraft(vital.raw);
            setEditing(false);
          }
        }}
      />
    );
  }

  if (!editable) {
    return <span className={css.vitalText}>{vital.display}</span>;
  }

  const beginEdit = () => {
    setDraft(vital.raw);
    setEditing(true);
  };
  return (
    <span
      className={`${css.vitalText} ${editCss.editable}`}
      role="button"
      tabIndex={0}
      title={`Edit ${vital.label ?? "value"}`}
      onClick={beginEdit}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          beginEdit();
        }
      }}
    >
      {vital.display}
    </span>
  );
}
