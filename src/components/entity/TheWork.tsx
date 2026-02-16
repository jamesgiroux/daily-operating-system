/**
 * TheWork — Meeting readiness, upcoming meetings, and commitments.
 * Generalized: accepts WorkSource instead of AccountDetail.
 */
import { Link } from "@tanstack/react-router";
import type { EntityIntelligence } from "@/types";
import type { WorkSource } from "@/lib/entity-types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { formatShortDate, formatMeetingType } from "@/lib/utils";
import { classifyAction, meetingTypeBadgeStyle, formatMeetingRowDate } from "@/lib/entity-utils";

interface TheWorkProps {
  data: WorkSource;
  intelligence: EntityIntelligence | null;
  sectionId?: string;
  chapterTitle?: string;
  addingAction?: boolean;
  setAddingAction?: (v: boolean) => void;
  newActionTitle?: string;
  setNewActionTitle?: (v: string) => void;
  creatingAction?: boolean;
  onCreateAction?: () => void;
}

const sectionLabelStyle: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 11,
  fontWeight: 500,
  textTransform: "uppercase",
  letterSpacing: "0.1em",
  color: "var(--color-spice-turmeric)",
  marginBottom: 16,
};

/* ── Action sub-components ── */

interface ActionRowProps {
  action: { id: string; title: string; dueDate?: string; source?: string };
  accentColor?: string;
  dateColor?: string;
  bold?: boolean;
}

function ActionRow({ action, accentColor, dateColor = "var(--color-text-tertiary)", bold }: ActionRowProps) {
  return (
    <Link
      to="/actions/$actionId"
      params={{ actionId: action.id }}
      style={{
        display: "block",
        position: "relative",
        padding: "14px 0 14px 20px",
        borderBottom: "1px solid var(--color-rule-light)",
        textDecoration: "none",
        color: "inherit",
      }}
    >
      {accentColor && (
        <div
          style={{
            position: "absolute",
            left: 0,
            top: 14,
            bottom: 14,
            width: 3,
            borderRadius: 2,
            background: accentColor,
          }}
        />
      )}
      <div
        style={{
          fontFamily: "var(--font-sans)",
          fontSize: 14,
          lineHeight: 1.55,
          fontWeight: bold ? 500 : 400,
          color: "var(--color-text-primary)",
        }}
      >
        {action.title}
      </div>
      {(action.dueDate || action.source) && (
        <div style={{ display: "flex", gap: 16, marginTop: 4 }}>
          {action.dueDate && (
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                fontWeight: 500,
                letterSpacing: "0.04em",
                color: dateColor,
              }}
            >
              {formatShortDate(action.dueDate)}
            </span>
          )}
          {action.source && (
            <span style={{ fontFamily: "var(--font-sans)", fontSize: 12, color: "var(--color-text-tertiary)" }}>
              {action.source}
            </span>
          )}
        </div>
      )}
    </Link>
  );
}

interface ActionGroupProps {
  label: string;
  labelColor: string;
  actions: Array<{ id: string; title: string; dueDate?: string; source?: string }>;
  accentColor?: string;
  dateColor?: string;
  bold?: boolean;
}

function ActionGroup({ label, labelColor, actions, accentColor, dateColor, bold }: ActionGroupProps) {
  if (actions.length === 0) return null;
  return (
    <div style={{ marginBottom: 24 }}>
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 10,
          fontWeight: 500,
          textTransform: "uppercase",
          letterSpacing: "0.1em",
          color: labelColor,
          marginBottom: 12,
          paddingBottom: 6,
          borderBottom: "1px solid var(--color-rule-light)",
        }}
      >
        {label}
      </div>
      {actions.map((a) => (
        <ActionRow key={a.id} action={a} accentColor={accentColor} dateColor={dateColor} bold={bold} />
      ))}
    </div>
  );
}

/* ── Main component ── */

export function TheWork({
  data,
  intelligence,
  sectionId = "the-work",
  chapterTitle = "The Work",
  addingAction,
  setAddingAction,
  newActionTitle,
  setNewActionTitle,
  creatingAction,
  onCreateAction,
}: TheWorkProps) {
  const readiness = intelligence?.nextMeetingReadiness;
  const now = new Date();

  const overdue = data.openActions.filter((a) => classifyAction(a, now) === "overdue");
  const thisWeek = data.openActions.filter((a) => classifyAction(a, now) === "this-week");
  const upcoming = data.openActions.filter((a) => classifyAction(a, now) === "upcoming");
  const noDue = data.openActions.filter((a) => classifyAction(a, now) === "no-date");

  const upcomingMeetings = data.upcomingMeetings ?? [];

  return (
    <section id={sectionId} style={{ scrollMarginTop: 60, paddingTop: 80 }}>
      <ChapterHeading title={chapterTitle} />

      {/* Readiness Callout */}
      {readiness && readiness.prepItems.length > 0 && (
        <div
          style={{
            background: "var(--color-paper-linen)",
            borderLeft: "3px solid var(--color-spice-turmeric)",
            borderRadius: "0 8px 8px 0",
            padding: "24px 28px",
            marginBottom: 48,
          }}
        >
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              fontWeight: 500,
              textTransform: "uppercase",
              letterSpacing: "0.1em",
              color: "var(--color-spice-turmeric)",
              marginBottom: 10,
            }}
          >
            Next Meeting
            {readiness.meetingTitle && (
              <span
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 10,
                  fontWeight: 500,
                  textTransform: "none",
                  letterSpacing: "normal",
                  color: "var(--color-text-primary)",
                  marginLeft: 8,
                }}
              >
                {readiness.meetingTitle}
              </span>
            )}
            {readiness.meetingDate && (
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  fontWeight: 500,
                  textTransform: "uppercase",
                  letterSpacing: "0.1em",
                  color: "var(--color-text-tertiary)",
                  marginLeft: 8,
                }}
              >
                {formatShortDate(readiness.meetingDate)}
              </span>
            )}
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            {readiness.prepItems.map((item, i) => (
              <p
                key={i}
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 15,
                  lineHeight: 1.65,
                  color: "var(--color-text-primary)",
                  margin: 0,
                }}
              >
                {item}
              </p>
            ))}
          </div>
        </div>
      )}

      {/* Upcoming Meetings */}
      {upcomingMeetings.length > 0 && (
        <div>
          <div style={sectionLabelStyle}>Upcoming Meetings</div>
          <div>
            {upcomingMeetings.map((m) => (
              <div
                key={m.id}
                style={{
                  display: "grid",
                  gridTemplateColumns: "90px 1fr auto",
                  gap: 16,
                  padding: "14px 0",
                  borderBottom: "1px solid var(--color-rule-light)",
                  alignItems: "baseline",
                }}
              >
                <span
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 12,
                    fontWeight: 500,
                    color: "var(--color-text-primary)",
                    whiteSpace: "nowrap",
                  }}
                >
                  {formatMeetingRowDate(m.startTime)}
                </span>
                <span
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 14,
                    fontWeight: 400,
                    color: "var(--color-text-primary)",
                  }}
                >
                  {m.title}
                </span>
                <span style={{ display: "flex", gap: 8, alignItems: "baseline" }}>
                  <span style={meetingTypeBadgeStyle(m.meetingType)}>
                    {formatMeetingType(m.meetingType)}
                  </span>
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Spacer between meetings and commitments */}
      <div style={{ height: 48 }} />

      {/* Commitments (Open Actions) */}
      <div>
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            fontWeight: 500,
            textTransform: "uppercase",
            letterSpacing: "0.1em",
            color: "var(--color-text-primary)",
            marginBottom: 24,
          }}
        >
          Commitments
        </div>

        {data.openActions.length > 0 ? (
          <div>
            <ActionGroup
              label="Overdue"
              labelColor="var(--color-spice-terracotta)"
              actions={overdue}
              accentColor="var(--color-spice-terracotta)"
              dateColor="var(--color-spice-terracotta)"
              bold
            />
            <ActionGroup
              label="This Week"
              labelColor="var(--color-spice-turmeric)"
              actions={thisWeek}
              accentColor="var(--color-spice-turmeric)"
            />
            <ActionGroup
              label="Upcoming"
              labelColor="var(--color-text-tertiary)"
              actions={[...upcoming, ...noDue]}
            />
          </div>
        ) : (
          <p
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              color: "var(--color-text-tertiary)",
              fontStyle: "italic",
            }}
          >
            No open actions.
          </p>
        )}

        {/* Inline action creation */}
        {setAddingAction && onCreateAction && (
          <div style={{ marginTop: 12 }}>
            {addingAction ? (
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                <input
                  value={newActionTitle ?? ""}
                  onChange={(e) => setNewActionTitle?.(e.target.value)}
                  placeholder="New action..."
                  autoFocus
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && (newActionTitle ?? "").trim()) onCreateAction();
                    if (e.key === "Escape") setAddingAction(false);
                  }}
                  style={{
                    flex: 1,
                    fontFamily: "var(--font-sans)",
                    fontSize: 14,
                    color: "var(--color-text-primary)",
                    background: "none",
                    border: "none",
                    borderBottom: "1px solid var(--color-rule-light)",
                    outline: "none",
                    padding: "4px 0",
                  }}
                />
                <button
                  onClick={onCreateAction}
                  disabled={creatingAction || !(newActionTitle ?? "").trim()}
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 10,
                    color: "var(--color-text-tertiary)",
                    background: "none",
                    border: "none",
                    cursor: "pointer",
                    textTransform: "uppercase",
                    letterSpacing: "0.06em",
                    padding: 0,
                  }}
                >
                  {creatingAction ? "..." : "Add"}
                </button>
                <button
                  onClick={() => setAddingAction(false)}
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 10,
                    color: "var(--color-text-tertiary)",
                    background: "none",
                    border: "none",
                    cursor: "pointer",
                    padding: 0,
                  }}
                >
                  x
                </button>
              </div>
            ) : (
              <button
                onClick={() => setAddingAction(true)}
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  color: "var(--color-text-tertiary)",
                  background: "none",
                  border: "none",
                  cursor: "pointer",
                  padding: "4px 0",
                  textTransform: "uppercase",
                  letterSpacing: "0.06em",
                }}
              >
                + Add Action
              </button>
            )}
          </div>
        )}
      </div>
    </section>
  );
}
