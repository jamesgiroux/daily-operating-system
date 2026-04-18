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

export function RelationshipFabric({ detail }: RelationshipFabricProps) {
  const npsText = detail.nps != null ? String(detail.nps) : null;
  const beta = formatBetaPrograms(detail.strategicPrograms);

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
      content: npsText ? (
        <span
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 20,
            color: "var(--color-text-primary)",
          }}
        >
          {npsText}
        </span>
      ) : (
        "— not captured"
      ),
      gap: !npsText,
    },
    { label: "Speaking slots", content: "None captured", gap: true },
    { label: "Referrals made", content: "None captured", gap: true },
    { label: "Advocacy trend", content: "Unknown — not captured", gap: true },
  ];

  const allGaps = rows.every((r) => r.gap);

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

      {allGaps && (
        <div
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 14,
            lineHeight: 1.6,
            color: "var(--color-text-secondary)",
            marginTop: 20,
            paddingTop: 16,
            borderTop: "1px solid var(--color-rule-light)",
            fontStyle: "italic",
          }}
        >
          We haven't systematically captured advocacy signal for this account.
          If the relationship is healthy, this is a likely source of unrealized
          value.
        </div>
      )}
    </div>
  );
}
