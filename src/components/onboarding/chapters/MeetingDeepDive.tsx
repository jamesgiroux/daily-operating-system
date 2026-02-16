import { ArrowRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";

interface MeetingDeepDiveProps {
  onNext: () => void;
}

/** Mono uppercase section label */
function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <div
      style={{
        fontFamily: "var(--font-mono)",
        fontSize: 10,
        fontWeight: 500,
        textTransform: "uppercase" as const,
        letterSpacing: "0.1em",
        color: "var(--color-text-tertiary)",
        marginBottom: 8,
      }}
    >
      {children}
    </div>
  );
}

/** Editorial chip — replaces shadcn Badge */
function Chip({ children, color }: { children: React.ReactNode; color?: string }) {
  return (
    <span
      style={{
        fontFamily: "var(--font-mono)",
        fontSize: 11,
        letterSpacing: "0.02em",
        border: "1px solid var(--color-rule-heavy)",
        borderRadius: 4,
        padding: "2px 8px",
        color: color || "var(--color-text-secondary)",
      }}
    >
      {children}
    </span>
  );
}

export function MeetingDeepDive({ onNext }: MeetingDeepDiveProps) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
      <ChapterHeading
        title="This is what prepared looks like"
        epigraph="Every meeting gets this automatically. History, context, risks, talking points — compiled from your data, past meetings, and AI analysis."
      />

      {/* Mock expanded meeting card */}
      <div
        style={{
          borderTop: "2px solid var(--color-spice-turmeric)",
          paddingTop: 20,
        }}
      >
        {/* Meeting header */}
        <div style={{ display: "flex", flexDirection: "column", gap: 4, marginBottom: 20 }}>
          <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-text-tertiary)" }}>
              10:30 AM
            </span>
            <span style={{ color: "var(--color-text-tertiary)", opacity: 0.5 }}>—</span>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-text-tertiary)", opacity: 0.7 }}>
              11:30 AM
            </span>
          </div>
          <h3
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 22,
              fontWeight: 400,
              color: "var(--color-text-primary)",
              margin: 0,
            }}
          >
            Acme Corp Quarterly Sync
          </h3>
          <span style={{ fontSize: 14, color: "var(--color-spice-turmeric)" }}>Acme Corp</span>
        </div>

        {/* Prep content */}
        <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
          {/* Quick Context */}
          <div style={{ borderTop: "1px solid var(--color-rule-light)", paddingTop: 16 }}>
            <SectionLabel>At a Glance</SectionLabel>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
              <Chip>Enterprise</Chip>
              <Chip>$1.2M ARR</Chip>
              <Chip color="var(--color-garden-sage)">Health: Green</Chip>
              <Chip>Ring 1</Chip>
            </div>
          </div>

          {/* Attendees */}
          <div style={{ borderTop: "1px solid var(--color-rule-light)", paddingTop: 16 }}>
            <SectionLabel>Attendees</SectionLabel>
            <div style={{ display: "flex", flexDirection: "column", gap: 4, fontSize: 14 }}>
              <p style={{ margin: 0, color: "var(--color-text-secondary)" }}>
                <span style={{ fontWeight: 500, color: "var(--color-text-primary)" }}>Sarah Chen</span> — VP Engineering{" "}
                <span style={{ fontSize: 12, color: "var(--color-spice-turmeric)" }}>(Decision-maker for expansion)</span>
              </p>
              <p style={{ margin: 0, color: "var(--color-text-secondary)" }}>
                <span style={{ fontWeight: 500, color: "var(--color-text-primary)" }}>Marcus Rivera</span> — Director, Platform{" "}
                <span style={{ fontSize: 12, color: "var(--color-spice-turmeric)" }}>(Day-to-day contact)</span>
              </p>
            </div>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 24 }}>
            {/* Since Last Meeting */}
            <div style={{ borderTop: "1px solid var(--color-rule-light)", paddingTop: 16 }}>
              <SectionLabel>Since Last Meeting</SectionLabel>
              <ul style={{ margin: 0, paddingLeft: 0, listStyle: "none", display: "flex", flexDirection: "column", gap: 4 }}>
                {["Phase 1 migration completed ahead of schedule", "NPS survey deployed — 3 detractors identified", "SOW for Phase 2 sent to legal"].map((item) => (
                  <li key={item} style={{ display: "flex", alignItems: "flex-start", gap: 8, fontSize: 13, color: "var(--color-text-secondary)" }}>
                    <span style={{ marginTop: 6, width: 5, height: 5, borderRadius: "50%", background: "var(--color-text-primary)", flexShrink: 0 }} />
                    {item}
                  </li>
                ))}
              </ul>
            </div>

            {/* Talking Points */}
            <div style={{ borderTop: "1px solid var(--color-rule-light)", paddingTop: 16 }}>
              <SectionLabel><span style={{ color: "var(--color-spice-turmeric)" }}>Talking Points</span></SectionLabel>
              <ul style={{ margin: 0, paddingLeft: 0, listStyle: "none", display: "flex", flexDirection: "column", gap: 4 }}>
                {["Celebrate Phase 1 completion — set up case study", "Address NPS detractor concerns", "Phase 2 timeline and resource needs"].map((item) => (
                  <li key={item} style={{ display: "flex", alignItems: "flex-start", gap: 8, fontSize: 13, color: "var(--color-text-secondary)" }}>
                    <span style={{ marginTop: 6, width: 5, height: 5, borderRadius: "50%", background: "var(--color-spice-turmeric)", flexShrink: 0 }} />
                    {item}
                  </li>
                ))}
              </ul>
            </div>

            {/* Risks */}
            <div style={{ borderTop: "1px solid var(--color-rule-light)", paddingTop: 16 }}>
              <SectionLabel><span style={{ color: "var(--color-spice-terracotta)" }}>Risks</span></SectionLabel>
              <ul style={{ margin: 0, paddingLeft: 0, listStyle: "none", display: "flex", flexDirection: "column", gap: 4 }}>
                {["Key engineer leaving in March — knowledge transfer at risk", "NPS trending down — 3 detractors need follow-up"].map((item) => (
                  <li key={item} style={{ display: "flex", alignItems: "flex-start", gap: 8, fontSize: 13, color: "var(--color-text-secondary)" }}>
                    <span style={{ marginTop: 6, width: 5, height: 5, borderRadius: "50%", background: "var(--color-spice-terracotta)", flexShrink: 0 }} />
                    {item}
                  </li>
                ))}
              </ul>
            </div>

            {/* Open Items */}
            <div style={{ borderTop: "1px solid var(--color-rule-light)", paddingTop: 16 }}>
              <SectionLabel>Open Items</SectionLabel>
              <ul style={{ margin: 0, paddingLeft: 0, listStyle: "none", display: "flex", flexDirection: "column", gap: 4 }}>
                <li style={{ display: "flex", alignItems: "flex-start", gap: 8, fontSize: 13, color: "var(--color-text-secondary)" }}>
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      fontWeight: 500,
                      color: "var(--color-spice-terracotta)",
                      marginTop: 2,
                      flexShrink: 0,
                    }}
                  >
                    OVERDUE
                  </span>
                  Send updated SOW to legal team
                </li>
                <li style={{ display: "flex", alignItems: "flex-start", gap: 8, fontSize: 13, color: "var(--color-text-secondary)" }}>
                  <span style={{ marginTop: 6, width: 5, height: 5, borderRadius: "50%", background: "var(--color-text-tertiary)", flexShrink: 0 }} />
                  Follow up on NPS survey responses
                </li>
              </ul>
            </div>
          </div>
        </div>
      </div>

      {/* Post-meeting teaser */}
      <div
        style={{
          borderTop: "1px solid var(--color-rule-light)",
          paddingTop: 20,
          fontSize: 14,
          color: "var(--color-text-secondary)",
        }}
      >
        <span style={{ fontWeight: 500, color: "var(--color-text-primary)" }}>After a meeting:</span>{" "}
        attach a transcript or capture quick outcomes. They feed into the next prep — wins, risks,
        actions, and decisions carry forward automatically.
      </div>

      <div className="flex justify-end">
        <Button onClick={onNext}>
          Continue
          <ArrowRight className="ml-2 size-4" />
        </Button>
      </div>
    </div>
  );
}
