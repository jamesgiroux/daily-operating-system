/**
 * DatePicker — Editorial date picker composing Calendar + Popover.
 * Uses design tokens from design-tokens.css for the editorial palette.
 */
import * as React from "react";
import { CalendarIcon } from "lucide-react";

import { Calendar } from "@/components/ui/calendar";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";

interface DatePickerProps {
  value: string | undefined;
  onChange: (date: string) => void;
  label?: string;
  placeholder?: string;
}

/** Parse "YYYY-MM-DD" into a local Date (noon to avoid timezone drift). */
function parseISO(iso: string): Date | undefined {
  const [y, m, d] = iso.split("-").map(Number);
  if (!y || !m || !d) return undefined;
  const date = new Date(y, m - 1, d, 12);
  return isNaN(date.getTime()) ? undefined : date;
}

/** Format a Date as "YYYY-MM-DD". */
function toISO(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

/** Format a Date as display text, e.g. "Feb 16, 2026". */
function formatDisplay(date: Date): string {
  return date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

function DatePicker({ value, onChange, label, placeholder = "Pick a date" }: DatePickerProps) {
  const [open, setOpen] = React.useState(false);

  const selected = React.useMemo(() => {
    if (!value) return undefined;
    return parseISO(value);
  }, [value]);

  const displayDate = selected ? formatDisplay(selected) : undefined;

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button
          type="button"
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 8,
            padding: "6px 0",
            background: "none",
            border: "none",
            borderBottom: "1px solid var(--color-rule-light)",
            cursor: "pointer",
            width: "100%",
            textAlign: "left",
          }}
        >
          {label && (
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                fontWeight: 500,
                textTransform: "uppercase",
                letterSpacing: "0.1em",
                color: "var(--color-text-tertiary)",
                marginRight: 4,
              }}
            >
              {label}
            </span>
          )}
          <span
            style={{
              fontFamily: displayDate
                ? "var(--font-serif)"
                : "var(--font-sans)",
              fontSize: 14,
              color: displayDate
                ? "var(--color-text-primary)"
                : "var(--color-text-tertiary)",
              flex: 1,
            }}
          >
            {displayDate ?? placeholder}
          </span>
          <CalendarIcon
            size={14}
            style={{ color: "var(--color-text-tertiary)", flexShrink: 0 }}
          />
        </button>
      </PopoverTrigger>
      <PopoverContent
        align="start"
        style={{
          width: "auto",
          padding: 0,
          background: "var(--color-paper-cream)",
          border: "1px solid var(--color-rule-light)",
          borderRadius: "var(--radius-editorial-lg)",
          boxShadow: "var(--shadow-md)",
        }}
      >
        <Calendar
          mode="single"
          selected={selected}
          onSelect={(day) => {
            if (day) {
              onChange(toISO(day));
            } else {
              onChange("");
            }
            setOpen(false);
          }}
          className="editorial-calendar"
          classNames={{
            caption_label: "editorial-calendar-caption",
            weekday: "editorial-calendar-weekday",
            today: "editorial-calendar-today",
          }}
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 13,
            color: "var(--color-text-primary)",
          }}
        />
        {value && (
          <div
            style={{
              borderTop: "1px solid var(--color-rule-light)",
              padding: "6px 12px",
              textAlign: "center",
            }}
          >
            <button
              type="button"
              onClick={() => {
                onChange("");
                setOpen(false);
              }}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                fontWeight: 500,
                textTransform: "uppercase",
                letterSpacing: "0.06em",
                color: "var(--color-text-tertiary)",
                background: "none",
                border: "none",
                cursor: "pointer",
                padding: "2px 8px",
              }}
            >
              Clear
            </button>
          </div>
        )}
      </PopoverContent>
    </Popover>
  );
}

export { DatePicker };
