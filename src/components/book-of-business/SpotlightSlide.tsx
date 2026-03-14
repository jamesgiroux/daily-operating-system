import { EditableText } from "@/components/ui/EditableText";
import { formatArr } from "@/lib/utils";
import type { AccountDeepDive, BookOfBusinessContent } from "@/types/reports";
import s from "./BookOfBusinessSlides.module.css";

interface SpotlightSlideProps {
  dive: AccountDeepDive;
  index: number;
  total: number;
  content: BookOfBusinessContent;
  onUpdate: (content: BookOfBusinessContent) => void;
}

export function SpotlightSlide({ dive, index, total, content, onUpdate }: SpotlightSlideProps) {
  const diveIndex = content.deepDives.findIndex((item) => item.accountId === dive.accountId);
  if (diveIndex < 0) return null;

  const updateDive = (nextDive: AccountDeepDive) => {
    const next = [...content.deepDives];
    next[diveIndex] = nextDive;
    onUpdate({ ...content, deepDives: next });
  };

  const addWorkstream = () => updateDive({
    ...dive,
    activeWorkstreams: [...dive.activeWorkstreams, "Add an active workstream."],
  });

  const addRisk = () => updateDive({
    ...dive,
    risksAndGaps: [...dive.risksAndGaps, "Add a portfolio gap or risk."],
  });

  return (
    <section id={`spotlight-${index}`} className={s.slide}>
      <div className={s.sectionHeader}>
        <div className={s.overline}>Account Spotlight {index} of {total}</div>
        <div className={s.sectionActions}>
          <button
            type="button"
            className={`${s.button} ${s.buttonDanger}`}
            onClick={() => onUpdate({
              ...content,
              deepDives: content.deepDives.filter((_, itemIndex) => itemIndex !== diveIndex),
            })}
          >
            Remove Spotlight
          </button>
        </div>
      </div>

      <h2 className={s.title}>{dive.accountName}</h2>
      <div className={s.itemMeta}>
        {dive.arr != null && <span>${formatArr(dive.arr)}</span>}
        {dive.renewalDate && <span>Renewal {dive.renewalDate}</span>}
      </div>

      <EditableText
        as="p"
        value={dive.statusNarrative}
        onChange={(value) => updateDive({ ...dive, statusNarrative: value })}
        multiline
        className={s.heroSummary}
      />

      <div className={s.heroCallout}>
        <div className={s.sourceNote}>Revenue Impact</div>
        <EditableText
          value={dive.renewalOrGrowthImpact}
          onChange={(value) => updateDive({ ...dive, renewalOrGrowthImpact: value })}
          multiline={false}
          className={`${s.itemText} ${s.itemTextStrong}`}
        />
      </div>

      <div className={s.detailColumns}>
        <div className={s.column}>
          <div className={s.sectionHeader}>
            <div className={s.columnLabel}>Active Workstreams</div>
            <button type="button" className={`${s.button} ${s.buttonPrimary}`} onClick={addWorkstream}>
              Add
            </button>
          </div>
          {dive.activeWorkstreams.length === 0 ? (
            <p className={s.emptyMessage}>No workstreams captured yet.</p>
          ) : (
            <div className={s.detailList}>
              {dive.activeWorkstreams.map((item, itemIndex) => (
                <div key={`${dive.accountId}-workstream-${itemIndex}`} className={s.bulletRow}>
                  <span className={s.bullet} />
                  <div className={s.itemBody}>
                    <EditableText
                      value={item}
                      onChange={(value) => {
                        const nextItems = [...dive.activeWorkstreams];
                        nextItems[itemIndex] = value;
                        updateDive({ ...dive, activeWorkstreams: nextItems });
                      }}
                      multiline={false}
                      className={s.itemText}
                    />
                  </div>
                  <button
                    type="button"
                    className={`${s.button} ${s.buttonDanger}`}
                    onClick={() => updateDive({
                      ...dive,
                      activeWorkstreams: dive.activeWorkstreams.filter((_, idx) => idx !== itemIndex),
                    })}
                  >
                    Remove
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className={s.column}>
          <div className={s.sectionHeader}>
            <div className={s.columnLabel}>Risks & Gaps</div>
            <button type="button" className={`${s.button} ${s.buttonPrimary}`} onClick={addRisk}>
              Add
            </button>
          </div>
          {dive.risksAndGaps.length === 0 ? (
            <p className={s.emptyMessage}>No portfolio gaps captured yet.</p>
          ) : (
            <div className={s.detailList}>
              {dive.risksAndGaps.map((item, itemIndex) => (
                <div key={`${dive.accountId}-risk-${itemIndex}`} className={s.bulletRow}>
                  <span className={`${s.bullet} ${s.bulletRisk}`} />
                  <div className={s.itemBody}>
                    <EditableText
                      value={item}
                      onChange={(value) => {
                        const nextItems = [...dive.risksAndGaps];
                        nextItems[itemIndex] = value;
                        updateDive({ ...dive, risksAndGaps: nextItems });
                      }}
                      multiline={false}
                      className={s.itemText}
                    />
                  </div>
                  <button
                    type="button"
                    className={`${s.button} ${s.buttonDanger}`}
                    onClick={() => updateDive({
                      ...dive,
                      risksAndGaps: dive.risksAndGaps.filter((_, idx) => idx !== itemIndex),
                    })}
                  >
                    Remove
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </section>
  );
}
