import { EditableText } from "@/components/ui/EditableText";
import type { BookOfBusinessContent } from "@/types/reports";
import s from "./BookOfBusinessSlides.module.css";

interface ValueThemesSlideProps {
  content: BookOfBusinessContent;
  onUpdate: (content: BookOfBusinessContent) => void;
}

export function ValueThemesSlide({ content, onUpdate }: ValueThemesSlideProps) {
  const addValueItem = () => {
    const account = content.accountSnapshot.find(
      (item) => !content.valueDelivered.some((value) => value.accountName === item.accountName),
    ) ?? content.accountSnapshot[0];
    if (!account) return;
    onUpdate({
      ...content,
      valueDelivered: [
        ...content.valueDelivered,
        {
          accountName: account.accountName,
          headlineOutcome: "Add the customer outcome delivered.",
          whyItMatters: "Add why it mattered to the account or business.",
          source: null,
        },
      ],
    });
  };

  const addTheme = () =>
    onUpdate({
      ...content,
      keyThemes: [
        ...content.keyThemes,
        {
          title: "Add a cross-book theme",
          narrative: "Describe the pattern across the portfolio.",
          citedAccounts: [],
        },
      ],
    });

  return (
    <section id="value-themes" className={s.slideTight}>
      <div className={s.sectionBlock}>
        <div className={s.sectionHeader}>
          <div className={s.overline}>Value Delivered</div>
          <button type="button" className={`${s.button} ${s.buttonPrimary}`} onClick={addValueItem}>
            Add Outcome
          </button>
        </div>

        {content.valueDelivered.length === 0 ? (
          <div className={s.emptyBlock}>
            <p className={s.emptyMessage}>No outcomes captured yet.</p>
          </div>
        ) : (
          content.valueDelivered.map((item, index) => (
            <div key={`${item.accountName}-${index}`} className={s.itemRow}>
              <div className={s.itemBody}>
                <div className={s.itemMeta}>
                  <span>{item.accountName}</span>
                  {item.source && <span>{item.source}</span>}
                </div>
                <EditableText
                  value={item.headlineOutcome}
                  onChange={(value) => {
                    const next = [...content.valueDelivered];
                    next[index] = { ...next[index], headlineOutcome: value };
                    onUpdate({ ...content, valueDelivered: next });
                  }}
                  multiline={false}
                  className={`${s.itemText} ${s.itemTextStrong}`}
                />
                <EditableText
                  as="div"
                  value={item.whyItMatters}
                  onChange={(value) => {
                    const next = [...content.valueDelivered];
                    next[index] = { ...next[index], whyItMatters: value };
                    onUpdate({ ...content, valueDelivered: next });
                  }}
                  multiline={false}
                  className={`${s.itemText} ${s.itemTextMuted}`}
                />
              </div>
              <div className={s.itemActions}>
                <button
                  type="button"
                  className={`${s.button} ${s.buttonDanger}`}
                  onClick={() => onUpdate({
                    ...content,
                    valueDelivered: content.valueDelivered.filter((_, itemIndex) => itemIndex !== index),
                  })}
                >
                  Remove
                </button>
              </div>
            </div>
          ))
        )}
      </div>

      <div className={s.sectionBlock}>
        <div className={s.sectionHeader}>
          <div className={s.overline}>Portfolio Themes</div>
          <button type="button" className={`${s.button} ${s.buttonPrimary}`} onClick={addTheme}>
            Add Theme
          </button>
        </div>

        {content.keyThemes.length === 0 ? (
          <div className={s.emptyBlock}>
            <p className={s.emptyMessage}>No cross-book themes have been written yet.</p>
          </div>
        ) : (
          content.keyThemes.map((theme, index) => (
            <div key={`${theme.title}-${index}`} className={s.themeBlock}>
              <div className={s.sectionHeader}>
                <EditableText
                  as="h2"
                  value={theme.title}
                  onChange={(value) => {
                    const next = [...content.keyThemes];
                    next[index] = { ...next[index], title: value };
                    onUpdate({ ...content, keyThemes: next });
                  }}
                  multiline={false}
                  className={s.themeTitle}
                />
                <button
                  type="button"
                  className={`${s.button} ${s.buttonDanger}`}
                  onClick={() => onUpdate({
                    ...content,
                    keyThemes: content.keyThemes.filter((_, itemIndex) => itemIndex !== index),
                  })}
                >
                  Remove
                </button>
              </div>
              <EditableText
                as="p"
                value={theme.narrative}
                onChange={(value) => {
                  const next = [...content.keyThemes];
                  next[index] = { ...next[index], narrative: value };
                  onUpdate({ ...content, keyThemes: next });
                }}
                multiline
                className={s.itemText}
              />
              {theme.citedAccounts.length > 0 && (
                <div className={s.themeCitations}>{theme.citedAccounts.join(" · ")}</div>
              )}
            </div>
          ))
        )}
      </div>
    </section>
  );
}
