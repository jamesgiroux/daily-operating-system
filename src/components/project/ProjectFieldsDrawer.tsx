/**
 * ProjectFieldsDrawer — Sheet drawer for editing project structured fields.
 * Simpler than account (no ARR/NPS/renewal).
 */
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { DatePicker } from "@/components/ui/date-picker";

interface ProjectFieldsDrawerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  editName: string;
  setEditName: (v: string) => void;
  editStatus: string;
  setEditStatus: (v: string) => void;
  editMilestone: string;
  setEditMilestone: (v: string) => void;
  editOwner: string;
  setEditOwner: (v: string) => void;
  editTargetDate: string;
  setEditTargetDate: (v: string) => void;
  setDirty: (v: boolean) => void;
  onSave: () => void;
  onCancel: () => void;
  saving: boolean;
  dirty: boolean;
}

const statusOptions = ["active", "on_hold", "completed", "archived"];

const labelStyle: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 10,
  fontWeight: 500,
  textTransform: "uppercase",
  letterSpacing: "0.1em",
  color: "var(--color-text-tertiary)",
  marginBottom: 6,
};

const inputStyle: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: 14,
  color: "var(--color-text-primary)",
  background: "none",
  border: "none",
  borderBottom: "1px solid var(--color-rule-light)",
  outline: "none",
  padding: "6px 0",
  width: "100%",
};

const selectStyle: React.CSSProperties = {
  ...inputStyle,
  cursor: "pointer",
};

export function ProjectFieldsDrawer({
  open,
  onOpenChange,
  editName,
  setEditName,
  editStatus,
  setEditStatus,
  editMilestone,
  setEditMilestone,
  editOwner,
  setEditOwner,
  editTargetDate,
  setEditTargetDate,
  setDirty,
  onSave,
  onCancel,
  saving,
  dirty,
}: ProjectFieldsDrawerProps) {
  function change<T>(setter: (v: T) => void) {
    return (v: T) => {
      setter(v);
      setDirty(true);
    };
  }

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" style={{ width: 380, padding: "32px 28px" }}>
        <SheetHeader>
          <SheetTitle
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 24,
              fontWeight: 400,
              color: "var(--color-text-primary)",
            }}
          >
            Edit Fields
          </SheetTitle>
        </SheetHeader>

        <div style={{ marginTop: 32, display: "flex", flexDirection: "column", gap: 24 }}>
          {/* Name */}
          <div>
            <div style={labelStyle}>Name</div>
            <input
              value={editName}
              onChange={(e) => change(setEditName)(e.target.value)}
              style={inputStyle}
            />
          </div>

          {/* Status */}
          <div>
            <div style={labelStyle}>Status</div>
            <select
              value={editStatus}
              onChange={(e) => change(setEditStatus)(e.target.value)}
              style={selectStyle}
            >
              {statusOptions.map((s) => (
                <option key={s} value={s}>
                  {s.replace("_", " ").replace(/\b\w/g, (c) => c.toUpperCase())}
                </option>
              ))}
            </select>
          </div>

          {/* Milestone */}
          <div>
            <div style={labelStyle}>Current Milestone</div>
            <input
              value={editMilestone}
              onChange={(e) => change(setEditMilestone)(e.target.value)}
              placeholder="Current milestone"
              style={inputStyle}
            />
          </div>

          {/* Owner */}
          <div>
            <div style={labelStyle}>Owner</div>
            <input
              value={editOwner}
              onChange={(e) => change(setEditOwner)(e.target.value)}
              placeholder="Project owner"
              style={inputStyle}
            />
          </div>

          {/* Target Date */}
          <div>
            <div style={labelStyle}>Target Date</div>
            <DatePicker
              value={editTargetDate}
              onChange={change(setEditTargetDate)}
              placeholder="Set target date"
            />
          </div>
        </div>

        {/* Save / Cancel */}
        <div style={{ marginTop: 40, display: "flex", gap: 12 }}>
          <button
            onClick={onSave}
            disabled={saving || !dirty}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 500,
              textTransform: "uppercase",
              letterSpacing: "0.06em",
              color: dirty ? "var(--color-garden-olive)" : "var(--color-text-tertiary)",
              background: "none",
              border: "none",
              cursor: dirty ? "pointer" : "default",
              padding: 0,
            }}
          >
            {saving ? "Saving…" : "Save"}
          </button>
          <button
            onClick={onCancel}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 500,
              textTransform: "uppercase",
              letterSpacing: "0.06em",
              color: "var(--color-text-tertiary)",
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: 0,
            }}
          >
            Cancel
          </button>
        </div>
      </SheetContent>
    </Sheet>
  );
}
