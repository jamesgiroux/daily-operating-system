/**
 * StakeholderGallery — People chapter.
 * 2-column grid of stakeholder cards with colored engagement badges.
 * Falls back to linkedPeople when no intelligence stakeholders exist.
 * Includes an optional "Your Team" strip for account team members.
 * Generalized: configurable title/id, accountTeam optional.
 */
import { Link } from "@tanstack/react-router";
import type { EntityIntelligence, Person, AccountTeamMember } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";

interface StakeholderGalleryProps {
  intelligence: EntityIntelligence | null;
  linkedPeople: Person[];
  accountTeam?: AccountTeamMember[];
  sectionId?: string;
  chapterTitle?: string;
  emptyMessage?: string;
}

const engagementBadgeStyles: Record<string, { background: string; color: string }> = {
  champion: { background: "rgba(201, 162, 39, 0.12)", color: "var(--color-spice-turmeric)" },
  high: { background: "rgba(126, 170, 123, 0.14)", color: "var(--color-garden-rosemary)" },
  medium: { background: "rgba(143, 163, 196, 0.14)", color: "var(--color-garden-larkspur)" },
  low: { background: "rgba(196, 101, 74, 0.10)", color: "var(--color-spice-terracotta)" },
  neutral: { background: "rgba(143, 163, 196, 0.14)", color: "var(--color-garden-larkspur)" },
  detractor: { background: "rgba(196, 101, 74, 0.10)", color: "var(--color-spice-terracotta)" },
};

const defaultBadgeStyle = {
  background: "rgba(143, 163, 196, 0.14)",
  color: "var(--color-text-tertiary)",
};

function buildEpigraph(stakeholders: { name: string }[]): string {
  const count = stakeholders.length;
  if (count === 0) return "";
  const numberWords: Record<number, string> = {
    1: "One", 2: "Two", 3: "Three", 4: "Four", 5: "Five",
    6: "Six", 7: "Seven", 8: "Eight", 9: "Nine", 10: "Ten",
    11: "Eleven", 12: "Twelve",
  };
  const word = numberWords[count] ?? String(count);
  const noun = count === 1 ? "stakeholder shapes" : "stakeholders shape";
  return `${word} ${noun} this relationship across the organization.`;
}

export function StakeholderGallery({
  intelligence,
  linkedPeople,
  accountTeam,
  sectionId = "the-room",
  chapterTitle = "The Room",
  emptyMessage = "No people linked yet.",
}: StakeholderGalleryProps) {
  const stakeholders = intelligence?.stakeholderInsights ?? [];
  const hasStakeholders = stakeholders.length > 0;
  const epigraph = hasStakeholders ? buildEpigraph(stakeholders) : undefined;
  const teamMembers = accountTeam ?? [];

  return (
    <section id={sectionId} style={{ scrollMarginTop: 60, paddingTop: 80 }}>
      <ChapterHeading title={chapterTitle} epigraph={epigraph} />

      {hasStakeholders ? (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "40px 48px" }}>
          {stakeholders.map((s, i) => {
            const matched = linkedPeople.find(
              (p) => p.name.toLowerCase() === s.name.toLowerCase()
            );
            const card = (
              <div key={i}>
                <div style={{ display: "flex", alignItems: "baseline", gap: 10, marginBottom: 8, flexWrap: "wrap" }}>
                  <span style={{ fontFamily: "var(--font-sans)", fontSize: 16, fontWeight: 500, color: "var(--color-text-primary)" }}>
                    {s.name}
                  </span>
                  {s.engagement && (
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 9,
                        fontWeight: 500,
                        textTransform: "uppercase",
                        letterSpacing: "0.08em",
                        padding: "2px 7px",
                        borderRadius: 3,
                        ...(engagementBadgeStyles[s.engagement.toLowerCase()] ?? defaultBadgeStyle),
                      }}
                    >
                      {s.engagement}
                    </span>
                  )}
                </div>
                {s.role && (
                  <p style={{ fontFamily: "var(--font-sans)", fontSize: 13, fontWeight: 400, color: "var(--color-text-tertiary)", margin: "0 0 8px 0" }}>
                    {s.role}
                  </p>
                )}
                {s.assessment && (
                  <p style={{ fontFamily: "var(--font-sans)", fontSize: 14, lineHeight: 1.6, color: "var(--color-text-secondary)", margin: 0 }}>
                    {s.assessment}
                  </p>
                )}
              </div>
            );

            if (matched) {
              return (
                <Link key={i} to="/people/$personId" params={{ personId: matched.id }} style={{ textDecoration: "none", color: "inherit" }}>
                  {card}
                </Link>
              );
            }
            return card;
          })}
        </div>
      ) : linkedPeople.length > 0 ? (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "40px 48px" }}>
          {linkedPeople.map((p) => (
            <Link key={p.id} to="/people/$personId" params={{ personId: p.id }} style={{ textDecoration: "none", color: "inherit" }}>
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 16, fontWeight: 500, color: "var(--color-text-primary)" }}>
                {p.name}
              </span>
              {p.role && (
                <p style={{ fontFamily: "var(--font-sans)", fontSize: 13, fontWeight: 400, color: "var(--color-text-tertiary)", margin: "4px 0 0 0" }}>
                  {p.role}
                </p>
              )}
            </Link>
          ))}
        </div>
      ) : (
        <p style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-tertiary)", fontStyle: "italic" }}>
          {emptyMessage}
        </p>
      )}

      {/* Your Team strip */}
      {teamMembers.length > 0 && (
        <div
          style={{
            borderTop: "1px solid var(--color-rule-heavy)",
            borderBottom: "1px solid var(--color-rule-heavy)",
            padding: "14px 0",
            marginTop: 40,
            display: "flex",
            alignItems: "baseline",
            gap: 24,
            flexWrap: "wrap",
          }}
        >
          <span style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 500, textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--color-text-tertiary)" }}>
            Your Team
          </span>
          {teamMembers.map((member) => (
            <span key={member.personId} style={{ display: "inline-flex", alignItems: "baseline", gap: 6 }}>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 500, textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--color-text-tertiary)" }}>
                {member.role}
              </span>
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-secondary)" }}>
                {member.personName}
              </span>
            </span>
          ))}
        </div>
      )}
    </section>
  );
}
