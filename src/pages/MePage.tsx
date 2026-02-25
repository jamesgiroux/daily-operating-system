/**
 * MePage — User entity editorial page (/me).
 * Six-section layout: About Me, What I Deliver, My Priorities,
 * My Playbooks, Context Entries, Attachments.
 * ADR-0089/0090. Eucalyptus accent.
 */
import { useState, useMemo, useCallback, useRef } from "react";
import { User, Briefcase, Target, BookOpen, FileText, Paperclip, Upload } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { useNavigate } from "@tanstack/react-router";

import { useMe } from "@/hooks/useMe";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { formatShortDate } from "@/lib/utils";
import type { AnnualPriority, QuarterlyPriority } from "@/types";

import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { EditableText } from "@/components/ui/EditableText";
import { EditableList } from "@/components/ui/EditableList";
import { EntityPicker } from "@/components/ui/entity-picker";

import s from "./MePage.module.css";

// ─── Chapter definitions ──────────────────────────────────────────────

const CHAPTERS = [
  { id: "about-me", label: "About Me", icon: <User size={18} strokeWidth={1.5} /> },
  { id: "what-i-deliver", label: "What I Deliver", icon: <Briefcase size={18} strokeWidth={1.5} /> },
  { id: "my-priorities", label: "My Priorities", icon: <Target size={18} strokeWidth={1.5} /> },
  { id: "my-playbooks", label: "My Playbooks", icon: <BookOpen size={18} strokeWidth={1.5} /> },
  { id: "context-entries", label: "Context", icon: <FileText size={18} strokeWidth={1.5} /> },
  { id: "attachments", label: "Attachments", icon: <Paperclip size={18} strokeWidth={1.5} /> },
];

// ─── JSON helpers ─────────────────────────────────────────────────────

function parseJsonArray<T>(json: string | null): T[] {
  if (!json) return [];
  try {
    const parsed = JSON.parse(json);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function parsePlaybooks(json: string | null): Record<string, string> {
  if (!json) return {};
  try {
    const parsed = JSON.parse(json);
    return typeof parsed === "object" && parsed !== null ? parsed : {};
  } catch {
    return {};
  }
}

// ─── Priority sub-component ──────────────────────────────────────────

function PrioritySection({
  label,
  items,
  onSave,
}: {
  label: string;
  items: (AnnualPriority | QuarterlyPriority)[];
  onSave: (updated: (AnnualPriority | QuarterlyPriority)[]) => void;
}) {
  const [adding, setAdding] = useState(false);
  const [newText, setNewText] = useState("");

  const handleAdd = useCallback(() => {
    const text = newText.trim();
    if (!text) return;
    const newItem: AnnualPriority = {
      id: crypto.randomUUID(),
      text,
      linkedEntityId: null,
      linkedEntityType: null,
      createdAt: new Date().toISOString(),
    };
    onSave([...items, newItem]);
    setNewText("");
    setAdding(false);
  }, [newText, items, onSave]);

  const handleRemove = useCallback(
    (id: string) => {
      onSave(items.filter((i) => i.id !== id));
    },
    [items, onSave],
  );

  const handleEditText = useCallback(
    (id: string, text: string) => {
      onSave(items.map((i) => (i.id === id ? { ...i, text } : i)));
    },
    [items, onSave],
  );

  const handleLinkEntity = useCallback(
    (id: string, entityId: string | null, _name?: string, entityType?: "account" | "project") => {
      onSave(
        items.map((i) =>
          i.id === id
            ? { ...i, linkedEntityId: entityId, linkedEntityType: entityType ?? null }
            : i,
        ),
      );
    },
    [items, onSave],
  );

  return (
    <div>
      <h3 className={s.subsectionLabel}>{label}</h3>
      <div className={s.priorityList}>
        {items.map((item) => (
          <div key={item.id} className={s.priorityItem}>
            <div className={s.priorityText}>
              <EditableText
                value={item.text}
                onChange={(v) => handleEditText(item.id, v)}
                multiline={false}
                placeholder="Priority..."
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 14,
                  color: "var(--color-text-primary)",
                  lineHeight: 1.5,
                }}
              />
            </div>
            <div className={s.priorityEntityLink}>
              <EntityPicker
                value={item.linkedEntityId ?? null}
                onChange={(entityId, name, entityType) =>
                  handleLinkEntity(item.id, entityId, name, entityType)
                }
                entityType="all"
                placeholder="Link..."
              />
            </div>
            <button
              className={s.priorityDelete}
              onClick={() => handleRemove(item.id)}
              title="Remove"
            >
              &times;
            </button>
          </div>
        ))}
      </div>
      {adding ? (
        <div className={s.priorityAddForm}>
          <input
            className={s.priorityAddInput}
            value={newText}
            onChange={(e) => setNewText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleAdd();
              if (e.key === "Escape") {
                setNewText("");
                setAdding(false);
              }
            }}
            onBlur={() => {
              if (newText.trim()) handleAdd();
              else setAdding(false);
            }}
            autoFocus
            placeholder="What are you focused on?"
          />
        </div>
      ) : (
        <button className={s.priorityAdd} onClick={() => setAdding(true)}>
          + Add priority
        </button>
      )}
    </div>
  );
}

// ─── Context entry sub-component ─────────────────────────────────────

function ContextEntryList({
  entries,
  onUpdate,
  onDelete,
  onCreate,
}: {
  entries: { id: string; title: string; content: string; createdAt: string }[];
  onUpdate: (id: string, title: string, content: string) => void;
  onDelete: (id: string) => void;
  onCreate: (title: string, content: string) => void;
}) {
  const [adding, setAdding] = useState(false);
  const [newTitle, setNewTitle] = useState("");
  const [newContent, setNewContent] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editTitle, setEditTitle] = useState("");
  const [editContent, setEditContent] = useState("");

  const handleCreate = () => {
    if (!newTitle.trim() || !newContent.trim()) return;
    onCreate(newTitle.trim(), newContent.trim());
    setNewTitle("");
    setNewContent("");
    setAdding(false);
  };

  const startEdit = (entry: { id: string; title: string; content: string }) => {
    setEditingId(entry.id);
    setEditTitle(entry.title);
    setEditContent(entry.content);
  };

  const commitEdit = () => {
    if (editingId && editTitle.trim() && editContent.trim()) {
      onUpdate(editingId, editTitle.trim(), editContent.trim());
    }
    setEditingId(null);
  };

  return (
    <div>
      <div className={s.entryList}>
        {entries.map((entry) =>
          editingId === entry.id ? (
            <div key={entry.id} className={s.editEntryForm}>
              <input
                className={s.addEntryInput}
                value={editTitle}
                onChange={(e) => setEditTitle(e.target.value)}
                placeholder="Title"
                autoFocus
              />
              <textarea
                className={s.addEntryTextarea}
                value={editContent}
                onChange={(e) => setEditContent(e.target.value)}
                placeholder="Content"
                rows={3}
              />
              <div className={s.addEntryActions}>
                <button className={s.addEntryCancel} onClick={() => setEditingId(null)}>
                  Cancel
                </button>
                <button className={s.addEntrySave} onClick={commitEdit}>
                  Save
                </button>
              </div>
            </div>
          ) : (
            <div key={entry.id} className={s.entryItem}>
              <div className={s.entryHeader}>
                <span className={s.entryTitle}>{entry.title}</span>
                <span className={s.entryDate}>{formatShortDate(entry.createdAt)}</span>
              </div>
              <div className={s.entryContent}>{entry.content}</div>
              <div className={s.entryActions}>
                <button className={s.entryActionBtn} onClick={() => startEdit(entry)}>
                  Edit
                </button>
                <button className={s.entryActionBtn} onClick={() => onDelete(entry.id)}>
                  Delete
                </button>
              </div>
            </div>
          ),
        )}
      </div>

      {adding ? (
        <div className={s.addEntryForm}>
          <input
            className={s.addEntryInput}
            value={newTitle}
            onChange={(e) => setNewTitle(e.target.value)}
            placeholder="e.g., 'Infrastructure scaling philosophy' or 'Customer success metrics'"
            autoFocus
          />
          <textarea
            className={s.addEntryTextarea}
            value={newContent}
            onChange={(e) => setNewContent(e.target.value)}
            placeholder="Write 1–3 paragraphs about your approach, methodology, or key insight. This context will be retrieved during account/person enrichment when relevant."
            rows={3}
          />
          <div className={s.addEntryActions}>
            <button
              className={s.addEntryCancel}
              onClick={() => {
                setAdding(false);
                setNewTitle("");
                setNewContent("");
              }}
            >
              Cancel
            </button>
            <button className={s.addEntrySave} onClick={handleCreate}>
              Save
            </button>
          </div>
        </div>
      ) : (
        <button className={s.priorityAdd} onClick={() => setAdding(true)}>
          + Add context entry
        </button>
      )}
    </div>
  );
}

// ─── Main page component ─────────────────────────────────────────────

export default function MePage() {
  const me = useMe();
  const navigate = useNavigate();
  useRevealObserver(!me.loading && !!me.userEntity);

  const shellConfig = useMemo(
    () => ({
      folioLabel: "Me",
      atmosphereColor: "eucalyptus" as const,
      activePage: "me" as const,
      chapters: CHAPTERS,
      folioActions: (
        <div style={{ display: "flex", gap: 8 }}>
          <button
            onClick={() => navigate({ to: "/me/reports/$reportType", params: { reportType: "weekly_impact" } })}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              letterSpacing: "0.06em",
              textTransform: "uppercase" as const,
              color: "var(--color-garden-eucalyptus)",
              background: "none",
              border: "1px solid var(--color-garden-eucalyptus)",
              borderRadius: 4,
              padding: "2px 10px",
              cursor: "pointer",
            }}
          >
            Weekly Impact
          </button>
          <button
            onClick={() => navigate({ to: "/me/reports/$reportType", params: { reportType: "monthly_wrapped" } })}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              letterSpacing: "0.06em",
              textTransform: "uppercase" as const,
              color: "var(--color-garden-sage)",
              background: "none",
              border: "1px solid var(--color-garden-sage)",
              borderRadius: 4,
              padding: "2px 10px",
              cursor: "pointer",
            }}
          >
            Monthly Wrapped
          </button>
        </div>
      ),
    }),
    [navigate],
  );
  useRegisterMagazineShell(shellConfig);

  if (me.loading) return <EditorialLoading />;
  if (me.error && !me.userEntity) {
    return <EditorialError message={me.error} onRetry={me.load} />;
  }

  const entity = me.userEntity;
  if (!entity) return <EditorialLoading />;

  // Parse JSON fields
  const differentiators = parseJsonArray<string>(entity.differentiators);
  const objections = parseJsonArray<string>(entity.objections);
  const annualPriorities = parseJsonArray<AnnualPriority>(entity.annualPriorities);
  const quarterlyPriorities = parseJsonArray<QuarterlyPriority>(entity.quarterlyPriorities);
  const playbooks = parsePlaybooks(entity.playbooks);

  // Activity check: any non-null user-editable field
  const hasContent = !!(
    entity.name || entity.company || entity.title || entity.focus ||
    entity.valueProposition || entity.successDefinition ||
    entity.productContext || entity.companyBio || entity.roleDescription ||
    entity.howImMeasured || entity.pricingModel || entity.competitiveContext ||
    (entity.differentiators && parseJsonArray<string>(entity.differentiators).length > 0) ||
    (entity.objections && parseJsonArray<string>(entity.objections).length > 0) ||
    (entity.annualPriorities && parseJsonArray<AnnualPriority>(entity.annualPriorities).length > 0) ||
    (entity.quarterlyPriorities && parseJsonArray<QuarterlyPriority>(entity.quarterlyPriorities).length > 0) ||
    (entity.playbooks && entity.playbooks !== "{}")
  );

  return (
    <div className={s.page}>
      {/* ═══ HERO ═══ */}
      <section className={s.hero}>
        <h1 className={s.heroTitle}>
          {entity.name || "Your Profile"}
        </h1>
        {(entity.title || entity.company) && (
          <p className={s.heroSubtitle}>
            {[entity.title, entity.company].filter(Boolean).join(" at ")}
          </p>
        )}
        <hr className={s.heroRule} />
      </section>

      {/* Activity indicator */}
      {hasContent && (
        <div className={s.activityLine}>
          Your profile shapes every briefing, insight, and recommendation.
        </div>
      )}

      {/* ═══ SECTION 1: About Me ═══ */}
      <section id="about-me" className={s.section} style={{ scrollMarginTop: 60 }}>
        <ChapterHeading
          title="About Me"
          epigraph="Who you are and what you do."
        />

        <div className={s.fieldGrid}>
          <div className={s.fieldRow}>
            <div className={s.fieldLabel}>Name</div>
            <EditableText
              value={entity.name ?? ""}
              onChange={(v) => me.saveField("name", v)}
              multiline={false}
              placeholder="Your name"
              style={{ fontFamily: "var(--font-sans)", fontSize: 15, fontWeight: 400, color: "var(--color-text-primary)" }}
            />
          </div>
          <div className={s.fieldRow}>
            <div className={s.fieldLabel}>Title</div>
            <EditableText
              value={entity.title ?? ""}
              onChange={(v) => me.saveField("title", v)}
              multiline={false}
              placeholder="Your title"
              style={{ fontFamily: "var(--font-sans)", fontSize: 15, fontWeight: 400, color: "var(--color-text-primary)" }}
            />
          </div>
          <div className={s.fieldRow}>
            <div className={s.fieldLabel}>Company</div>
            <EditableText
              value={entity.company ?? ""}
              onChange={(v) => me.saveField("company", v)}
              multiline={false}
              placeholder="Your company"
              style={{ fontFamily: "var(--font-sans)", fontSize: 15, fontWeight: 400, color: "var(--color-text-primary)" }}
            />
          </div>
          <div className={s.fieldRow}>
            <div className={s.fieldLabel}>Current Focus</div>
            <EditableText
              value={entity.focus ?? ""}
              onChange={(v) => me.saveField("focus", v)}
              multiline={false}
              placeholder="What you're focused on"
              style={{ fontFamily: "var(--font-sans)", fontSize: 15, fontWeight: 400, color: "var(--color-text-primary)" }}
            />
          </div>
        </div>

        <hr className={s.thinRule} />

        <div className={s.fieldRow}>
          <div className={s.fieldLabel}>Company Bio</div>
          <EditableText
            value={entity.companyBio ?? ""}
            onChange={(v) => me.saveField("company_bio", v)}
            placeholder="Brief description of your company..."
            style={{ fontFamily: "var(--font-sans)", fontSize: 15, fontWeight: 300, color: "var(--color-text-primary)", lineHeight: 1.65 }}
          />
        </div>

        <div className={s.fieldRow}>
          <div className={s.fieldLabel}>Role Description</div>
          <EditableText
            value={entity.roleDescription ?? ""}
            onChange={(v) => me.saveField("role_description", v)}
            placeholder="What does your role involve?"
            style={{ fontFamily: "var(--font-sans)", fontSize: 15, fontWeight: 300, color: "var(--color-text-primary)", lineHeight: 1.65 }}
          />
        </div>

        <div className={s.fieldRow}>
          <div className={s.fieldLabel}>How I'm Measured</div>
          <EditableText
            value={entity.howImMeasured ?? ""}
            onChange={(v) => me.saveField("how_im_measured", v)}
            placeholder="KPIs, metrics, success criteria..."
            style={{ fontFamily: "var(--font-sans)", fontSize: 15, fontWeight: 300, color: "var(--color-text-primary)", lineHeight: 1.65 }}
          />
        </div>
      </section>

      {/* ═══ SECTION 2: What I Deliver ═══ */}
      <section id="what-i-deliver" className={`${s.section} editorial-reveal`} style={{ scrollMarginTop: 60 }}>
        <ChapterHeading
          title="What I Deliver"
          epigraph="Your value proposition, product context, and competitive landscape."
        />

        <div className={s.fieldRow}>
          <div className={s.fieldLabel}>Value Proposition</div>
          <EditableText
            value={entity.valueProposition ?? ""}
            onChange={(v) => me.saveField("value_proposition", v)}
            placeholder="What value do you deliver to customers?"
            style={{ fontFamily: "var(--font-sans)", fontSize: 15, fontWeight: 300, color: "var(--color-text-primary)", lineHeight: 1.65 }}
          />
        </div>

        <div className={s.fieldRow}>
          <div className={s.fieldLabel}>Success Definition</div>
          <EditableText
            value={entity.successDefinition ?? ""}
            onChange={(v) => me.saveField("success_definition", v)}
            placeholder="What does success look like for your customers?"
            style={{ fontFamily: "var(--font-sans)", fontSize: 15, fontWeight: 300, color: "var(--color-text-primary)", lineHeight: 1.65 }}
          />
        </div>

        <div className={s.fieldRow}>
          <div className={s.fieldLabel}>Product Context</div>
          <EditableText
            value={entity.productContext ?? ""}
            onChange={(v) => me.saveField("product_context", v)}
            placeholder="Product, platform, or service you support..."
            style={{ fontFamily: "var(--font-sans)", fontSize: 15, fontWeight: 300, color: "var(--color-text-primary)", lineHeight: 1.65 }}
          />
        </div>

        <div className={s.fieldRow}>
          <div className={s.fieldLabel}>Pricing Model</div>
          <EditableText
            value={entity.pricingModel ?? ""}
            onChange={(v) => me.saveField("pricing_model", v)}
            placeholder="How your product/service is priced..."
            style={{ fontFamily: "var(--font-sans)", fontSize: 15, fontWeight: 300, color: "var(--color-text-primary)", lineHeight: 1.65 }}
          />
        </div>

        <hr className={s.thinRule} />

        <div className={s.fieldRow}>
          <div className={s.fieldLabel}>Differentiators</div>
          <EditableList
            items={differentiators}
            onChange={(items) => me.saveField("differentiators", JSON.stringify(items))}
            placeholder="Add differentiator..."
            fieldId="differentiators"
          />
        </div>

        <hr className={s.thinRule} />

        <div className={s.fieldRow}>
          <div className={s.fieldLabel}>Objections</div>
          <EditableList
            items={objections}
            onChange={(items) => me.saveField("objections", JSON.stringify(items))}
            placeholder="Add common objection..."
            fieldId="objections"
          />
        </div>

        <hr className={s.thinRule} />

        <div className={s.fieldRow}>
          <div className={s.fieldLabel}>Competitive Context</div>
          <EditableText
            value={entity.competitiveContext ?? ""}
            onChange={(v) => me.saveField("competitive_context", v)}
            placeholder="Key competitors, positioning, win themes..."
            style={{ fontFamily: "var(--font-sans)", fontSize: 15, fontWeight: 300, color: "var(--color-text-primary)", lineHeight: 1.65 }}
          />
        </div>
      </section>

      {/* ═══ SECTION 3: My Priorities ═══ */}
      <section id="my-priorities" className={`${s.section} editorial-reveal`} style={{ scrollMarginTop: 60 }}>
        <ChapterHeading
          title="My Priorities"
          epigraph="Annual bets and quarterly focus areas. They persist until you remove them."
        />

        <PrioritySection
          label="This Year"
          items={annualPriorities}
          onSave={(updated) => me.saveField("annual_priorities", JSON.stringify(updated))}
        />

        <hr className={s.thinRule} />

        <PrioritySection
          label="This Quarter"
          items={quarterlyPriorities}
          onSave={(updated) => me.saveField("quarterly_priorities", JSON.stringify(updated))}
        />
      </section>

      {/* ═══ SECTION 4: My Playbooks ═══ */}
      <section id="my-playbooks" className={`${s.section} editorial-reveal`} style={{ scrollMarginTop: 60 }}>
        <ChapterHeading
          title="My Playbooks"
          epigraph="Your approaches and methodologies."
        />

        <PlaybooksSection
          playbooks={playbooks}
          onSave={(updated) => me.saveField("playbooks", JSON.stringify(updated))}
        />
      </section>

      {/* ═══ SECTION 5: Context Entries ═══ */}
      <section id="context-entries" className={`${s.section} editorial-reveal`} style={{ scrollMarginTop: 60 }}>
        <ChapterHeading
          title="Context"
          epigraph="Professional knowledge that enriches your briefings. Embedding happens in the background."
        />

        <ContextEntryList
          entries={me.contextEntries}
          onUpdate={(id, title, content) => me.updateEntry(id, title, content)}
          onDelete={(id) => me.deleteEntry(id)}
          onCreate={(title, content) => me.createEntry(title, content)}
        />
      </section>

      {/* ═══ SECTION 6: Attachments ═══ */}
      <section id="attachments" className={`${s.section} editorial-reveal`} style={{ scrollMarginTop: 60 }}>
        <ChapterHeading
          title="Attachments"
          epigraph="Documents that provide deeper context."
        />

        <AttachmentsSection />
      </section>

      {/* ═══ FINIS ═══ */}
      <div className="editorial-reveal">
        <FinisMarker />
      </div>
      <div style={{ height: 80 }} />
    </div>
  );
}

// ─── Attachments sub-component ────────────────────────────────────────

interface AttachmentFile {
  name: string;
  path: string;
}

function AttachmentsSection() {
  const [files, setFiles] = useState<AttachmentFile[]>([]);
  const [dragOver, setDragOver] = useState(false);
  const dropRef = useRef<HTMLDivElement>(null);

  const processFile = useCallback(async (filePath: string) => {
    try {
      const result = await invoke<string>("process_user_attachment", { path: filePath });
      const name = filePath.split(/[\\/]/).pop() || filePath;
      setFiles((prev) => [...prev, { name, path: result }]);
    } catch (err) {
      console.error("Failed to process attachment:", err);
    }
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setDragOver(false);
      const droppedFiles = e.dataTransfer.files;
      for (let i = 0; i < droppedFiles.length; i++) {
        const file = droppedFiles[i];
        // Tauri file drops expose the path via the webkitRelativePath or name
        // but for security, Tauri intercepts and provides full paths via its own
        // drop handler. We use the file name as a fallback display.
        if ((file as unknown as { path?: string }).path) {
          processFile((file as unknown as { path: string }).path);
        }
      }
    },
    [processFile],
  );

  const handleBrowse = useCallback(async () => {
    try {
      const selected = await openFileDialog({
        multiple: true,
        filters: [
          { name: "Documents", extensions: ["pdf", "md", "txt", "csv", "docx", "json"] },
        ],
      });
      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        for (const filePath of paths) {
          if (filePath) processFile(filePath);
        }
      }
    } catch {
      // user cancelled
    }
  }, [processFile]);

  return (
    <div>
      <div
        ref={dropRef}
        className={`${s.dropzone} ${dragOver ? s.dropzoneActive : ""}`}
        onDragOver={(e) => {
          e.preventDefault();
          setDragOver(true);
        }}
        onDragLeave={() => setDragOver(false)}
        onDrop={handleDrop}
      >
        <Upload size={20} className={s.dropzoneIcon} />
        <span className={s.dropzoneText}>
          Drop files here or{" "}
          <button className={s.dropzoneBrowse} onClick={handleBrowse} type="button">
            browse
          </button>
        </span>
        <span className={s.dropzoneHint}>PDF, Markdown, TXT, CSV, DOCX, JSON</span>
      </div>

      {files.length > 0 && (
        <div className={s.attachmentList}>
          {files.map((f, i) => (
            <div key={i} className={s.attachmentItem}>
              <Paperclip size={14} />
              <span className={s.attachmentName}>{f.name}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ─── Playbooks sub-component ─────────────────────────────────────────

function PlaybooksSection({
  playbooks,
  onSave,
}: {
  playbooks: Record<string, string>;
  onSave: (updated: Record<string, string>) => void;
}) {
  // CS preset: three named sections. Default: single methodology field.
  const csFields = [
    { key: "at_risk_accounts", label: "At-Risk Accounts", placeholder: "Your approach to at-risk accounts..." },
    { key: "renewal_approach", label: "Renewal Approach", placeholder: "How you manage renewals..." },
    { key: "ebr_qbr_prep", label: "EBR / QBR Preparation", placeholder: "Your EBR/QBR process..." },
  ];

  const hasCSContent = csFields.some((f) => playbooks[f.key]);
  const hasMethodology = !!playbooks.methodology;

  // Show CS fields if any CS content exists or no methodology exists
  const showCS = hasCSContent || !hasMethodology;

  return (
    <div>
      {showCS ? (
        <>
          {csFields.map((field) => (
            <div key={field.key} className={s.fieldRow}>
              <div className={s.fieldLabel}>{field.label}</div>
              <EditableText
                value={playbooks[field.key] ?? ""}
                onChange={(v) => onSave({ ...playbooks, [field.key]: v })}
                placeholder={field.placeholder}
                style={{ fontFamily: "var(--font-sans)", fontSize: 15, fontWeight: 300, color: "var(--color-text-primary)", lineHeight: 1.65 }}
              />
              {field.key !== csFields[csFields.length - 1].key && <hr className={s.thinRule} />}
            </div>
          ))}
        </>
      ) : (
        <div className={s.fieldRow}>
          <div className={s.fieldLabel}>My Methodology</div>
          <EditableText
            value={playbooks.methodology ?? ""}
            onChange={(v) => onSave({ ...playbooks, methodology: v })}
            placeholder="Describe your methodology..."
            style={{ fontFamily: "var(--font-sans)", fontSize: 15, fontWeight: 300, color: "var(--color-text-primary)", lineHeight: 1.65 }}
          />
        </div>
      )}
    </div>
  );
}
