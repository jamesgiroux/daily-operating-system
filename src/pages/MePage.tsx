/**
 * MePage — User entity editorial page (/me).
 * Three-section layout: About Me, My Priorities, Context & Knowledge.
 * ADR-0089/0090. Eucalyptus accent.
 */
import { useState, useEffect, useMemo, useCallback, useRef } from "react";
import { User, Target, FileText, Paperclip, Upload } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { useNavigate } from "@tanstack/react-router";
import { toast } from "sonner";
import { getPortfolioReportLabel } from "@/lib/report-config";

import { useMe } from "@/hooks/useMe";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import type { AnnualPriority, QuarterlyPriority, FeatureFlags } from "@/types";

import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { EditableText } from "@/components/ui/EditableText";
import { EntityPicker } from "@/components/ui/entity-picker";
import { ContextEntryList } from "@/components/entity/ContextEntryList";

import s from "./MePage.module.css";

// ─── Chapter definitions ──────────────────────────────────────────────

const CHAPTERS = [
  { id: "about-me", label: "About Me", icon: <User size={18} strokeWidth={1.8} /> },
  { id: "my-priorities", label: "My Priorities", icon: <Target size={18} strokeWidth={1.8} /> },
  { id: "context-entries", label: "Context", icon: <FileText size={18} strokeWidth={1.8} /> },
  { id: "attachments", label: "Attachments", icon: <Paperclip size={18} strokeWidth={1.8} /> },
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

// ─── Main page component ─────────────────────────────────────────────

export default function MePage() {
  const me = useMe();
  const navigate = useNavigate();
  const [activePreset, setActivePreset] = useState<string>("customer-success");
  const [bobEnabled, setBobEnabled] = useState(false);
  useRevealObserver(!me.loading && !!me.userEntity);

  useEffect(() => {
    invoke<{ role?: string }>("get_config")
      .then((c) => setActivePreset(c.role ?? "customer-success"))
      .catch(() => {});
    invoke<FeatureFlags>("get_feature_flags")
      .then((flags) => setBobEnabled(flags.book_of_business_enabled))
      .catch(() => {});
  }, []);

  const shellConfig = useMemo(
    () => ({
      folioLabel: "Me",
      atmosphereColor: "eucalyptus" as const,
      activePage: "me" as const,
      chapters: CHAPTERS,
      folioActions: (
        <div style={{ display: "flex", gap: 8 }}>
          {bobEnabled && (
            <button
              onClick={() => navigate({ to: "/me/reports/book_of_business" })}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                fontWeight: 600,
                letterSpacing: "0.06em",
                textTransform: "uppercase" as const,
                color: "var(--color-spice-turmeric)",
                background: "none",
                border: "1px solid var(--color-spice-turmeric)",
                borderRadius: 4,
                padding: "2px 10px",
                cursor: "pointer",
              }}
            >
              {getPortfolioReportLabel(activePreset)}
            </button>
          )}
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
    [navigate, activePreset, bobEnabled],
  );
  useRegisterMagazineShell(shellConfig);

  if (me.loading) return <EditorialLoading />;
  if (me.error && !me.userEntity) {
    return <EditorialError message={me.error} onRetry={me.load} />;
  }

  const entity = me.userEntity;
  if (!entity) return <EditorialLoading />;

  // Parse JSON fields
  const annualPriorities = parseJsonArray<AnnualPriority>(entity.annualPriorities);
  const quarterlyPriorities = parseJsonArray<QuarterlyPriority>(entity.quarterlyPriorities);

  // Activity check: any non-null user-editable field
  const hasContent = !!(
    entity.name || entity.company || entity.title || entity.focus ||
    entity.companyBio || entity.roleDescription || entity.howImMeasured ||
    (entity.annualPriorities && parseJsonArray<AnnualPriority>(entity.annualPriorities).length > 0) ||
    (entity.quarterlyPriorities && parseJsonArray<QuarterlyPriority>(entity.quarterlyPriorities).length > 0)
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

      {/* ═══ SECTION 2: My Priorities ═══ */}
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

      {/* ═══ SECTION 3: Context & Knowledge ═══ */}
      <section id="context-entries" className={`${s.section} editorial-reveal`} style={{ scrollMarginTop: 60 }}>
        <ChapterHeading
          title="Context"
          epigraph="Professional knowledge that shapes your briefings. Embedding happens in the background."
        />

        <ContextEntryList
          entries={me.contextEntries}
          onUpdate={(id, title, content) => me.updateEntry(id, title, content)}
          onDelete={(id) => me.deleteEntry(id)}
          onCreate={(title, content) => me.createEntry(title, content)}
        />
      </section>

      {/* ═══ SECTION 4: Attachments ═══ */}
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
      toast.error("Failed to process file");
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

