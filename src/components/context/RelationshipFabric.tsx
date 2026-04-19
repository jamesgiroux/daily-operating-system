/**
 * RelationshipFabric — Chapter 8 of the Context tab.
 *
 * Renders reference-weight rows describing the relationship surface around
 * the account — advocacy, beta programs, NPS history, case studies, etc.
 * Most fields aren't yet captured in schema; they render as gap sentinels.
 *
 * DOS-18: chapter implementation. No new schema.
 * Mockup: .fabric-list / "Relationship fabric" in
 *         .docs/mockups/account-context-globex.html
 */
import type { AccountDetail, StrategicProgram } from "@/types";

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
  /** When string, rendered as plain muted gap text. When ReactNode, rendered as-is. */
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
    <span style={{ display: "inline-flex", alignItems: "baseline", flexWrap: "wrap", gap: 8 }}>
      <span
        style={{
          fontFamily: "var(--font-serif)",
          fontWeight: 500,
          fontStyle: "normal",
          color: "var(--color-garden-rosemary)",
          fontSize: 18,
          marginRight: 4,
        }}
      >
        {npsText}
      </span>
      {npsQuote && (
        <span
          style={{
            background: "var(--color-garden-sage-15, rgba(126,170,123,0.15))",
            padding: "6px 12px",
            borderRadius: "var(--radius-sm, 4px)",
            fontFamily: "var(--font-serif)",
            fontSize: 14,
            fontStyle: "italic",
            color: "var(--color-text-primary)",
          }}
        >
          {npsQuote}
        </span>
      )}
      <a
        href="#their-voice"
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 9,
          textTransform: "uppercase",
          letterSpacing: "0.08em",
          color: "var(--color-spice-turmeric)",
          borderBottom: "1px dotted var(--color-spice-turmeric)",
          textDecoration: "none",
          marginLeft: 4,
        }}
      >
        See quote wall →
      </a>
    </span>
  ) : (
    "— not captured"
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

  // Closing editorial copy always renders — the chapter's parting thought sits
  // beside the list whether fields are captured or not (per mockup).

  return (
    <div>
      <div style={{ display: "flex", flexDirection: "column" }}>
        {rows.map((row) => (
          <div
            key={row.label}
            style={{
              display: "grid",
              gridTemplateColumns: "200px 1fr",
              gap: 24,
              padding: "10px 0",
              borderBottom: "1px solid var(--color-rule-light)",
              alignItems: "baseline",
            }}
          >
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                textTransform: "uppercase",
                letterSpacing: "0.08em",
                color: "var(--color-text-tertiary)",
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
                fontStyle: row.gap ? "italic" : "normal",
              }}
            >
              {row.content}
            </span>
          </div>
        ))}
      </div>

      {(() => {
        const subject = accountName && accountName.trim().length > 0 ? accountName : "this account";
        const npsClause = npsText
          ? `Given the NPS ${npsText} and healthy relationship, this is`
          : "If the relationship is healthy, this is";
        return (
          <div
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 16,
              lineHeight: 1.65,
              color: "var(--color-text-secondary)",
              marginTop: 32,
              padding: "24px 32px",
              background: "var(--color-paper-warm-white)",
              borderLeft: "2px solid var(--color-text-tertiary)",
              borderRadius: "0 var(--radius-md, 6px) var(--radius-md, 6px) 0",
              fontStyle: "italic",
              maxWidth: 720,
            }}
          >
            We haven&apos;t systematically captured advocacy signal for {subject}.{" "}
            {npsClause} a likely source of unrealized value.
          </div>
        );
      })()}
    </div>
  );
}
