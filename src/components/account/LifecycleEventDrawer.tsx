/**
 * LifecycleEventDrawer — Sheet for recording lifecycle events.
 * Event type, date, ARR impact, notes.
 */
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { DatePicker } from "@/components/ui/date-picker";

interface LifecycleEventDrawerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  newEventType: string;
  setNewEventType: (v: string) => void;
  newEventDate: string;
  setNewEventDate: (v: string) => void;
  newArrImpact: string;
  setNewArrImpact: (v: string) => void;
  newEventNotes: string;
  setNewEventNotes: (v: string) => void;
  onSave: () => Promise<void>;
}

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

const selectStyle: React.CSSProperties = {
  width: "100%",
  padding: "8px 12px",
  borderRadius: 4,
  border: "1px solid var(--color-rule-light)",
  background: "var(--color-paper-warm-white)",
  fontFamily: "var(--font-sans)",
  fontSize: 14,
  color: "var(--color-text-primary)",
  outline: "none",
  height: 38,
};

export function LifecycleEventDrawer({
  open,
  onOpenChange,
  newEventType,
  setNewEventType,
  newEventDate,
  setNewEventDate,
  newArrImpact,
  setNewArrImpact,
  newEventNotes,
  setNewEventNotes,
  onSave,
}: LifecycleEventDrawerProps) {
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" style={{ width: 380, padding: 32 }}>
        <SheetHeader>
          <SheetTitle
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 22,
              fontWeight: 400,
              color: "var(--color-text-primary)",
            }}
          >
            Record Event
          </SheetTitle>
          <SheetDescription style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-tertiary)" }}>
            Record a lifecycle event for this account.
          </SheetDescription>
        </SheetHeader>

        <div style={{ display: "flex", flexDirection: "column", gap: 20, marginTop: 24 }}>
          <div>
            <label style={labelStyle}>Event Type</label>
            <select
              value={newEventType}
              onChange={(e) => setNewEventType(e.target.value)}
              style={selectStyle}
            >
              <option value="renewal">Renewal</option>
              <option value="expansion">Expansion</option>
              <option value="churn">Churn</option>
              <option value="downgrade">Downgrade</option>
            </select>
          </div>

          <div>
            <label style={labelStyle}>Date</label>
            <DatePicker
              value={newEventDate}
              onChange={setNewEventDate}
              placeholder="Select date"
            />
          </div>

          <div>
            <label style={labelStyle}>ARR Impact</label>
            <Input
              type="number"
              value={newArrImpact}
              onChange={(e) => setNewArrImpact(e.target.value)}
              placeholder="Annual revenue impact"
            />
          </div>

          <div>
            <label style={labelStyle}>Notes</label>
            <Input
              value={newEventNotes}
              onChange={(e) => setNewEventNotes(e.target.value)}
              placeholder="Optional notes"
            />
          </div>
        </div>

        <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 32 }}>
          <Button
            variant="ghost"
            onClick={() => onOpenChange(false)}
            style={{ fontFamily: "var(--font-sans)", fontSize: 13 }}
          >
            Cancel
          </Button>
          <Button
            onClick={async () => {
              await onSave();
              onOpenChange(false);
            }}
            disabled={!newEventDate}
            style={{ fontFamily: "var(--font-sans)", fontSize: 13 }}
          >
            Save Event
          </Button>
        </div>
      </SheetContent>
    </Sheet>
  );
}
