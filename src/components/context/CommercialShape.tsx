/**
 * CommercialShape — Chapter 6 of the Context tab (reference weight).
 *
 * Renders a two-column reference grid of commercial facts about the account.
 * Rows map to existing columns on `accounts` where possible; fields with no
 * backing column render as saffron-italic gap sentinels.
 *
 * Full field capture + Intelligence Loop wiring is tracked in DOS-251 for
 * v1.2.2 (Role-Aware Intelligence). Until then, only ARR + renewal date are
 * user-editable; the rest are display-only gap rows.
 *
 * Mockup: `.docs/mockups/account-context-globex.html` Chapter 6.
 */
import { EditableText } from "@/components/ui/EditableText";
import type { AccountDetail } from "@/types";
import css from "./ReferenceGrid.module.css";

interface CommercialShapeProps {
  detail: AccountDetail;
  /** Thin wrapper over update_account_field (useAccountFieldSave). */
  onUpdateField?: (field: string, value: string) => Promise<void> | void;
}

interface ShapeRow {
  label: string;
  value: string;
  gap: boolean;
  accent?: boolean;
  editableField?: string;
  editableValue?: string;
}

const currencyFormatter = new Intl.NumberFormat(undefined, {
  style: "currency",
  currency: "USD",
  maximumFractionDigits: 0,
});

function formatArr(arr?: number | null): string {
  if (arr == null || arr === 0) return "";
  return currencyFormatter.format(arr);
}

function formatDate(date?: string | null): string {
  if (!date) return "";
  const d = new Date(date);
  if (isNaN(d.getTime())) return date;
  return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

export function CommercialShape({ detail, onUpdateField }: CommercialShapeProps) {
  const arrText = formatArr(detail.arr);
  const renewalText = formatDate(detail.renewalDate);
  const contractStartText = formatDate(detail.contractStart);

  const rows: ShapeRow[] = [
    {
      label: "ARR (current)",
      value: arrText || "— not captured",
      gap: !arrText,
      accent: !!arrText,
      editableField: "arr",
      editableValue: detail.arr != null ? String(detail.arr) : "",
    },
    { label: "12-month trend", value: "— not yet available", gap: true },
    {
      label: "Contract type",
      value: contractStartText && renewalText ? "Annual" : "— not captured",
      gap: !(contractStartText && renewalText),
    },
    {
      label: "Renewal date",
      value: renewalText || "— not captured",
      gap: !renewalText,
      editableField: "contract_end",
      editableValue: detail.renewalDate ?? "",
    },
    { label: "Auto-renew", value: "— not captured", gap: true },
    { label: "Multi-year remaining", value: "— not captured", gap: true },
    { label: "Customer fiscal year", value: "— not captured", gap: true },
    { label: "Previous renewal outcome", value: "— not captured", gap: true },
    { label: "Procurement complexity", value: "— not yet surveyed", gap: true },
    { label: "Discount history", value: "— none captured", gap: true },
    { label: "Discount appetite", value: "— unknown", gap: true },
    { label: "Payment behavior", value: "— unknown", gap: true },
    { label: "Budget holder", value: "— unknown", gap: true },
  ];

  return (
    <div>
      <div className={css.grid}>
        {rows.map((row) => {
          const canEdit = Boolean(onUpdateField && row.editableField);
          const valueClass = row.gap && !canEdit
            ? `${css.value} ${css.valueGap}`
            : row.accent
              ? `${css.value} ${css.valueAccent}`
              : css.value;
          return (
            <div key={row.label} className={css.row}>
              <span className={css.label}>{row.label}</span>
              <span className={valueClass}>
                {canEdit ? (
                  <EditableText
                    value={row.editableValue ?? ""}
                    placeholder={row.gap ? "Capture →" : row.value}
                    onChange={(v) => onUpdateField?.(row.editableField!, v.trim())}
                    as="span"
                    multiline={false}
                  />
                ) : (
                  row.value
                )}
              </span>
            </div>
          );
        })}
      </div>

      <div className={css.caveat}>
        Full field capture coming in the next release of DailyOS
      </div>
    </div>
  );
}
