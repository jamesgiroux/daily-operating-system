/**
 * EditableDate — Date picker using shadcn Popover + Calendar.
 *
 * Renders a clickable date display. On click, opens a calendar popover.
 * Selecting a date commits immediately. Optional clear button removes the date.
 */
import { useState } from "react";
import { Calendar } from "@/components/ui/calendar";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { formatFullDate } from "@/lib/utils";
import styles from "./editable-date.module.css";

interface EditableDateProps {
  value: string;
  onSave: (v: string) => void;
  urgencyColor?: string;
}

export function EditableDate({
  value,
  onSave,
  urgencyColor,
}: EditableDateProps) {
  const [open, setOpen] = useState(false);

  const dateValue = value ? value.split("T")[0] : "";
  // Parse to Date for Calendar's selected prop (noon to avoid timezone shift)
  const selected = dateValue ? new Date(dateValue + "T12:00:00") : undefined;

  function handleSelect(day: Date | undefined) {
    if (!day) return;
    const yyyy = day.getFullYear();
    const mm = String(day.getMonth() + 1).padStart(2, "0");
    const dd = String(day.getDate()).padStart(2, "0");
    onSave(`${yyyy}-${mm}-${dd}`);
    setOpen(false);
  }

  return (
    <span className={styles.wrapper}>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <button
            type="button"
            className={styles.trigger}
            style={urgencyColor ? { color: urgencyColor } : undefined}
          >
            {dateValue ? (
              <span>{formatFullDate(dateValue)}</span>
            ) : (
              <span className={styles.placeholder}>
                Add due date
              </span>
            )}
          </button>
        </PopoverTrigger>
        <PopoverContent align="start" className="w-auto p-0">
          <Calendar
            mode="single"
            selected={selected}
            onSelect={handleSelect}
            defaultMonth={selected}
          />
        </PopoverContent>
      </Popover>
      {dateValue && (
        <button
          onClick={() => onSave("")}
          className={styles.clearButton}
        >
          Clear
        </button>
      )}
    </span>
  );
}
