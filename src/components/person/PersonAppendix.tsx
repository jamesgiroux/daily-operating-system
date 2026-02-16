/**
 * PersonAppendix — Appendix section for person detail editorial page.
 * Details grid (name, email, role, org, first/last seen), notes (editable),
 * duplicate detection candidates with merge buttons.
 */
import React from "react";
import type { PersonDetail, DuplicateCandidate, ContentFile } from "@/types";
import { formatShortDate } from "@/lib/utils";
import { FileListSection } from "@/components/entity/FileListSection";

interface PersonAppendixProps {
  detail: PersonDetail;
  editName: string;
  setEditName: (v: string) => void;
  editRole: string;
  setEditRole: (v: string) => void;
  editNotes: string;
  setEditNotes: (v: string) => void;
  onSave: () => void;
  dirty: boolean;
  saving: boolean;
  duplicateCandidates: DuplicateCandidate[];
  onMergeSuggested: (candidate: DuplicateCandidate) => void;
  merging: boolean;
  // Files
  files?: ContentFile[];
  onIndexFiles?: () => void;
  indexing?: boolean;
  indexFeedback?: string | null;
}

const sectionLabelStyle: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 11,
  fontWeight: 500,
  textTransform: "uppercase",
  letterSpacing: "0.1em",
  color: "var(--color-text-tertiary)",
  marginBottom: 16,
};

const ruleStyle: React.CSSProperties = {
  borderTop: "2px solid var(--color-rule-heavy)",
  paddingTop: 24,
  marginTop: 32,
};

export function PersonAppendix({
  detail,
  editName,
  setEditName,
  editRole,
  setEditRole,
  editNotes,
  setEditNotes,
  onSave,
  dirty,
  saving,
  duplicateCandidates,
  onMergeSuggested,
  merging,
  files,
  onIndexFiles,
  indexing,
  indexFeedback,
}: PersonAppendixProps) {
  const fieldDirty =
    editName !== detail.name ||
    editRole !== (detail.role ?? "") ||
    editNotes !== (detail.notes ?? "");

  return (
    <section id="appendix" style={{ scrollMarginTop: 60, paddingTop: 80 }}>
      <div
        style={{
          borderTop: "3px double var(--color-rule-heavy)",
          paddingTop: 32,
        }}
      >
        <div style={sectionLabelStyle}>Appendix</div>

        {/* Details — editable fields */}
        <div style={ruleStyle}>
          <div
            style={{
              display: "flex",
              alignItems: "baseline",
              justifyContent: "space-between",
              marginBottom: 16,
            }}
          >
            <div style={sectionLabelStyle}>Details</div>
            {(dirty || fieldDirty) && (
              <button
                onClick={onSave}
                disabled={saving}
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  color: "var(--color-garden-larkspur)",
                  background: "none",
                  border: "none",
                  cursor: saving ? "default" : "pointer",
                  textTransform: "uppercase",
                  letterSpacing: "0.06em",
                  padding: 0,
                }}
              >
                {saving ? "Saving\u2026" : "Save"}
              </button>
            )}
          </div>

          <div
            style={{
              display: "grid",
              gridTemplateColumns: "100px 1fr",
              gap: "12px 24px",
            }}
          >
            <FieldLabel>Name</FieldLabel>
            <FieldInput value={editName} onChange={setEditName} placeholder="Full name" />

            <FieldLabel>Email</FieldLabel>
            <FieldValue>{detail.email}</FieldValue>

            <FieldLabel>Role</FieldLabel>
            <FieldInput value={editRole} onChange={setEditRole} placeholder="Role / Title" />

            <FieldLabel>Organization</FieldLabel>
            <FieldValue>{detail.organization ?? "\u2014"}</FieldValue>

            <FieldLabel>Relationship</FieldLabel>
            <FieldValue style={{ textTransform: "capitalize" }}>{detail.relationship}</FieldValue>

            {detail.firstSeen && (
              <>
                <FieldLabel>First Seen</FieldLabel>
                <FieldValue>{formatShortDate(detail.firstSeen)}</FieldValue>
              </>
            )}

            {detail.lastSeen && (
              <>
                <FieldLabel>Last Seen</FieldLabel>
                <FieldValue>{formatShortDate(detail.lastSeen)}</FieldValue>
              </>
            )}
          </div>
        </div>

        {/* Notes (editable) */}
        <div style={ruleStyle}>
          <div style={sectionLabelStyle}>Notes</div>
          <textarea
            value={editNotes}
            onChange={(e) => setEditNotes(e.target.value)}
            placeholder="Notes about this person\u2026"
            rows={6}
            style={{
              width: "100%",
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              lineHeight: 1.65,
              color: "var(--color-text-primary)",
              background: "none",
              border: "none",
              borderBottom: "1px solid var(--color-rule-light)",
              outline: "none",
              resize: "vertical",
              padding: "8px 0",
            }}
          />
        </div>

        {/* Files */}
        {files && (
          <div style={ruleStyle}>
            <FileListSection
              files={files}
              onIndexFiles={onIndexFiles}
              indexing={indexing}
              indexFeedback={indexFeedback}
            />
          </div>
        )}

        {/* Duplicate Detection */}
        {duplicateCandidates.length > 0 && (
          <div style={ruleStyle}>
            <div style={sectionLabelStyle}>
              Potential Duplicates ({duplicateCandidates.length})
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {duplicateCandidates.map((candidate) => {
                const targetName =
                  candidate.person1Id === detail.id
                    ? candidate.person2Name
                    : candidate.person1Name;
                return (
                  <div
                    key={`${candidate.person1Id}-${candidate.person2Id}`}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      padding: "10px 0",
                      borderBottom: "1px solid var(--color-rule-light)",
                    }}
                  >
                    <div>
                      <span
                        style={{
                          fontFamily: "var(--font-sans)",
                          fontSize: 14,
                          fontWeight: 500,
                          color: "var(--color-text-primary)",
                        }}
                      >
                        Merge into {targetName}
                      </span>
                      <span
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 10,
                          color: "var(--color-text-tertiary)",
                          marginLeft: 12,
                        }}
                      >
                        {candidate.reason} \u00B7 {Math.round(candidate.confidence * 100)}%
                      </span>
                    </div>
                    <button
                      onClick={() => onMergeSuggested(candidate)}
                      disabled={merging}
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 10,
                        color: "var(--color-garden-larkspur)",
                        background: "none",
                        border: "none",
                        cursor: merging ? "default" : "pointer",
                        textTransform: "uppercase",
                        letterSpacing: "0.06em",
                        padding: 0,
                      }}
                    >
                      Merge
                    </button>
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </div>
    </section>
  );
}

function FieldLabel({ children }: { children: React.ReactNode }) {
  return (
    <span
      style={{
        fontFamily: "var(--font-mono)",
        fontSize: 10,
        fontWeight: 500,
        textTransform: "uppercase",
        letterSpacing: "0.06em",
        color: "var(--color-text-tertiary)",
        paddingTop: 6,
      }}
    >
      {children}
    </span>
  );
}

function FieldValue({
  children,
  style,
}: {
  children: React.ReactNode;
  style?: React.CSSProperties;
}) {
  return (
    <span
      style={{
        fontFamily: "var(--font-sans)",
        fontSize: 14,
        color: "var(--color-text-primary)",
        paddingTop: 4,
        ...style,
      }}
    >
      {children}
    </span>
  );
}

function FieldInput({
  value,
  onChange,
  placeholder,
}: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
}) {
  return (
    <input
      type="text"
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
      style={{
        fontFamily: "var(--font-sans)",
        fontSize: 14,
        color: "var(--color-text-primary)",
        background: "none",
        border: "none",
        borderBottom: "1px solid var(--color-rule-light)",
        outline: "none",
        padding: "4px 0",
        width: "100%",
      }}
    />
  );
}
