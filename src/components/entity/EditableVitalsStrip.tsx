/**
 * EditableVitalsStrip — editable version of VitalsStrip that always renders
 * all preset vital fields, including empty ones with placeholder UI.
 *
 * Each field type has an appropriate inline editor:
 *  - currency/number: click to reveal input, commit on blur/Enter
 *  - select: click to cycle through options
 *  - date: click to open DatePicker popover
 *  - text: click to reveal input, commit on blur/Enter
 */
import { useState, useRef, useEffect } from "react";
import type { PresetVitalField } from "@/types/preset";
import { formatArr, formatShortDate } from "@/lib/utils";
import { DatePicker } from "@/components/ui/date-picker";

/** Loose data shape — uses index signature to accept any entity detail type. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type EntityData = Record<string, any>;

interface EditableVitalsStripProps {
  fields: PresetVitalField[];
  entityData: EntityData;
  metadata?: Record<string, string>;
  onFieldChange: (key: string, columnMapping: string | undefined, source: string, value: string) => void;
  /** Extra signal-derived vitals appended read-only (e.g. meeting frequency) */
  extraVitals?: { text: string; highlight?: string }[];
}

const highlightColor: Record<string, string> = {
  turmeric: "var(--color-spice-turmeric)",
  saffron: "var(--color-spice-saffron)",
  olive: "var(--color-garden-olive)",
  larkspur: "var(--color-garden-larkspur)",
};

const HIGHLIGHT_MAP: Record<string, string | undefined> = {
  arr: "turmeric",
  health: undefined,
  status: "olive",
  relationship: "larkspur",
};

const healthColorMap: Record<string, string> = {
  yellow: "saffron",
};

/** Resolve a field value from entity data or metadata. */
function resolveValue(field: PresetVitalField, entityData: EntityData, metadata?: Record<string, string>): string {
  if (field.source === "metadata") {
    return metadata?.[field.key] ?? "";
  }
  if (field.source === "signal") {
    const signals = entityData.signals as Record<string, unknown> | undefined;
    const v = signals?.[field.key];
    return v != null ? String(v) : "";
  }
  // column
  const col = field.columnMapping ?? field.key;
  const colMap: Record<string, string> = {
    arr: "arr",
    health: "health",
    lifecycle: "lifecycle",
    contract_end: "renewalDate",
    contract_start: "contractStart",
    nps: "nps",
    status: "status",
    owner: "owner",
    target_date: "targetDate",
    relationship: "relationship",
  };
  const prop = colMap[col] ?? col;
  const v = entityData[prop];
  return v != null ? String(v) : "";
}

/** Format a value for display in the strip. */
function formatDisplay(field: PresetVitalField, value: string): string {
  if (!value) return "";
  switch (field.fieldType) {
    case "currency": {
      const num = parseFloat(value);
      if (isNaN(num)) return value;
      return `$${formatArr(num)}`;
    }
    case "number":
      return value;
    case "date": {
      const key = field.key;
      if (key === "contract_end" || key === "renewal") {
        return formatRenewalCountdown(value);
      }
      return formatShortDate(value);
    }
    case "select":
    case "text": {
      if (field.key === "health") {
        return value.charAt(0).toUpperCase() + value.slice(1);
      }
      if (field.key === "status") {
        return value.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
      }
      return value;
    }
    default:
      return value;
  }
}

function getHighlight(field: PresetVitalField, value: string): string | undefined {
  if (field.key === "health") return healthColorMap[value];
  return HIGHLIGHT_MAP[field.key];
}

function formatRenewalCountdown(dateStr: string): string {
  try {
    const renewal = new Date(dateStr);
    const now = new Date();
    const diffDays = Math.round((renewal.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));
    if (diffDays < 0) return `${Math.abs(diffDays)}d overdue`;
    return `Renewal in ${diffDays}d`;
  } catch {
    return dateStr;
  }
}

/** Inline text/number input that auto-focuses on mount. */
function InlineInput({
  value,
  onCommit,
  onCancel,
  type,
  prefix,
  label,
}: {
  value: string;
  onCommit: (v: string) => void;
  onCancel: () => void;
  type: "text" | "number";
  prefix?: string;
  label: string;
}) {
  const [draft, setDraft] = useState(value);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    // Auto-focus on mount
    setTimeout(() => inputRef.current?.focus(), 0);
  }, []);

  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 2 }}>
      {prefix && (
        <span style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--color-text-secondary)" }}>
          {prefix}
        </span>
      )}
      <input
        ref={inputRef}
        type={type}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={() => onCommit(draft)}
        onKeyDown={(e) => {
          if (e.key === "Enter") onCommit(draft);
          if (e.key === "Escape") onCancel();
        }}
        placeholder={label}
        style={{
          width: type === "number" ? 80 : 120,
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 500,
          color: "var(--color-text-primary)",
          background: "transparent",
          border: "none",
          borderBottom: "1px solid var(--color-text-tertiary)",
          outline: "none",
          padding: "0 2px",
          textTransform: "uppercase",
          letterSpacing: "0.06em",
        }}
      />
    </span>
  );
}

/** Inline select dropdown that auto-opens on mount. */
function InlineSelect({
  value,
  options,
  onCommit,
  onCancel,
}: {
  value: string;
  options: string[];
  onCommit: (v: string) => void;
  onCancel: () => void;
}) {
  const selectRef = useRef<HTMLSelectElement>(null);

  useEffect(() => {
    // Auto-focus; some browsers will open the dropdown on focus
    setTimeout(() => {
      selectRef.current?.focus();
    }, 0);
  }, []);

  return (
    <select
      ref={selectRef}
      value={value}
      onChange={(e) => onCommit(e.target.value)}
      onBlur={() => onCancel()}
      onKeyDown={(e) => {
        if (e.key === "Escape") onCancel();
      }}
      style={{
        fontFamily: "var(--font-mono)",
        fontSize: 12,
        fontWeight: 500,
        color: "var(--color-text-primary)",
        background: "var(--color-paper-warm-white)",
        border: "1px solid var(--color-rule-light)",
        borderRadius: 4,
        outline: "none",
        padding: "2px 4px",
        textTransform: "uppercase",
        letterSpacing: "0.06em",
        cursor: "pointer",
      }}
    >
      <option value="">Not set</option>
      {options.map((opt) => (
        <option key={opt} value={opt}>
          {opt}
        </option>
      ))}
    </select>
  );
}

/** A single vital field in the strip. */
function VitalField({
  field,
  entityData,
  metadata,
  onFieldChange,
}: {
  field: PresetVitalField;
  entityData: EntityData;
  metadata?: Record<string, string>;
  onFieldChange: EditableVitalsStripProps["onFieldChange"];
}) {
  const [editing, setEditing] = useState(false);
  const value = resolveValue(field, entityData, metadata);
  const isEmpty = !value;
  const isSignal = field.source === "signal";
  const display = isEmpty ? field.label : formatDisplay(field, value);
  const highlight = isEmpty ? undefined : getHighlight(field, value);
  const color = highlight ? highlightColor[highlight] : "var(--color-text-secondary)";

  const commit = (v: string) => {
    onFieldChange(field.key, field.columnMapping, field.source, v);
  };

  // Signal fields are read-only
  if (isSignal) {
    if (isEmpty) return null;
    return (
      <span
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 500,
          textTransform: "uppercase",
          letterSpacing: "0.06em",
          color,
          whiteSpace: "nowrap",
        }}
      >
        {field.fieldType === "currency" ? `${display} ${field.label}` : `${field.label} ${display}`}
      </span>
    );
  }

  // Select: click to show dropdown
  if (field.fieldType === "select" && field.options?.length) {
    return (
      <span style={{ display: "inline-flex", alignItems: "center", whiteSpace: "nowrap" }}>
        {editing ? (
          <InlineSelect
            value={value}
            options={field.options!}
            onCommit={(v) => {
              commit(v);
              setEditing(false);
            }}
            onCancel={() => setEditing(false)}
          />
        ) : (
          <span
            onClick={() => setEditing(true)}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              fontWeight: 500,
              textTransform: "uppercase",
              letterSpacing: "0.06em",
              color: isEmpty ? "var(--color-text-tertiary)" : color,
              opacity: isEmpty ? 0.5 : 1,
              borderBottom: isEmpty ? "1px dashed var(--color-text-tertiary)" : undefined,
              cursor: "pointer",
            }}
            title={`Click to ${isEmpty ? "set" : "change"} ${field.label.toLowerCase()}`}
          >
            {isEmpty ? field.label : `${display} ${field.label}`}
          </span>
        )}
      </span>
    );
  }

  // Date: use DatePicker
  if (field.fieldType === "date") {
    return (
      <span style={{ display: "inline-flex", alignItems: "center", whiteSpace: "nowrap" }}>
        {editing ? (
          <span style={{ display: "inline-block", width: 160 }}>
            <DatePicker
              value={value || undefined}
              onChange={(v) => {
                commit(v);
                setEditing(false);
              }}
              placeholder={`Set ${field.label.toLowerCase()}`}
            />
          </span>
        ) : (
          <span
            onClick={() => setEditing(true)}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              fontWeight: 500,
              textTransform: "uppercase",
              letterSpacing: "0.06em",
              color: isEmpty ? "var(--color-text-tertiary)" : color,
              opacity: isEmpty ? 0.5 : 1,
              borderBottom: isEmpty ? "1px dashed var(--color-text-tertiary)" : undefined,
              cursor: "pointer",
            }}
            title={`Click to ${isEmpty ? "set" : "edit"} ${field.label.toLowerCase()}`}
          >
            {isEmpty ? field.label : display}
          </span>
        )}
      </span>
    );
  }

  // Currency / Number / Text: inline input on click
  const inputType = field.fieldType === "currency" || field.fieldType === "number" ? "number" : "text";
  const prefix = field.fieldType === "currency" ? "$" : undefined;

  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 4, whiteSpace: "nowrap" }}>
      {editing ? (
        <InlineInput
          value={value}
          onCommit={(v) => {
            commit(v);
            setEditing(false);
          }}
          onCancel={() => setEditing(false)}
          type={inputType}
          prefix={prefix}
          label={field.label}
        />
      ) : (
        <span
          onClick={() => setEditing(true)}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            fontWeight: 500,
            textTransform: "uppercase",
            letterSpacing: "0.06em",
            color: isEmpty ? "var(--color-text-tertiary)" : color,
            opacity: isEmpty ? 0.5 : 1,
            borderBottom: isEmpty ? "1px dashed var(--color-text-tertiary)" : undefined,
            cursor: "pointer",
          }}
          title={`Click to ${isEmpty ? "set" : "edit"} ${field.label.toLowerCase()}`}
        >
          {isEmpty
            ? field.label
            : field.fieldType === "currency"
              ? `${display} ${field.label}`
              : field.key === "health"
                ? `${display} ${field.label}`
                : field.key === "status" || field.key === "relationship"
                  ? display
                  : `${field.label} ${display}`}
        </span>
      )}
    </span>
  );
}

export function EditableVitalsStrip({
  fields,
  entityData,
  metadata,
  onFieldChange,
  extraVitals,
}: EditableVitalsStripProps) {
  if (fields.length === 0) return null;

  const allItems: React.ReactNode[] = [];

  for (const field of fields) {
    allItems.push(
      <VitalField
        key={field.key}
        field={field}
        entityData={entityData}
        metadata={metadata}
        onFieldChange={onFieldChange}
      />,
    );
  }

  // Append extra read-only vitals (signal-derived)
  if (extraVitals) {
    for (const ev of extraVitals) {
      allItems.push(
        <span
          key={`extra-${ev.text}`}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            fontWeight: 500,
            textTransform: "uppercase",
            letterSpacing: "0.06em",
            color: ev.highlight ? highlightColor[ev.highlight] ?? "var(--color-text-secondary)" : "var(--color-text-secondary)",
            whiteSpace: "nowrap",
          }}
        >
          {ev.text}
        </span>,
      );
    }
  }

  return (
    <div
      style={{
        marginTop: 48,
        borderTop: "1px solid var(--color-rule-heavy)",
        borderBottom: "1px solid var(--color-rule-heavy)",
        padding: "14px 0",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 24, flexWrap: "wrap" }}>
        {allItems.map((item, i) => (
          <span key={i} style={{ display: "flex", alignItems: "center", gap: 24 }}>
            {i > 0 && (
              <span
                style={{
                  width: 3,
                  height: 3,
                  borderRadius: "50%",
                  background: "var(--color-text-tertiary)",
                  flexShrink: 0,
                }}
              />
            )}
            {item}
          </span>
        ))}
      </div>
    </div>
  );
}
