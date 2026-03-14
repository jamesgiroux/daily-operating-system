import { EditableText } from "@/components/ui/EditableText";
import type { BookOfBusinessContent } from "@/types/reports";
import s from "./BookOfBusinessSlides.module.css";

interface AskSlideProps {
  content: BookOfBusinessContent;
  onUpdate: (content: BookOfBusinessContent) => void;
}

export function AskSlide({ content, onUpdate }: AskSlideProps) {
  const addAsk = () =>
    onUpdate({
      ...content,
      leadershipAsks: [
        ...content.leadershipAsks,
        {
          ask: "Add the decision or support needed.",
          context: "Explain why this matters now.",
          impactedAccounts: [],
          status: "new",
        },
      ],
      hasLeadershipAsks: true,
    });

  return (
    <section id="the-ask" className={s.slideTight}>
      <div className={s.sectionHeader}>
        <div className={s.overline}>Leadership Asks</div>
        <button type="button" className={`${s.button} ${s.buttonPrimary}`} onClick={addAsk}>
          Add Ask
        </button>
      </div>

      {content.leadershipAsks.length === 0 ? (
        <div className={s.emptyBlock}>
          <p className={s.emptyMessage}>No leadership asks are captured yet.</p>
        </div>
      ) : (
        <div className={s.sectionBlock}>
          {content.leadershipAsks.map((ask, index) => (
            <div key={`${ask.ask}-${index}`} className={s.itemRow}>
              <span className={s.itemIndex}>{index + 1}</span>
              <div className={s.itemBody}>
                <EditableText
                  value={ask.ask}
                  onChange={(value) => {
                    const next = [...content.leadershipAsks];
                    next[index] = { ...next[index], ask: value };
                    onUpdate({ ...content, leadershipAsks: next });
                  }}
                  multiline={false}
                  className={`${s.itemText} ${s.itemTextStrong}`}
                />
                <EditableText
                  as="div"
                  value={ask.context}
                  onChange={(value) => {
                    const next = [...content.leadershipAsks];
                    next[index] = { ...next[index], context: value };
                    onUpdate({ ...content, leadershipAsks: next });
                  }}
                  multiline={false}
                  className={`${s.itemText} ${s.itemTextMuted}`}
                />
                {ask.impactedAccounts.length > 0 && (
                  <div className={s.themeCitations}>{ask.impactedAccounts.join(" · ")}</div>
                )}
              </div>
              <div className={s.itemActions}>
                {ask.status && <span className={s.statusBadge}>{ask.status}</span>}
                <button
                  type="button"
                  className={`${s.button} ${s.buttonDanger}`}
                  onClick={() => {
                    const next = content.leadershipAsks.filter((_, itemIndex) => itemIndex !== index);
                    onUpdate({
                      ...content,
                      leadershipAsks: next,
                      hasLeadershipAsks: next.length > 0,
                    });
                  }}
                >
                  Remove
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
