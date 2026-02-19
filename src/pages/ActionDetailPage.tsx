import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { useParams, Link, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { PriorityPicker } from "@/components/ui/priority-picker";
import { EntityPicker } from "@/components/ui/entity-picker";
import { Calendar } from "@/components/ui/calendar";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { formatFullDate } from "@/lib/utils";
import { classifyAction } from "@/lib/entity-utils";
import { Check, Circle } from "lucide-react";
import type { ActionDetail } from "@/types";

// =============================================================================
// Priority accent colors (matches meetingTypeBadgeStyle pattern)
// =============================================================================

const PRIORITY_STYLE: Record<string, { bg: string; text: string }> = {
  P1: { bg: "rgba(196,101,74,0.10)", text: "var(--color-spice-terracotta)" },
  P2: { bg: "rgba(201,162,39,0.10)", text: "var(--color-spice-turmeric)" },
  P3: { bg: "rgba(143,163,196,0.12)", text: "var(--color-garden-larkspur)" },
};

function priorityAccent(priority: string): string {
  return PRIORITY_STYLE[priority]?.text ?? "var(--color-text-tertiary)";
}

// =============================================================================
// Shared inline styles
// =============================================================================

const monoLabel: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 10,
  fontWeight: 500,
  textTransform: "uppercase",
  letterSpacing: "0.1em",
  color: "var(--color-text-tertiary)",
};

const refKey: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 10,
  fontWeight: 500,
  textTransform: "uppercase",
  letterSpacing: "0.06em",
  color: "var(--color-text-tertiary)",
  width: 100,
  flexShrink: 0,
  paddingTop: 3,
};

const refValue: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: 14,
  color: "var(--color-text-primary)",
  flex: 1,
  minWidth: 0,
};

// =============================================================================
// Main component
// =============================================================================

export default function ActionDetailPage() {
  const { actionId } = useParams({ strict: false }) as {
    actionId?: string;
  };
  const navigate = useNavigate();
  const [detail, setDetail] = useState<ActionDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [toggling, setToggling] = useState(false);
  const [saveStatus, setSaveStatus] = useState<"idle" | "saving" | "saved">("idle");
  const saveTimerRef = useRef<ReturnType<typeof setTimeout>>();

  // Register magazine shell
  const shellConfig = useMemo(
    () => ({
      folioLabel: detail?.title ? (detail.title.length > 30 ? detail.title.slice(0, 30) + "…" : detail.title) : "Action",
      atmosphereColor: "terracotta" as const,
      activePage: "actions" as const,
      backLink: { label: "Back", onClick: () => window.history.length > 1 ? window.history.back() : navigate({ to: "/actions", search: { search: undefined } }) },
      folioStatusText: saveStatus === "saving" ? "Saving…" : saveStatus === "saved" ? "✓ Saved" : undefined,
    }),
    [detail?.title, navigate, saveStatus],
  );
  useRegisterMagazineShell(shellConfig);

  const load = useCallback(async () => {
    if (!actionId) return;
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<ActionDetail>("get_action_detail", {
        actionId,
      });
      setDetail(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [actionId]);

  useEffect(() => {
    load();
  }, [load]);

  async function toggleStatus() {
    if (!detail) return;
    setToggling(true);
    try {
      if (detail.status === "completed") {
        await invoke("reopen_action", { id: detail.id });
      } else {
        await invoke("complete_action", { id: detail.id });
      }
      await load();
    } finally {
      setToggling(false);
    }
  }

  async function saveField(updates: Record<string, unknown>) {
    if (!detail) return;
    clearTimeout(saveTimerRef.current);
    setSaveStatus("saving");
    try {
      await invoke("update_action", {
        request: { id: detail.id, ...updates },
      });
      await load();
      setSaveStatus("saved");
      saveTimerRef.current = setTimeout(() => setSaveStatus("idle"), 1500);
    } catch {
      setSaveStatus("idle");
    }
  }

  // ── Loading state ──

  if (loading) {
    return (
      <div className="editorial-loading" style={{ padding: "120px 120px 80px", maxWidth: 680, margin: "0 auto" }}>
        <div style={{ height: 16, width: 96, marginBottom: 16, background: "var(--color-rule-light)", borderRadius: 2 }} />
        <div style={{ height: 40, width: 384, marginBottom: 8, background: "var(--color-rule-light)", borderRadius: 2 }} />
        <div style={{ height: 16, width: 192, marginBottom: 24, background: "var(--color-rule-light)", borderRadius: 2 }} />
        <div style={{ height: 1, width: "100%", background: "var(--color-rule-heavy)" }} />
        <div style={{ marginTop: 40, display: "flex", flexDirection: "column", gap: 24 }}>
          <div style={{ height: 80, width: "100%", background: "var(--color-rule-light)", borderRadius: 2 }} />
          <div style={{ height: 128, width: "100%", background: "var(--color-rule-light)", borderRadius: 2 }} />
        </div>
      </div>
    );
  }

  // ── Error state ──

  if (error || !detail) {
    return (
      <div style={{ padding: "120px 120px 80px", maxWidth: 680, margin: "0 auto", textAlign: "center" }}>
        <p style={{ fontFamily: "var(--font-serif)", fontSize: 24, color: "var(--color-text-primary)", marginBottom: 16 }}>
          Something went wrong
        </p>
        <p style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-secondary)", marginBottom: 24 }}>
          {error ?? "Action not found"}
        </p>
        <button
          onClick={load}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            fontWeight: 500,
            textTransform: "uppercase",
            letterSpacing: "0.06em",
            padding: "8px 20px",
            border: "1px solid var(--color-rule-light)",
            borderRadius: 4,
            background: "none",
            color: "var(--color-text-primary)",
            cursor: "pointer",
          }}
        >
          Try again
        </button>
      </div>
    );
  }

  const isCompleted = detail.status === "completed";
  const hasSource = detail.sourceId && detail.sourceMeetingTitle;
  const isAutoGenerated = detail.sourceType && detail.sourceType !== "manual";
  const accent = priorityAccent(detail.priority);
  const pStyle = PRIORITY_STYLE[detail.priority];

  // Due date urgency
  const dueUrgency = classifyAction(detail, new Date());
  const dueColor =
    dueUrgency === "overdue"
      ? "var(--color-spice-terracotta)"
      : dueUrgency === "this-week"
        ? "var(--color-spice-turmeric)"
        : undefined;

  return (
    <div style={{ padding: "120px 120px 80px", maxWidth: 680, margin: "0 auto" }}>

        {/* ── Title band ── */}
        <div style={{ display: "flex", alignItems: "flex-start", gap: 14, marginBottom: 8 }}>
          {/* Status toggle circle */}
          <button
            onClick={toggleStatus}
            disabled={toggling}
            style={{
              marginTop: 6,
              flexShrink: 0,
              background: "none",
              border: "none",
              cursor: toggling ? "wait" : "pointer",
              padding: 0,
              color: isCompleted ? "var(--color-text-tertiary)" : accent,
              opacity: toggling ? 0.5 : 1,
              transition: "color 0.15s, opacity 0.15s",
            }}
            title={isCompleted ? "Reopen action" : "Complete action"}
          >
            {isCompleted ? (
              <Check size={22} strokeWidth={2.5} />
            ) : (
              <Circle size={22} strokeWidth={1.5} />
            )}
          </button>

          {/* Editable title */}
          <div style={{ flex: 1, minWidth: 0 }}>
            <EditableText
              value={detail.title}
              onSave={(title) => saveField({ title })}
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 28,
                fontWeight: 400,
                lineHeight: 1.2,
                color: "var(--color-text-primary)",
                opacity: isCompleted ? 0.5 : 1,
                textDecoration: isCompleted ? "line-through" : "none",
              }}
            />
          </div>
        </div>

        {/* Priority + Status strip */}
        <div style={{
          display: "flex",
          alignItems: "center",
          gap: 10,
          marginBottom: 24,
          paddingLeft: 36, // align with title (past circle)
          flexWrap: "wrap",
        }}>
          {/* Priority pill */}
          {pStyle && (
            <span style={{
              fontFamily: "var(--font-mono)",
              fontSize: 9,
              fontWeight: 500,
              textTransform: "uppercase",
              letterSpacing: "0.06em",
              padding: "2px 7px",
              borderRadius: 3,
              background: pStyle.bg,
              color: pStyle.text,
              whiteSpace: "nowrap",
            }}>
              {detail.priority}
            </span>
          )}

          {/* Status text */}
          <span style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            fontWeight: 500,
            textTransform: "uppercase",
            letterSpacing: "0.06em",
            color: "var(--color-text-tertiary)",
          }}>
            {isCompleted ? "Completed" : "Open"}
          </span>

          {/* Waiting on */}
          {detail.waitingOn && (
            <span style={{
              fontFamily: "var(--font-mono)",
              fontSize: 9,
              fontWeight: 500,
              textTransform: "uppercase",
              letterSpacing: "0.06em",
              padding: "2px 7px",
              borderRadius: 3,
              background: "rgba(30,37,48,0.06)",
              color: "var(--color-text-tertiary)",
              whiteSpace: "nowrap",
            }}>
              Waiting: {detail.waitingOn}
            </span>
          )}

          {/* Source badge (meeting link or label) */}
          {hasSource && (
            <Link
              to="/meeting/$meetingId"
              params={{ meetingId: detail.sourceId! }}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 9,
                fontWeight: 500,
                textTransform: "uppercase",
                letterSpacing: "0.06em",
                padding: "2px 7px",
                borderRadius: 3,
                background: "rgba(30,37,48,0.06)",
                color: "var(--color-text-tertiary)",
                textDecoration: "none",
                whiteSpace: "nowrap",
              }}
            >
              From meeting
            </Link>
          )}
        </div>

        {/* PriorityPicker (click row to change) */}
        <div style={{ paddingLeft: 36, marginBottom: 24 }}>
          <PriorityPicker
            value={detail.priority}
            onChange={(p) => saveField({ priority: p })}
          />
        </div>

        {/* Separator */}
        <div style={{
          height: 1,
          background: "var(--color-rule-light)",
          marginBottom: 40,
        }} />

        {/* ── 3. Context section ── */}
        <div style={{ marginBottom: 40 }}>
          <div style={{ ...monoLabel, marginBottom: 12 }}>Context</div>
          <EditableTextarea
            value={detail.context ?? ""}
            onSave={(context) =>
              context
                ? saveField({ context })
                : saveField({ clearContext: true })
            }
            placeholder="Add context..."
          />
          {isAutoGenerated && detail.context && (
            <p style={{
              fontFamily: "var(--font-sans)",
              fontSize: 12,
              fontStyle: "italic",
              color: "var(--color-text-tertiary)",
              marginTop: 8,
            }}>
              Auto-generated — may be updated by next briefing
            </p>
          )}
        </div>

        {/* ── 4. Reference section ── */}
        <div style={{ marginBottom: 40 }}>
          <div style={{ ...monoLabel, marginBottom: 16 }}>Reference</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>

            {/* Account */}
            <div style={{ display: "flex", alignItems: "flex-start" }}>
              <span style={refKey}>Account</span>
              <div style={refValue}>
                {detail.accountId && detail.accountName ? (
                  <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                    <span style={{
                      width: 6,
                      height: 6,
                      borderRadius: "50%",
                      background: "var(--color-spice-turmeric)",
                      flexShrink: 0,
                    }} />
                    <Link
                      to="/accounts/$accountId"
                      params={{ accountId: detail.accountId }}
                      style={{
                        color: "var(--color-text-primary)",
                        textDecoration: "none",
                        borderBottom: "1px solid var(--color-rule-light)",
                      }}
                    >
                      {detail.accountName}
                    </Link>
                    <button
                      onClick={() => saveField({ clearAccount: true })}
                      style={{
                        background: "none",
                        border: "none",
                        cursor: "pointer",
                        fontFamily: "var(--font-sans)",
                        fontSize: 14,
                        color: "var(--color-text-tertiary)",
                        padding: "0 2px",
                        lineHeight: 1,
                      }}
                      title="Remove account"
                    >
                      ×
                    </button>
                  </span>
                ) : (
                  <EntityPicker
                    value={null}
                    onChange={(id) => {
                      if (id) saveField({ accountId: id });
                    }}
                    entityType="account"
                    placeholder="Link account"
                  />
                )}
              </div>
            </div>

            {/* Due */}
            <div style={{ display: "flex", alignItems: "flex-start" }}>
              <span style={refKey}>Due</span>
              <div style={{ ...refValue, color: dueColor ?? refValue.color }}>
                <EditableDate
                  value={detail.dueDate ?? ""}
                  onSave={(v) =>
                    v
                      ? saveField({ dueDate: v })
                      : saveField({ clearDueDate: true })
                  }
                  urgencyColor={dueColor}
                />
              </div>
            </div>

            {/* Created (read-only) */}
            <div style={{ display: "flex", alignItems: "flex-start" }}>
              <span style={refKey}>Created</span>
              <span style={refValue}>{formatFullDate(detail.createdAt)}</span>
            </div>

            {/* Completed (read-only, only when completed) */}
            {detail.completedAt && (
              <div style={{ display: "flex", alignItems: "flex-start" }}>
                <span style={refKey}>Completed</span>
                <span style={refValue}>{formatFullDate(detail.completedAt)}</span>
              </div>
            )}

            {/* Source — meeting link (if not already shown in strip) or editable label */}
            {hasSource ? (
              <div style={{ display: "flex", alignItems: "flex-start" }}>
                <span style={refKey}>Source</span>
                <div style={refValue}>
                  <Link
                    to="/meeting/$meetingId"
                    params={{ meetingId: detail.sourceId! }}
                    style={{
                      color: "var(--color-text-primary)",
                      textDecoration: "none",
                      borderBottom: "1px solid var(--color-rule-light)",
                    }}
                  >
                    {detail.sourceMeetingTitle}
                  </Link>
                </div>
              </div>
            ) : (
              <div style={{ display: "flex", alignItems: "flex-start" }}>
                <span style={refKey}>Source</span>
                <div style={refValue}>
                  <EditableInline
                    value={detail.sourceLabel ?? ""}
                    onSave={(v) =>
                      v
                        ? saveField({ sourceLabel: v })
                        : saveField({ clearSourceLabel: true })
                    }
                    placeholder="Add source"
                  />
                </div>
              </div>
            )}
          </div>
        </div>

        {/* ── 5. Action bar ── */}
        <div style={{ display: "flex", justifyContent: "flex-end", alignItems: "center", gap: 12 }}>
          {saveStatus !== "idle" && (
            <span style={{
              fontFamily: "var(--font-mono)",
              fontSize: 9,
              fontWeight: 500,
              textTransform: "uppercase",
              letterSpacing: "0.06em",
              color: "var(--color-text-tertiary)",
              opacity: saveStatus === "saved" ? 1 : 0.6,
              transition: "opacity 0.3s",
            }}>
              {saveStatus === "saving" ? "Saving…" : "Saved"}
            </span>
          )}
          <button
            onClick={toggleStatus}
            disabled={toggling}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 500,
              textTransform: "uppercase",
              letterSpacing: "0.06em",
              padding: "8px 20px",
              border: "1px solid var(--color-rule-light)",
              borderRadius: 4,
              background: "none",
              color: "var(--color-text-primary)",
              cursor: toggling ? "wait" : "pointer",
              opacity: toggling ? 0.5 : 1,
              transition: "opacity 0.15s",
            }}
          >
            {isCompleted ? "Reopen" : "Mark Complete"}
          </button>
        </div>
    </div>
  );
}

// =============================================================================
// Inline Editable Components (editorial styling)
// =============================================================================

/** Click-to-edit single-line text (for title). */
function EditableText({
  value,
  onSave,
  style: baseStyle,
}: {
  value: string;
  onSave: (v: string) => void;
  style?: React.CSSProperties;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setDraft(value);
  }, [value]);

  useEffect(() => {
    if (editing) inputRef.current?.select();
  }, [editing]);

  function commit() {
    setEditing(false);
    if (draft.trim() && draft.trim() !== value) {
      onSave(draft.trim());
    } else {
      setDraft(value);
    }
  }

  if (editing) {
    return (
      <input
        ref={inputRef}
        type="text"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") {
            setDraft(value);
            setEditing(false);
          }
        }}
        style={{
          ...baseStyle,
          width: "100%",
          background: "none",
          border: "none",
          borderBottom: "1px solid var(--color-rule-light)",
          outline: "none",
          padding: 0,
        }}
      />
    );
  }

  return (
    <span
      onClick={() => setEditing(true)}
      style={{ ...baseStyle, cursor: "pointer" }}
    >
      {value || (
        <span style={{ color: "var(--color-text-tertiary)", fontStyle: "italic" }}>
          Click to edit
        </span>
      )}
    </span>
  );
}

/** Click-to-edit short inline text. */
function EditableInline({
  value,
  onSave,
  placeholder,
}: {
  value: string;
  onSave: (v: string) => void;
  placeholder?: string;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setDraft(value);
  }, [value]);

  useEffect(() => {
    if (editing) inputRef.current?.focus();
  }, [editing]);

  function commit() {
    setEditing(false);
    if (draft.trim() !== value) {
      onSave(draft.trim());
    }
  }

  if (editing) {
    return (
      <input
        ref={inputRef}
        type="text"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") {
            setDraft(value);
            setEditing(false);
          }
        }}
        placeholder={placeholder}
        style={{
          fontFamily: "var(--font-sans)",
          fontSize: 14,
          color: "var(--color-text-primary)",
          background: "none",
          border: "none",
          borderBottom: "1px solid var(--color-rule-light)",
          outline: "none",
          padding: 0,
        }}
      />
    );
  }

  return (
    <span
      onClick={() => setEditing(true)}
      style={{ cursor: "pointer" }}
    >
      {value ? (
        <span>{value}</span>
      ) : (
        <span style={{ color: "var(--color-text-tertiary)", fontStyle: "italic" }}>
          {placeholder ?? "Add"}
        </span>
      )}
    </span>
  );
}

/** Click-to-edit multiline text. */
function EditableTextarea({
  value,
  onSave,
  placeholder,
}: {
  value: string;
  onSave: (v: string) => void;
  placeholder?: string;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    setDraft(value);
  }, [value]);

  useEffect(() => {
    if (editing) textareaRef.current?.focus();
  }, [editing]);

  function commit() {
    setEditing(false);
    if (draft.trim() !== value) {
      onSave(draft.trim());
    }
  }

  const proseStyle: React.CSSProperties = {
    fontFamily: "var(--font-sans)",
    fontSize: 15,
    lineHeight: 1.65,
    color: "var(--color-text-primary)",
    maxWidth: 620,
  };

  if (editing) {
    return (
      <textarea
        ref={textareaRef}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            setDraft(value);
            setEditing(false);
          }
        }}
        placeholder={placeholder}
        rows={4}
        style={{
          ...proseStyle,
          width: "100%",
          resize: "none",
          background: "none",
          border: "none",
          borderBottom: "1px solid var(--color-rule-light)",
          outline: "none",
          padding: 0,
        }}
      />
    );
  }

  return (
    <div
      onClick={() => setEditing(true)}
      style={{ ...proseStyle, cursor: "pointer" }}
    >
      {value ? (
        <p style={{ margin: 0, whiteSpace: "pre-line" }}>{value}</p>
      ) : (
        <p style={{ margin: 0, color: "var(--color-text-tertiary)", fontStyle: "italic" }}>
          {placeholder ?? "Click to add..."}
        </p>
      )}
    </div>
  );
}

/** Date picker using shadcn Popover + Calendar. */
function EditableDate({
  value,
  onSave,
  urgencyColor,
}: {
  value: string;
  onSave: (v: string) => void;
  urgencyColor?: string;
}) {
  const [open, setOpen] = useState(false);

  const dateValue = value ? value.split("T")[0] : "";
  // Parse to Date for Calendar's selected prop (noon to avoid timezone shift)
  const selected = dateValue ? new Date(dateValue + "T12:00:00") : undefined;

  function handleSelect(day: Date | undefined) {
    if (!day) return;
    const yyyy = day.getFullYear();
    const mm = String(day.getMonth() + 1).padStart(2, "0");
    const dd = String(day.getDate()).padStart(2, "0");
    onSave(`${yyyy}-${mm}-${dd}`);
    setOpen(false);
  }

  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <button
            type="button"
            style={{
              cursor: "pointer",
              color: urgencyColor,
              background: "none",
              border: "none",
              padding: 0,
              fontFamily: "var(--font-sans)",
              fontSize: 14,
            }}
          >
            {dateValue ? (
              <span>{formatFullDate(dateValue)}</span>
            ) : (
              <span style={{ color: "var(--color-text-tertiary)", fontStyle: "italic" }}>
                Add due date
              </span>
            )}
          </button>
        </PopoverTrigger>
        <PopoverContent align="start" className="w-auto p-0">
          <Calendar
            mode="single"
            selected={selected}
            onSelect={handleSelect}
            defaultMonth={selected}
          />
        </PopoverContent>
      </Popover>
      {dateValue && (
        <button
          onClick={() => onSave("")}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 9,
            textTransform: "uppercase",
            letterSpacing: "0.06em",
            color: "var(--color-text-tertiary)",
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: 0,
          }}
        >
          Clear
        </button>
      )}
    </span>
  );
}
