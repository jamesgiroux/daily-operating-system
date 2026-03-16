import { ArrowRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import styles from "../onboarding.module.css";

interface MeetingDeepDiveProps {
  onNext: () => void;
}

export function MeetingDeepDive({ onNext }: MeetingDeepDiveProps) {
  return (
    <div className={`${styles.flexCol} ${styles.gap24}`}>
      <ChapterHeading
        title="This is what prepared looks like"
        epigraph="Every meeting gets this automatically. History, context, risks, talking points — compiled from your data, past meetings, and AI analysis."
      />

      {/* Mock expanded meeting card */}
      <div className={styles.ruleSectionAccent}>
        {/* Meeting header */}
        <div className={styles.meetingHeader}>
          <div className={`${styles.flexRow} ${styles.gap8}`}>
            <span className={styles.monoTimestamp}>
              10:30 AM
            </span>
            <span className={styles.dashSeparator}>—</span>
            <span className={styles.monoTimestampFaded}>
              11:30 AM
            </span>
          </div>
          <h3 className={styles.serifTitle}>
            Acme Corp Quarterly Sync
          </h3>
          <span className={styles.accentEntityLabel}>Acme Corp</span>
        </div>

        {/* Prep content */}
        <div className={`${styles.flexCol} ${styles.gap24}`}>
          {/* Quick Context */}
          <div className={styles.sectionWithRule}>
            <div className={styles.sectionLabel}>At a Glance</div>
            <div className={styles.flexWrap}>
              <span className={styles.chip}>Enterprise</span>
              <span className={styles.chip}>$1.2M ARR</span>
              <span className={styles.chipSage}>Health: Green</span>
              <span className={styles.chip}>Ring 1</span>
            </div>
          </div>

          {/* Attendees */}
          <div className={styles.sectionWithRule}>
            <div className={styles.sectionLabel}>Attendees</div>
            <div className={styles.attendeesBlock}>
              <p className={styles.attendeeText}>
                <span className={styles.attendeeName}>Sarah Chen</span> — VP Engineering{" "}
                <span className={styles.attendeeRole}>(Decision-maker for expansion)</span>
              </p>
              <p className={styles.attendeeText}>
                <span className={styles.attendeeName}>Marcus Rivera</span> — Director, Platform{" "}
                <span className={styles.attendeeRole}>(Day-to-day contact)</span>
              </p>
            </div>
          </div>

          <div className={styles.twoColGrid}>
            {/* Since Last Meeting */}
            <div className={styles.sectionWithRule}>
              <div className={styles.sectionLabel}>Since Last Meeting</div>
              <ul className={styles.plainList}>
                {["Phase 1 migration completed ahead of schedule", "NPS survey deployed — 3 detractors identified", "SOW for Phase 2 sent to legal"].map((item) => (
                  <li key={item} className={styles.listItem}>
                    <span className={styles.bulletDot} />
                    {item}
                  </li>
                ))}
              </ul>
            </div>

            {/* Talking Points */}
            <div className={styles.sectionWithRule}>
              <div className={styles.sectionLabel}><span className={styles.accentColor}>Talking Points</span></div>
              <ul className={styles.plainList}>
                {["Celebrate Phase 1 completion — set up case study", "Address NPS detractor concerns", "Phase 2 timeline and resource needs"].map((item) => (
                  <li key={item} className={styles.listItem}>
                    <span className={styles.bulletDotAccent} />
                    {item}
                  </li>
                ))}
              </ul>
            </div>

            {/* Risks */}
            <div className={styles.sectionWithRule}>
              <div className={styles.sectionLabel}><span className={styles.dangerColor}>Risks</span></div>
              <ul className={styles.plainList}>
                {["Key engineer leaving in March — knowledge transfer at risk", "NPS trending down — 3 detractors need follow-up"].map((item) => (
                  <li key={item} className={styles.listItem}>
                    <span className={styles.bulletDotDanger} />
                    {item}
                  </li>
                ))}
              </ul>
            </div>

            {/* Open Items */}
            <div className={styles.sectionWithRule}>
              <div className={styles.sectionLabel}>Open Items</div>
              <ul className={styles.plainList}>
                <li className={styles.listItem}>
                  <span className={styles.overdueBadge}>
                    OVERDUE
                  </span>
                  Send updated SOW to legal team
                </li>
                <li className={styles.listItem}>
                  <span className={styles.bulletDotMuted} />
                  Follow up on NPS survey responses
                </li>
              </ul>
            </div>
          </div>
        </div>
      </div>

      {/* Post-meeting teaser */}
      <div className={styles.postMeetingNote}>
        <span className={styles.accentText}>After a meeting:</span>{" "}
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
