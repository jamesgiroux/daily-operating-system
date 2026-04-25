/**
 * CommercialShape — Chapter 6 of the Context tab (reference weight).
 *
 * Renders a two-column reference grid of commercial facts about the account.
 * Rows map to existing columns on `accounts` where possible; fields without
 * backing columns persist through namespaced entity metadata.
 *
 * Mockup: `.docs/mockups/account-context-globex.html` Chapter 6.
 */
import { EditableText } from "@/components/ui/EditableText";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { AccountDetail } from "@/types";
import css from "./ReferenceGrid.module.css";

interface CommercialShapeProps {
  detail: AccountDetail;
  /** Thin wrapper over update_account_field (useAccountFieldSave). */
  onUpdateField?: (field: string, value: string) => Promise<void> | void;
  onUpdateMetadata?: (key: string, value: string) => Promise<void> | void;
  metadataValues: Record<string, string>;
}

interface ShapeRow {
  label: string;
  value: string;
  gap: boolean;
  accent?: boolean;
  editableField?: string;
  metadataKey?: string;
  editableValue?: string;
  options?: string[];
}

const CLEAR_SELECT_VALUE = "__clear__";

function EditableSelect({
  value,
  options,
  onChange,
}: {
  value: string;
  options: string[];
  onChange: (value: string) => void;
}) {
  return (
    <Select
      value={value || undefined}
      onValueChange={(nextValue) => onChange(nextValue === CLEAR_SELECT_VALUE ? "" : nextValue)}
    >
      <SelectTrigger size="sm">
        <SelectValue placeholder="Set value..." />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value={CLEAR_SELECT_VALUE}>Clear</SelectItem>
        {options.map((option) => (
          <SelectItem key={option} value={option}>
            {option}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
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

export function CommercialShape({
  detail,
  onUpdateField,
  onUpdateMetadata,
  metadataValues,
}: CommercialShapeProps) {
  const arrText = formatArr(detail.arr);
  const renewalText = formatDate(detail.renewalDate);

  const metadataText = (key: string) => metadataValues[key]?.trim() ?? "";

  const rows: ShapeRow[] = [
    {
      label: "ARR (current)",
      value: arrText || "— not captured",
      gap: !arrText,
      accent: !!arrText,
      editableField: "arr",
      editableValue: detail.arr != null ? String(detail.arr) : "",
    },
    {
      label: "12-month trend",
      value: metadataText("commercial_shape:trend_12mo") || "— not yet available",
      gap: !metadataText("commercial_shape:trend_12mo"),
      metadataKey: "commercial_shape:trend_12mo",
      editableValue: metadataValues["commercial_shape:trend_12mo"] ?? "",
    },
    {
      label: "Contract type",
      value: metadataText("commercial_shape:contract_type") || "— not captured",
      gap: !metadataText("commercial_shape:contract_type"),
      metadataKey: "commercial_shape:contract_type",
      editableValue: metadataValues["commercial_shape:contract_type"] ?? "",
      options: ["Annual", "Multi-year", "Month-to-month", "Custom"],
    },
    {
      label: "Renewal date",
      value: renewalText || "— not captured",
      gap: !renewalText,
      editableField: "contract_end",
      editableValue: detail.renewalDate ?? "",
    },
    {
      label: "Auto-renew",
      value: metadataText("commercial_shape:auto_renew") || "— not captured",
      gap: !metadataText("commercial_shape:auto_renew"),
      metadataKey: "commercial_shape:auto_renew",
      editableValue: metadataValues["commercial_shape:auto_renew"] ?? "",
      options: ["Yes", "No", "Unknown"],
    },
    {
      label: "Multi-year remaining",
      value: metadataText("commercial_shape:multi_year_remaining") || "— not captured",
      gap: !metadataText("commercial_shape:multi_year_remaining"),
      metadataKey: "commercial_shape:multi_year_remaining",
      editableValue: metadataValues["commercial_shape:multi_year_remaining"] ?? "",
    },
    {
      label: "Customer fiscal year",
      value: metadataText("commercial_shape:fiscal_year") || "— not captured",
      gap: !metadataText("commercial_shape:fiscal_year"),
      metadataKey: "commercial_shape:fiscal_year",
      editableValue: metadataValues["commercial_shape:fiscal_year"] ?? "",
    },
    {
      label: "Previous renewal outcome",
      value: metadataText("commercial_shape:prev_renewal_outcome") || "— not captured",
      gap: !metadataText("commercial_shape:prev_renewal_outcome"),
      metadataKey: "commercial_shape:prev_renewal_outcome",
      editableValue: metadataValues["commercial_shape:prev_renewal_outcome"] ?? "",
      options: ["On-time", "Late", "Lost", "Renegotiated", "First renewal"],
    },
    {
      label: "Procurement complexity",
      value: metadataText("commercial_shape:procurement_complexity") || "— not yet surveyed",
      gap: !metadataText("commercial_shape:procurement_complexity"),
      metadataKey: "commercial_shape:procurement_complexity",
      editableValue: metadataValues["commercial_shape:procurement_complexity"] ?? "",
      options: ["Simple", "Standard procurement", "Complex / multi-stakeholder", "Unknown"],
    },
    {
      label: "Discount history",
      value: metadataText("commercial_shape:discount_history") || "— none captured",
      gap: !metadataText("commercial_shape:discount_history"),
      metadataKey: "commercial_shape:discount_history",
      editableValue: metadataValues["commercial_shape:discount_history"] ?? "",
    },
    {
      label: "Discount appetite",
      value: metadataText("commercial_shape:discount_appetite") || "— unknown",
      gap: !metadataText("commercial_shape:discount_appetite"),
      metadataKey: "commercial_shape:discount_appetite",
      editableValue: metadataValues["commercial_shape:discount_appetite"] ?? "",
      options: ["None", "Modest", "Aggressive", "Unknown"],
    },
    {
      label: "Payment behavior",
      value: metadataText("commercial_shape:payment_behavior") || "— unknown",
      gap: !metadataText("commercial_shape:payment_behavior"),
      metadataKey: "commercial_shape:payment_behavior",
      editableValue: metadataValues["commercial_shape:payment_behavior"] ?? "",
      options: ["On-time", "Slow", "Net-30 strict", "Net-60+", "Variable"],
    },
    {
      label: "Budget holder",
      value: metadataText("commercial_shape:budget_holder") || "— unknown",
      gap: !metadataText("commercial_shape:budget_holder"),
      metadataKey: "commercial_shape:budget_holder",
      editableValue: metadataValues["commercial_shape:budget_holder"] ?? "",
    },
  ];

  return (
    <div>
      <div className={css.grid}>
        {rows.map((row) => {
          const canEdit = Boolean(
            (onUpdateField && row.editableField) || (onUpdateMetadata && row.metadataKey),
          );
          const valueClass = row.gap && !canEdit
            ? `${css.value} ${css.valueGap}`
            : row.accent
              ? `${css.value} ${css.valueAccent}`
              : css.value;
          return (
            <div key={row.label} className={css.row}>
              <span className={css.label}>{row.label}</span>
              <span className={valueClass}>
                {canEdit && row.options ? (
                  <EditableSelect
                    value={row.editableValue ?? ""}
                    options={row.options}
                    onChange={(v) => {
                      if (row.editableField) onUpdateField?.(row.editableField, v.trim());
                      else if (row.metadataKey) onUpdateMetadata?.(row.metadataKey, v.trim());
                    }}
                  />
                ) : canEdit ? (
                  <EditableText
                    value={row.editableValue ?? ""}
                    placeholder={row.gap ? "Capture →" : row.value}
                    onChange={(v) => {
                      if (row.editableField) onUpdateField?.(row.editableField, v.trim());
                      else if (row.metadataKey) onUpdateMetadata?.(row.metadataKey, v.trim());
                    }}
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

    </div>
  );
}
