/**
 * CommercialShape — Chapter 6 of the Context tab (reference weight).
 *
 * Renders a two-column reference grid of commercial facts about the account.
 * Rows map to existing columns on `accounts` / `account_technical_footprint`
 * where possible; fields with no backing column render as "gap" sentinels.
 *
 * DOS-18: chapter implementation. No new schema.
 * DOS-231: simple string fields expose an inline "Capture now" affordance
 *          via EditableText bound to update_account_field; structured fields
 *          (discount history, procurement complexity) stay read-only gap
 *          sentinels until a dedicated editor lands (tracked for v1.2.2).
 *
 * Mockup: .ref-grid / "Commercial shape" in
 *         .docs/mockups/account-context-globex.html
 */
import { EditableText } from "@/components/ui/EditableText";
import type { AccountDetail } from "@/types";

interface CommercialShapeProps {
  detail: AccountDetail;
  /** Thin wrapper over update_account_field (useAccountFieldSave). */
  onUpdateField?: (field: string, value: string) => Promise<void> | void;
}

interface ShapeRow {
  label: string;
  value: string;
  gap: boolean;
  /** When set, the row exposes an inline editor that writes to this account column. */
  editableField?: string;
  /** Current string form of the editable value (empty string when gap). */
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
  const npsText = detail.nps != null ? String(detail.nps) : "";

  const rows: ShapeRow[] = [
    {
      label: "ARR (current)",
      value: arrText || "— not captured",
      gap: !arrText,
      editableField: "arr",
      editableValue: detail.arr != null ? String(detail.arr) : "",
    },
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
    {
      label: "Contract start",
      value: contractStartText || "— not captured",
      gap: !contractStartText,
      editableField: "contract_start",
      editableValue: detail.contractStart ?? "",
    },
    {
      label: "NPS",
      value: npsText || "— not captured",
      gap: !npsText,
      editableField: "nps",
      editableValue: npsText,
    },
    // DOS-231: remaining rows are gap sentinels — no column exists on accounts.
    // Structured capture (procurement, discount history) lands with the
    // Context schema work in v1.2.2 (DOS-207). Simple string gap rows stay
    // read-only until their column exists to avoid silent data loss.
    { label: "Multi-year remaining", value: "— not captured", gap: true },
    { label: "Customer fiscal year", value: "— not captured", gap: true },
    { label: "Previous renewal outcome", value: "— not captured", gap: true },
    { label: "Procurement complexity", value: "— not yet surveyed", gap: true },
    { label: "Discount history", value: "— none captured", gap: true },
    { label: "Discount appetite", value: "— unknown", gap: true },
    { label: "Payment behavior", value: "— unknown", gap: true },
    { label: "Budget holder", value: "— unknown", gap: true },
  ];

  const gapCount = rows.filter((r) => r.gap).length;

  return (
    <div>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 1fr",
          rowGap: 8,
          columnGap: 32,
        }}
      >
        {rows.map((row) => {
          const canEdit = Boolean(onUpdateField && row.editableField);
          return (
            <div
              key={row.label}
              style={{
                display: "flex",
                justifyContent: "space-between",
                padding: "6px 0",
                borderBottom: "1px solid var(--color-rule-light)",
                gap: 16,
              }}
            >
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  textTransform: "uppercase",
                  letterSpacing: "0.08em",
                  color: "var(--color-text-tertiary)",
                  flexShrink: 0,
                }}
              >
                {row.label}
              </span>
              <span
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 13,
                  color: row.gap
                    ? "var(--color-text-tertiary)"
                    : "var(--color-text-primary)",
                  fontStyle: row.gap && !canEdit ? "italic" : "normal",
                  textAlign: "right",
                  minWidth: 0,
                }}
              >
                {canEdit ? (
                  <EditableText
                    value={row.editableValue ?? ""}
                    placeholder={row.gap ? "Capture now →" : row.value}
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

      {gapCount > 0 && (
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            textTransform: "uppercase",
            letterSpacing: "0.08em",
            color: "var(--color-spice-saffron)",
            marginTop: 20,
            padding: "8px 12px",
            background:
              "var(--color-spice-saffron-8, rgba(196,147,53,0.06))",
            border: "1px dashed var(--color-spice-saffron)",
          }}
        >
          {gapCount} of {rows.length} commercial fields unfilled
        </div>
      )}
    </div>
  );
}
