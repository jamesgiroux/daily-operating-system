/**
 * PresetFieldsEditor — Renders preset metadata fields as an editable form (I312).
 *
 * Takes a list of PresetMetadataField definitions and a key-value map of current
 * values, renders the appropriate input for each field type.
 */
import { DatePicker } from "@/components/ui/date-picker";
import type { PresetMetadataField } from "@/types/preset";

interface PresetFieldsEditorProps {
  fields: PresetMetadataField[];
  values: Record<string, string>;
  onChange: (key: string, value: string) => void;
}

const inputStyle: React.CSSProperties = {
  width: "100%",
  padding: "8px 12px",
  borderRadius: 4,
  border: "1px solid var(--color-rule-light)",
  background: "var(--color-paper-warm-white)",
  fontFamily: "var(--font-sans)",
  fontSize: 14,
  color: "var(--color-text-primary)",
  outline: "none",
};

const labelStyle: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 10,
  fontWeight: 600,
  textTransform: "uppercase",
  letterSpacing: "0.06em",
  color: "var(--color-text-tertiary)",
  marginBottom: 4,
  display: "block",
};

export function PresetFieldsEditor({
  fields,
  values,
  onChange,
}: PresetFieldsEditorProps) {
  if (fields.length === 0) return null;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 10,
          fontWeight: 600,
          letterSpacing: "0.08em",
          textTransform: "uppercase",
          color: "var(--color-text-tertiary)",
          borderTop: "1px solid var(--color-rule-light)",
          paddingTop: 16,
        }}
      >
        Custom Fields
      </div>
      {fields.map((field) => (
        <div key={field.key}>
          <label style={labelStyle}>
            {field.label}
            {field.required && (
              <span style={{ color: "var(--color-spice-terracotta)", marginLeft: 2 }}>*</span>
            )}
          </label>
          {renderField(field, values[field.key] ?? "", (v) => onChange(field.key, v))}
        </div>
      ))}
    </div>
  );
}

function renderField(
  field: PresetMetadataField,
  value: string,
  onChange: (v: string) => void,
) {
  switch (field.fieldType) {
    case "select":
      return (
        <select
          value={value}
          onChange={(e) => onChange(e.target.value)}
          style={{ ...inputStyle, height: 38 }}
        >
          <option value="">Not set</option>
          {(field.options ?? []).map((opt) => (
            <option key={opt} value={opt}>
              {opt}
            </option>
          ))}
        </select>
      );
    case "number":
      return (
        <input
          type="number"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={field.label}
          style={inputStyle}
        />
      );
    case "date":
      return (
        <DatePicker
          value={value}
          onChange={onChange}
          placeholder={`Set ${field.label.toLowerCase()}`}
        />
      );
    case "text":
    default:
      return (
        <input
          type="text"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={field.label}
          style={inputStyle}
        />
      );
  }
}
