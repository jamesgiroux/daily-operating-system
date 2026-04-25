/**
 * RelationshipFabric — Chapter 8 of the Context tab.
 *
 * Renders reference-weight rows describing the relationship surface around
 * the account — advocacy, beta programs, NPS history, case studies, etc.
 * Metadata-backed rows persist through namespaced entity metadata.
 *
 * Mockup: `.docs/mockups/account-context-globex.html` Chapter 8 (.fabric-list).
 */
import { EditableText } from "@/components/ui/EditableText";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { AccountDetail, StrategicProgram } from "@/types";
import css from "./RelationshipFabric.module.css";

interface RelationshipFabricProps {
  detail: AccountDetail;
  /**
   * Optional verbatim NPS comment. Rendered in a sage highlight next to the
   * score when present. Backend field is not yet wired; the prop is threaded
   * so the UI can light up the moment it exists without another diff.
   */
  npsQuote?: string;
  /**
   * Account name used in the closing editorial copy. Falls back to the
   * generic phrasing when missing so the surface never breaks.
   */
  accountName?: string;
  onUpdateField?: (field: string, value: string) => Promise<void> | void;
  onUpdateMetadata?: (key: string, value: string) => Promise<void> | void;
  metadataValues: Record<string, string>;
}

interface FabricRow {
  label: string;
  content: React.ReactNode;
  gap?: boolean;
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

function formatBetaPrograms(programs: StrategicProgram[] | undefined): React.ReactNode {
  const active = (programs ?? []).filter((p) => {
    const hay = `${p.name ?? ""} ${p.status ?? ""} ${p.notes ?? ""}`.toLowerCase();
    return hay.includes("beta") || hay.includes("pilot");
  });
  if (active.length === 0) return null;
  return (
    <>
      {active.map((p, i) => (
        <span key={i}>
          {i > 0 && ", "}
          <strong>{p.name}</strong>
          {p.status ? ` — ${p.status}` : ""}
        </span>
      ))}
    </>
  );
}

export function RelationshipFabric({
  detail,
  npsQuote,
  accountName,
  onUpdateField,
  onUpdateMetadata,
  metadataValues,
}: RelationshipFabricProps) {
  const npsText = detail.nps != null ? String(detail.nps) : null;
  const beta = formatBetaPrograms(detail.strategicPrograms);
  const metadataText = (key: string) => metadataValues[key]?.trim() ?? "";

  const npsContent: React.ReactNode = (
    <span className={css.npsBlock}>
      <span className={css.npsScore}>
        <EditableText
          value={npsText ?? ""}
          placeholder={npsText ? "NPS" : "Capture →"}
          onChange={(v) => onUpdateField?.("nps", v.trim())}
          as="span"
          multiline={false}
        />
      </span>
      {npsQuote && <span className={css.npsQuote}>{npsQuote}</span>}
      <a href="#their-voice" className={css.xrefPill}>See quote wall →</a>
    </span>
  );

  const rows: FabricRow[] = [
    {
      label: "Reference customer",
      content: metadataText("relationship_fabric:reference_customer") || "Unknown — not captured",
      gap: !metadataText("relationship_fabric:reference_customer"),
      metadataKey: "relationship_fabric:reference_customer",
      editableValue: metadataValues["relationship_fabric:reference_customer"] ?? "",
      options: ["Yes", "No", "Conditional", "Pending"],
    },
    {
      label: "Logo permission",
      content: metadataText("relationship_fabric:logo_permission") || "Unknown — not captured",
      gap: !metadataText("relationship_fabric:logo_permission"),
      metadataKey: "relationship_fabric:logo_permission",
      editableValue: metadataValues["relationship_fabric:logo_permission"] ?? "",
      options: ["Yes", "No", "Pending"],
    },
    {
      label: "Case study",
      content: metadataText("relationship_fabric:case_study") || "None — not captured",
      gap: !metadataText("relationship_fabric:case_study"),
      metadataKey: "relationship_fabric:case_study",
      editableValue: metadataValues["relationship_fabric:case_study"] ?? "",
    },
    {
      label: "Beta programs",
      content: beta ?? "None captured",
      gap: !beta,
    },
    {
      label: "NPS",
      content: npsContent,
      gap: !npsText,
      editableField: "nps",
      editableValue: npsText ?? "",
    },
    {
      label: "Speaking slots",
      content: metadataText("relationship_fabric:speaking_slots") || "None captured",
      gap: !metadataText("relationship_fabric:speaking_slots"),
      metadataKey: "relationship_fabric:speaking_slots",
      editableValue: metadataValues["relationship_fabric:speaking_slots"] ?? "",
    },
    {
      label: "Referrals made",
      content: metadataText("relationship_fabric:referrals_made") || "None captured",
      gap: !metadataText("relationship_fabric:referrals_made"),
      metadataKey: "relationship_fabric:referrals_made",
      editableValue: metadataValues["relationship_fabric:referrals_made"] ?? "",
    },
    {
      label: "Advocacy trend",
      content: metadataText("relationship_fabric:advocacy_trend") || "Unknown — not captured",
      gap: !metadataText("relationship_fabric:advocacy_trend"),
      metadataKey: "relationship_fabric:advocacy_trend",
      editableValue: metadataValues["relationship_fabric:advocacy_trend"] ?? "",
      options: ["Strengthening", "Steady", "Weakening", "Unknown"],
    },
  ];

  const subject = accountName && accountName.trim().length > 0 ? accountName : "this account";
  const npsClause = npsText
    ? `Given the NPS ${npsText} and healthy relationship, this is`
    : "If the relationship is healthy, this is";

  return (
    <div>
      <div className={css.list}>
        {rows.map((row) => {
          const canEdit = Boolean(
            (onUpdateField && row.editableField) || (onUpdateMetadata && row.metadataKey),
          );
          return (
            <div key={row.label} className={css.row}>
              <span className={css.label}>{row.label}</span>
              <span className={row.gap && !canEdit ? `${css.content} ${css.contentGap}` : css.content}>
                {canEdit && row.options ? (
                  <EditableSelect
                    value={row.editableValue ?? ""}
                    options={row.options}
                    onChange={(v) => onUpdateMetadata?.(row.metadataKey!, v.trim())}
                  />
                ) : canEdit && row.metadataKey ? (
                  <EditableText
                    value={row.editableValue ?? ""}
                    placeholder={row.gap ? "Capture →" : String(row.content)}
                    onChange={(v) => onUpdateMetadata?.(row.metadataKey!, v.trim())}
                    as="span"
                    multiline={false}
                  />
                ) : (
                  row.content
                )}
              </span>
            </div>
          );
        })}
      </div>

      <div className={css.editorialClose}>
        We haven&apos;t systematically captured advocacy signal for {subject}.{" "}
        {npsClause} a likely source of unrealized value.
      </div>
    </div>
  );
}
