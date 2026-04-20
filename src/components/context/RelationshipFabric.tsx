/**
 * RelationshipFabric — Chapter 8 of the Context tab.
 *
 * Renders reference-weight rows describing the relationship surface around
 * the account — advocacy, beta programs, NPS history, case studies, etc.
 * Most fields aren't yet captured in schema; they render as saffron-italic
 * gap sentinels.
 *
 * Full field capture + Intelligence Loop wiring is tracked in DOS-251 for
 * v1.2.2. Until then, only Beta programs + NPS light up from existing data.
 *
 * Mockup: `.docs/mockups/account-context-globex.html` Chapter 8 (.fabric-list).
 */
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
}

interface FabricRow {
  label: string;
  content: React.ReactNode;
  gap?: boolean;
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

export function RelationshipFabric({ detail, npsQuote, accountName }: RelationshipFabricProps) {
  const npsText = detail.nps != null ? String(detail.nps) : null;
  const beta = formatBetaPrograms(detail.strategicPrograms);

  const npsContent: React.ReactNode = npsText ? (
    <span className={css.npsBlock}>
      <span className={css.npsScore}>{npsText}</span>
      {npsQuote && <span className={css.npsQuote}>{npsQuote}</span>}
      <a href="#their-voice" className={css.xrefPill}>See quote wall →</a>
    </span>
  ) : (
    "Unknown — not captured"
  );

  const rows: FabricRow[] = [
    { label: "Reference customer", content: "Unknown — not captured", gap: true },
    { label: "Logo permission", content: "Unknown — not captured", gap: true },
    { label: "Case study", content: "None — not captured", gap: true },
    {
      label: "Beta programs",
      content: beta ?? "None captured",
      gap: !beta,
    },
    {
      label: "NPS",
      content: npsContent,
      gap: !npsText,
    },
    { label: "Speaking slots", content: "None captured", gap: true },
    { label: "Referrals made", content: "None captured", gap: true },
    { label: "Advocacy trend", content: "Unknown — not captured", gap: true },
  ];

  const subject = accountName && accountName.trim().length > 0 ? accountName : "this account";
  const npsClause = npsText
    ? `Given the NPS ${npsText} and healthy relationship, this is`
    : "If the relationship is healthy, this is";

  return (
    <div>
      <div className={css.list}>
        {rows.map((row) => (
          <div key={row.label} className={css.row}>
            <span className={css.label}>{row.label}</span>
            <span className={row.gap ? `${css.content} ${css.contentGap}` : css.content}>
              {row.content}
            </span>
          </div>
        ))}
      </div>

      <div className={css.editorialClose}>
        We haven&apos;t systematically captured advocacy signal for {subject}.{" "}
        {npsClause} a likely source of unrealized value.
      </div>

      <div className={css.caveat}>
        Full field capture coming in the next release of DailyOS
      </div>
    </div>
  );
}
