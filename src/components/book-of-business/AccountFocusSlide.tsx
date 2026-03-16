/**
 * AccountFocusSlide — Top 5 account focus priorities.
 * Slide 11: ranked accounts with objectives, tactics, and success signals.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import { formatArr } from "@/lib/utils";
import type { BookOfBusinessContent, AccountFocus } from "@/types/reports";

interface AccountFocusSlideProps {
  content: BookOfBusinessContent;
  onUpdate: (content: BookOfBusinessContent) => void;
}

export function AccountFocusSlide({ content, onUpdate }: AccountFocusSlideProps) {
  const [hoveredAccount, setHoveredAccount] = useState<number | null>(null);
  const [hoveredBullet, setHoveredBullet] = useState<{ acct: number; list: string; idx: number } | null>(null);

  const updateAccount = (index: number, patch: Partial<AccountFocus>) => {
    const next = [...content.accountFocus];
    next[index] = { ...next[index], ...patch };
    onUpdate({ ...content, accountFocus: next });
  };

  const updateBulletList = (acctIndex: number, field: "keyTactics" | "successSignals", bulletIndex: number, value: string) => {
    const next = [...content.accountFocus];
    const list = [...next[acctIndex][field]];
    list[bulletIndex] = value;
    next[acctIndex] = { ...next[acctIndex], [field]: list };
    onUpdate({ ...content, accountFocus: next });
  };

  const removeBullet = (acctIndex: number, field: "keyTactics" | "successSignals", bulletIndex: number) => {
    const next = [...content.accountFocus];
    next[acctIndex] = { ...next[acctIndex], [field]: next[acctIndex][field].filter((_, j) => j !== bulletIndex) };
    onUpdate({ ...content, accountFocus: next });
  };

  const addBullet = (acctIndex: number, field: "keyTactics" | "successSignals") => {
    const next = [...content.accountFocus];
    next[acctIndex] = { ...next[acctIndex], [field]: [...next[acctIndex][field], ""] };
    onUpdate({ ...content, accountFocus: next });
  };

  const removeAccount = (index: number) => {
    onUpdate({ ...content, accountFocus: content.accountFocus.filter((_, i) => i !== index) });
  };

  const addAccount = () => {
    onUpdate({
      ...content,
      accountFocus: [
        ...content.accountFocus,
        { rank: content.accountFocus.length + 1, accountName: "New Account", arr: 0, primaryObjective: "", keyTactics: [""], successSignals: [""] },
      ],
    });
  };

  const renderBulletList = (acctIndex: number, field: "keyTactics" | "successSignals", label: string, items: string[]) => (
    <div style={{ flex: 1 }}>
      <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--color-text-tertiary)", marginBottom: 10 }}>
        {label}
      </div>
      {items.map((item, bi) => (
        <div
          key={bi}
          onMouseEnter={() => setHoveredBullet({ acct: acctIndex, list: field, idx: bi })}
          onMouseLeave={() => setHoveredBullet(null)}
          style={{ display: "flex", alignItems: "baseline", gap: 10, paddingBottom: 6 }}
        >
          <span style={{ width: 4, height: 4, borderRadius: "50%", background: "var(--color-spice-turmeric)", flexShrink: 0, marginTop: 8 }} />
          <EditableText
            value={item}
            onChange={(v) => updateBulletList(acctIndex, field, bi, v)}
            multiline={false}
            placeholder="Add detail..."
            style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)", flex: 1 }}
          />
          {items.length > 1 && (
            <button
              onClick={(e) => { e.stopPropagation(); removeBullet(acctIndex, field, bi); }}
              style={{ opacity: hoveredBullet?.acct === acctIndex && hoveredBullet?.list === field && hoveredBullet?.idx === bi ? 0.6 : 0, transition: "opacity 0.15s", background: "none", border: "none", cursor: "pointer", padding: "2px 4px", fontSize: 12, color: "var(--color-text-tertiary)", flexShrink: 0 }}
              aria-label="Remove"
            >
              ✕
            </button>
          )}
        </div>
      ))}
      <button
        onClick={() => addBullet(acctIndex, field)}
        style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.06em", color: "var(--color-spice-turmeric)", background: "none", border: "none", cursor: "pointer", padding: "6px 0", textAlign: "left" }}
      >
        + Add
      </button>
    </div>
  );

  return (
    <section
      id="account-focus"
      style={{
        scrollMarginTop: 60,
        minHeight: "100vh",
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "120px 120px 80px",
        scrollSnapAlign: "start",
      }}
    >
      <div style={{ fontFamily: "var(--font-mono)", fontSize: 12, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.12em", color: "var(--color-spice-turmeric)", marginBottom: 32 }}>
        Top Account Focus
      </div>

      {content.accountFocus.map((acct, ai) => (
        <div
          key={ai}
          onMouseEnter={() => setHoveredAccount(ai)}
          onMouseLeave={() => setHoveredAccount(null)}
          style={{ marginBottom: 40, paddingBottom: 40, borderBottom: ai < content.accountFocus.length - 1 ? "1px solid var(--color-rule-light)" : "none", maxWidth: 900 }}
        >
          {/* Header: rank + name + ARR + remove */}
          <div style={{ display: "flex", alignItems: "baseline", gap: 16, marginBottom: 12 }}>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: 24, fontWeight: 600, color: "var(--color-spice-turmeric)", minWidth: 28, flexShrink: 0 }}>
              {acct.rank}
            </span>
            <EditableText
              value={acct.accountName}
              onChange={(v) => updateAccount(ai, { accountName: v })}
              multiline={false}
              style={{ fontFamily: "var(--font-serif)", fontSize: 24, fontWeight: 400, color: "var(--color-text-primary)" }}
            />
            <span style={{ fontFamily: "var(--font-mono)", fontSize: 16, fontWeight: 600, color: "var(--color-spice-turmeric)" }}>
              ${formatArr(acct.arr)}
            </span>
            {content.accountFocus.length > 1 && (
              <button
                onClick={(e) => { e.stopPropagation(); removeAccount(ai); }}
                style={{ opacity: hoveredAccount === ai ? 0.6 : 0, transition: "opacity 0.15s", background: "none", border: "none", cursor: "pointer", padding: "4px 6px", fontSize: 14, color: "var(--color-text-tertiary)", flexShrink: 0, marginLeft: "auto" }}
                aria-label="Remove"
              >
                ✕
              </button>
            )}
          </div>

          {/* Primary objective */}
          <div style={{ borderLeft: "3px solid var(--color-spice-turmeric)", paddingLeft: 16, marginBottom: 20, marginLeft: 44 }}>
            <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--color-text-tertiary)", marginBottom: 6 }}>
              Primary Objective
            </div>
            <EditableText
              value={acct.primaryObjective}
              onChange={(v) => updateAccount(ai, { primaryObjective: v })}
              multiline={false}
              placeholder="Objective..."
              style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--color-text-primary)" }}
            />
          </div>

          {/* Bullet lists */}
          <div style={{ display: "flex", gap: 48, marginLeft: 44 }}>
            {renderBulletList(ai, "keyTactics", "Key Tactics", acct.keyTactics)}
            {renderBulletList(ai, "successSignals", "Success Signals", acct.successSignals)}
          </div>
        </div>
      ))}

      {content.accountFocus.length === 0 && (
        <div style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--color-text-tertiary)", padding: "20px 0" }}>
          No account focus items defined.
        </div>
      )}

      <button
        onClick={addAccount}
        style={{ fontFamily: "var(--font-mono)", fontSize: 11, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.06em", color: "var(--color-spice-turmeric)", background: "none", border: "none", cursor: "pointer", padding: "12px 0", textAlign: "left" }}
      >
        + Add Account
      </button>
    </section>
  );
}
