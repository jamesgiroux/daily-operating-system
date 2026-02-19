/**
 * AccountFieldsDrawer — Sheet for editing account fields.
 * Name, health, lifecycle, ARR, NPS, renewal date.
 */
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { DatePicker } from "@/components/ui/date-picker";
import { PresetFieldsEditor } from "@/components/entity/PresetFieldsEditor";
import type { AccountHealth } from "@/types";
import type { PresetMetadataField } from "@/types/preset";

const healthOptions: AccountHealth[] = ["green", "yellow", "red"];
const lifecycleOptions = ["onboarding", "adoption", "nurture", "renewal", "churned"];

interface AccountFieldsDrawerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  editName: string;
  setEditName: (v: string) => void;
  editHealth: string;
  setEditHealth: (v: string) => void;
  editLifecycle: string;
  setEditLifecycle: (v: string) => void;
  editArr: string;
  setEditArr: (v: string) => void;
  editNps: string;
  setEditNps: (v: string) => void;
  editRenewal: string;
  setEditRenewal: (v: string) => void;
  setDirty: (v: boolean) => void;
  onSave: () => Promise<void>;
  onCancel: () => void;
  saving: boolean;
  dirty: boolean;
  /** I312: Optional preset metadata fields */
  metadataFields?: PresetMetadataField[];
  metadataValues?: Record<string, string>;
  onMetadataChange?: (key: string, value: string) => void;
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

export function AccountFieldsDrawer({
  open,
  onOpenChange,
  editName,
  setEditName,
  editHealth,
  setEditHealth,
  editLifecycle,
  setEditLifecycle,
  editArr,
  setEditArr,
  editNps,
  setEditNps,
  editRenewal,
  setEditRenewal,
  setDirty,
  onSave,
  onCancel,
  saving,
  dirty,
  metadataFields,
  metadataValues,
  onMetadataChange,
}: AccountFieldsDrawerProps) {
  function handleChange<T extends string>(setter: (v: T) => void) {
    return (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
      setter(e.target.value as T);
      setDirty(true);
    };
  }

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" style={{ width: 400, padding: 32 }}>
        <SheetHeader>
          <SheetTitle
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 22,
              fontWeight: 400,
              color: "var(--color-text-primary)",
            }}
          >
            Account Details
          </SheetTitle>
          <SheetDescription style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-tertiary)" }}>
            Edit account fields. Changes are saved when you click Save.
          </SheetDescription>
        </SheetHeader>

        <div style={{ display: "flex", flexDirection: "column", gap: 20, marginTop: 24 }}>
          <div>
            <label style={labelStyle}>Name</label>
            <input
              type="text"
              value={editName}
              onChange={handleChange(setEditName)}
              placeholder="Account name"
              style={inputStyle}
            />
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
            <div>
              <label style={labelStyle}>Health</label>
              <select
                value={editHealth}
                onChange={handleChange(setEditHealth)}
                style={{ ...inputStyle, height: 38 }}
              >
                <option value="">Not set</option>
                {healthOptions.map((h) => (
                  <option key={h} value={h}>{h}</option>
                ))}
              </select>
            </div>
            <div>
              <label style={labelStyle}>Lifecycle</label>
              <select
                value={editLifecycle}
                onChange={handleChange(setEditLifecycle)}
                style={{ ...inputStyle, height: 38 }}
              >
                <option value="">Not set</option>
                {lifecycleOptions.map((s) => (
                  <option key={s} value={s}>{s}</option>
                ))}
              </select>
            </div>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
            <div>
              <label style={labelStyle}>ARR</label>
              <input
                type="number"
                value={editArr}
                onChange={handleChange(setEditArr)}
                placeholder="Annual revenue"
                style={inputStyle}
              />
            </div>
            <div>
              <label style={labelStyle}>NPS</label>
              <input
                type="number"
                value={editNps}
                onChange={handleChange(setEditNps)}
                placeholder="NPS score"
                style={inputStyle}
              />
            </div>
          </div>

          <div>
            <label style={labelStyle}>Renewal Date</label>
            <DatePicker
              value={editRenewal}
              onChange={(v) => { setEditRenewal(v); setDirty(true); }}
              placeholder="Set renewal date"
            />
          </div>

          {metadataFields && metadataFields.length > 0 && metadataValues && onMetadataChange && (
            <PresetFieldsEditor
              fields={metadataFields}
              values={metadataValues}
              onChange={(key, value) => {
                onMetadataChange(key, value);
                setDirty(true);
              }}
            />
          )}
        </div>

        <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 32 }}>
          <Button
            variant="ghost"
            onClick={() => {
              onCancel();
              onOpenChange(false);
            }}
            style={{ fontFamily: "var(--font-sans)", fontSize: 13 }}
          >
            Cancel
          </Button>
          <Button
            onClick={async () => {
              await onSave();
              onOpenChange(false);
            }}
            disabled={saving || !dirty}
            style={{ fontFamily: "var(--font-sans)", fontSize: 13 }}
          >
            {saving ? "Saving..." : "Save"}
          </Button>
        </div>
      </SheetContent>
    </Sheet>
  );
}
