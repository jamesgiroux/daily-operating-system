import { EditableText } from "@/components/ui/EditableText";
import { formatArr } from "@/lib/utils";
import type { AccountSnapshotRow, BookOfBusinessContent, BookOpportunityItem, BookRiskItem } from "@/types/reports";
import s from "./BookOfBusinessSlides.module.css";

interface AttentionSlideProps {
  content: BookOfBusinessContent;
  onUpdate: (content: BookOfBusinessContent) => void;
}

function nextAccount(snapshot: AccountSnapshotRow[], usedNames: string[]): AccountSnapshotRow | null {
  return snapshot.find((account) => !usedNames.includes(account.accountName)) ?? snapshot[0] ?? null;
}

export function AttentionSlide({ content, onUpdate }: AttentionSlideProps) {
  const addRisk = () => {
    const account = nextAccount(content.accountSnapshot, content.topRisks.map((item) => item.accountName));
    if (!account) return;
    onUpdate({
      ...content,
      topRisks: [
        ...content.topRisks,
        {
          accountName: account.accountName,
          arr: account.arr,
          risk: "Add the portfolio risk to call out.",
        },
      ],
    });
  };

  const addOpportunity = () => {
    const account = nextAccount(
      content.accountSnapshot,
      content.topOpportunities.map((item) => item.accountName),
    );
    if (!account) return;
    onUpdate({
      ...content,
      topOpportunities: [
        ...content.topOpportunities,
        {
          accountName: account.accountName,
          estimatedValue: account.arr != null ? `$${formatArr(account.arr)}` : null,
          opportunity: "Add the expansion or growth motion.",
        },
      ],
    });
  };

  return (
    <section id="attention" className={s.slideTight}>
      <div className={s.sectionHeader}>
        <div className={s.overline}>What Needs Attention</div>
        <div className={s.sectionActions}>
          <button type="button" className={`${s.button} ${s.buttonPrimary}`} onClick={addRisk}>
            Add Risk
          </button>
          <button type="button" className={`${s.button} ${s.buttonPrimary}`} onClick={addOpportunity}>
            Add Opportunity
          </button>
        </div>
      </div>

      <div className={s.twoColumn}>
        <div className={s.column}>
          <div className={`${s.columnLabel} ${s.columnLabelRisk}`}>Risks</div>
          {content.topRisks.length === 0 ? (
            <div className={s.emptyBlock}>
              <p className={s.emptyMessage}>No portfolio risks have been called out yet.</p>
            </div>
          ) : (
            content.topRisks.map((item, index) => (
              <RiskItem
                key={`${item.accountName}-${index}`}
                index={index + 1}
                item={item}
                onUpdate={(updated) => {
                  const next = [...content.topRisks];
                  next[index] = updated;
                  onUpdate({ ...content, topRisks: next });
                }}
                onRemove={() => onUpdate({
                  ...content,
                  topRisks: content.topRisks.filter((_, itemIndex) => itemIndex !== index),
                })}
              />
            ))
          )}
        </div>

        <div className={s.column}>
          <div className={`${s.columnLabel} ${s.columnLabelOpportunity}`}>Opportunities</div>
          {content.topOpportunities.length === 0 ? (
            <div className={s.emptyBlock}>
              <p className={s.emptyMessage}>No portfolio opportunities have been called out yet.</p>
            </div>
          ) : (
            content.topOpportunities.map((item, index) => (
              <OpportunityItem
                key={`${item.accountName}-${index}`}
                index={index + 1}
                item={item}
                onUpdate={(updated) => {
                  const next = [...content.topOpportunities];
                  next[index] = updated;
                  onUpdate({ ...content, topOpportunities: next });
                }}
                onRemove={() => onUpdate({
                  ...content,
                  topOpportunities: content.topOpportunities.filter((_, itemIndex) => itemIndex !== index),
                })}
              />
            ))
          )}
        </div>
      </div>
    </section>
  );
}

function RiskItem({
  index,
  item,
  onUpdate,
  onRemove,
}: {
  index: number;
  item: BookRiskItem;
  onUpdate: (item: BookRiskItem) => void;
  onRemove: () => void;
}) {
  return (
    <div className={s.itemRow}>
      <span className={`${s.itemIndex} ${s.itemIndexRisk}`}>{index}</span>
      <div className={s.itemBody}>
        <div className={s.itemMeta}>
          <span>{item.accountName}</span>
          {item.arr != null && <span>${formatArr(item.arr)}</span>}
        </div>
        <EditableText
          value={item.risk}
          onChange={(value) => onUpdate({ ...item, risk: value })}
          multiline={false}
          className={s.itemText}
        />
      </div>
      <div className={s.itemActions}>
        <button type="button" className={`${s.button} ${s.buttonDanger}`} onClick={onRemove}>
          Remove
        </button>
      </div>
    </div>
  );
}

function OpportunityItem({
  index,
  item,
  onUpdate,
  onRemove,
}: {
  index: number;
  item: BookOpportunityItem;
  onUpdate: (item: BookOpportunityItem) => void;
  onRemove: () => void;
}) {
  return (
    <div className={s.itemRow}>
      <span className={`${s.itemIndex} ${s.itemIndexOpportunity}`}>{index}</span>
      <div className={s.itemBody}>
        <div className={s.itemMeta}>
          <span>{item.accountName}</span>
          {item.estimatedValue && <span>{item.estimatedValue}</span>}
        </div>
        <EditableText
          value={item.opportunity}
          onChange={(value) => onUpdate({ ...item, opportunity: value })}
          multiline={false}
          className={s.itemText}
        />
      </div>
      <div className={s.itemActions}>
        <button type="button" className={`${s.button} ${s.buttonDanger}`} onClick={onRemove}>
          Remove
        </button>
      </div>
    </div>
  );
}
