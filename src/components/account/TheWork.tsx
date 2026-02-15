/**
 * TheWork — Chapter 6: Meeting readiness, upcoming meetings, and commitments.
 */
import { Link } from "@tanstack/react-router";
import type { AccountDetail, EntityIntelligence } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { parseDate, formatShortDate, formatMeetingType } from "@/lib/utils";

interface TheWorkProps {
  detail: AccountDetail;
  intelligence: EntityIntelligence | null;
  addingAction?: boolean;
  setAddingAction?: (v: boolean) => void;
  newActionTitle?: string;
  setNewActionTitle?: (v: string) => void;
  creatingAction?: boolean;
  onCreateAction?: () => void;
}

/** Format a date string as "Feb 18 Tue". */
function formatMeetingRowDate(dateStr: string): string {
  const date = parseDate(dateStr);
  if (!date) return dateStr;
  const month = date.toLocaleDateString(undefined, { month: "short" });
  const day = date.getDate();
  const weekday = date.toLocaleDateString(undefined, { weekday: "short" });
  return `${month} ${day} ${weekday}`;
}

/** Return a badge style for a meeting type. */
function meetingTypeBadgeStyle(meetingType: string): React.CSSProperties {
  const base: React.CSSProperties = {
    fontFamily: "var(--font-mono)",
    fontSize: 9,
    fontWeight: 500,
    textTransform: "uppercase",
    letterSpacing: "0.06em",
    padding: "2px 7px",
    borderRadius: 3,
    whiteSpace: "nowrap",
  };

  if (meetingType === "customer" || meetingType === "qbr" || meetingType === "training") {
    return {
      ...base,
      background: "rgba(201,162,39,0.10)",
      color: "var(--color-spice-turmeric)",
    };
  }
  if (meetingType === "internal" || meetingType === "team_sync" || meetingType === "one_on_one") {
    return {
      ...base,
      background: "rgba(143,163,196,0.12)",
      color: "var(--color-garden-larkspur)",
    };
  }
  return {
    ...base,
    background: "rgba(30,37,48,0.06)",
    color: "var(--color-text-tertiary)",
  };
}

/** Classify an action as overdue, this-week, or upcoming based on due date. */
function classifyAction(
  action: { dueDate?: string },
  now: Date,
): "overdue" | "this-week" | "upcoming" | "no-date" {
  if (!action.dueDate) return "no-date";
  const due = parseDate(action.dueDate);
  if (!due) return "no-date";

  const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  if (due < startOfToday) return "overdue";

  const sevenDaysOut = new Date(startOfToday);
  sevenDaysOut.setDate(sevenDaysOut.getDate() + 7);
  if (due < sevenDaysOut) return "this-week";

  return "upcoming";
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

/* ── Extracted sub-components for action groups ── */

interface ActionRowProps {
  action: { id: string; title: string; dueDate?: string; source?: string };
  /** CSS color value for the left border bar. Omit for no bar. */
  accentColor?: string;
  /** CSS color value for the due date text. */
  dateColor?: string;
  /** Bold title for overdue emphasis. */
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

export function TheWork({
  detail,
  intelligence,
  addingAction,
  setAddingAction,
  newActionTitle,
  setNewActionTitle,
  creatingAction,
  onCreateAction,
}: TheWorkProps) {
  const readiness = intelligence?.nextMeetingReadiness;
  const now = new Date();

  // Classify actions into urgency groups
  const overdue = detail.openActions.filter((a) => classifyAction(a, now) === "overdue");
  const thisWeek = detail.openActions.filter((a) => classifyAction(a, now) === "this-week");
  const upcoming = detail.openActions.filter((a) => classifyAction(a, now) === "upcoming");
  const noDue = detail.openActions.filter((a) => classifyAction(a, now) === "no-date");

  return (
    <section id="the-work" style={{ scrollMarginTop: 60, paddingTop: 80 }}>
      <ChapterHeading title="The Work" />

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
          <p
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 15,
              lineHeight: 1.65,
              color: "var(--color-text-primary)",
              margin: 0,
            }}
          >
            {readiness.prepItems.join(" ")}
          </p>
        </div>
      )}

      {/* Spacer after readiness callout (only if callout rendered and meetings follow) */}
      {readiness &&
        readiness.prepItems.length > 0 &&
        detail.upcomingMeetings.length > 0 && (
          <div style={{ height: 0 }} />
        )}

      {/* Upcoming Meetings */}
      {detail.upcomingMeetings.length > 0 && (
        <div>
          <div style={sectionLabelStyle}>Upcoming Meetings</div>
          <div>
            {detail.upcomingMeetings.map((m) => (
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
                {/* Date column */}
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

                {/* Title column */}
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

                {/* Meta column */}
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

        {detail.openActions.length > 0 ? (
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
