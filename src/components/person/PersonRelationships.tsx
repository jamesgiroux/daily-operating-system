/**
 * PersonRelationships — "Their Network" chapter showing person-to-person
 * relationship edges, grouped by context entity. (I392, ADR-0088)
 *
 * Always renders so users can manually add connections even when none exist yet.
 */
import { useState, useCallback } from "react";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Plus, X, Check } from "lucide-react";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { NetworkIntelligence, PersonRelationshipEdge } from "@/types";
import type { RolePreset } from "@/types/preset";
import s from "./PersonRelationships.module.css";

/**
 * Dropdown choices for adding a connection. Asymmetric types appear twice:
 * once forward ("Manager" — the other person manages this one) and once
 * inverse ("Direct Report" — this person manages the other). Selecting an
 * inverse swaps from/to so the stored edge direction stays correct.
 */
interface RelChoice {
  /** Value stored in DB */
  type: string;
  /** Display label in dropdown */
  label: string;
  /** When true, from/to are swapped on insert (current person becomes "to") */
  inverse: boolean;
}

const RELATIONSHIP_CHOICES: RelChoice[] = [
  { type: "peer", label: "Peer", inverse: false },
  { type: "manager", label: "Manager", inverse: false },
  { type: "manager", label: "Direct Report", inverse: true },
  { type: "mentor", label: "Mentor", inverse: false },
  { type: "mentor", label: "Mentee", inverse: true },
  { type: "collaborator", label: "Collaborator", inverse: false },
  { type: "ally", label: "Ally", inverse: false },
  { type: "partner", label: "Partner", inverse: false },
  { type: "introduced_by", label: "Introduced By", inverse: false },
];

interface PersonRelationshipsProps {
  personId: string;
  network?: NetworkIntelligence;
  relationships: PersonRelationshipEdge[];
  preset?: RolePreset;
  chapterTitle?: string;
  onRelationshipsChanged?: () => void;
}

/** Inverse labels for asymmetric relationship types viewed from the "to" side. */
const INVERSE_LABELS: Record<string, string> = {
  manager: "Direct Report",
  mentor: "Mentee",
  introduced_by: "Introduced",
};

function resolveRelationshipLabel(type: string, preset?: RolePreset, inverse?: boolean): string {
  if (inverse && INVERSE_LABELS[type]) {
    return INVERSE_LABELS[type];
  }
  if (preset?.relationshipVocabulary?.[type]) {
    return preset.relationshipVocabulary[type];
  }
  return type
    .replace(/_/g, " ")
    .replace(/\b\w/g, (c) => c.toUpperCase());
}

function confidencePercent(c: number): string {
  return `${Math.round(c * 100)}%`;
}

export function PersonRelationships({
  personId,
  network,
  relationships,
  preset,
  chapterTitle = "Their Network",
  onRelationshipsChanged,
}: PersonRelationshipsProps) {
  const [adding, setAdding] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<{ id: string; name: string; email: string; organization?: string }[]>([]);
  const [selectedPerson, setSelectedPerson] = useState<{ id: string; name: string } | null>(null);
  const [selectedChoiceIdx, setSelectedChoiceIdx] = useState(0);
  const [saving, setSaving] = useState(false);

  const handleSearch = useCallback(async (query: string) => {
    setSearchQuery(query);
    if (query.length < 2) {
      setSearchResults([]);
      return;
    }
    try {
      const results = await invoke<{ id: string; name: string; email: string; organization?: string }[]>("search_people", { query });
      const connectedIds = new Set(relationships.map((r) =>
        r.fromPersonId === personId ? r.toPersonId : r.fromPersonId
      ));
      setSearchResults(results.filter((p) => p.id !== personId && !connectedIds.has(p.id)));
    } catch {
      setSearchResults([]);
    }
  }, [personId, relationships]);

  const handleAdd = useCallback(async () => {
    if (!selectedPerson) return;
    setSaving(true);
    const choice = RELATIONSHIP_CHOICES[selectedChoiceIdx];
    try {
      await invoke("upsert_person_relationship", {
        payload: {
          fromPersonId: choice.inverse ? selectedPerson.id : personId,
          toPersonId: choice.inverse ? personId : selectedPerson.id,
          relationshipType: choice.type,
          direction: "symmetric",
          confidence: 1.0,
          source: "user_confirmed",
        },
      });
      setAdding(false);
      setSelectedPerson(null);
      setSearchQuery("");
      setSearchResults([]);
      setSelectedChoiceIdx(0);
      onRelationshipsChanged?.();
    } catch (err) {
      console.error("Failed to add relationship:", err);
      toast.error("Failed to add relationship");
    } finally {
      setSaving(false);
    }
  }, [personId, selectedPerson, selectedChoiceIdx, onRelationshipsChanged]);

  const handleDelete = useCallback(async (id: string) => {
    try {
      await invoke("delete_person_relationship", { id });
      onRelationshipsChanged?.();
    } catch (err) {
      console.error("Failed to delete relationship:", err);
      toast.error("Failed to remove relationship");
    }
  }, [onRelationshipsChanged]);

  const handleConfirm = useCallback(async (rel: PersonRelationshipEdge, choiceIdx: number) => {
    const choice = RELATIONSHIP_CHOICES[choiceIdx];
    const isFrom = rel.fromPersonId === personId;
    const otherId = isFrom ? rel.toPersonId : rel.fromPersonId;
    try {
      await invoke("upsert_person_relationship", {
        payload: {
          fromPersonId: choice.inverse ? otherId : personId,
          toPersonId: choice.inverse ? personId : otherId,
          relationshipType: choice.type,
          direction: "symmetric",
          confidence: 1.0,
          source: "user_confirmed",
          contextEntityId: rel.contextEntityId ?? null,
          contextEntityType: rel.contextEntityType ?? null,
        },
      });
      // Delete the original suggested edge if it's a different ID
      // (upsert may have created a new row instead of updating the AI one)
      try {
        await invoke("delete_person_relationship", { id: rel.id });
      } catch {
        // May already be replaced by upsert — that's fine
      }
      onRelationshipsChanged?.();
    } catch (err) {
      console.error("Failed to confirm relationship:", err);
      toast.error("Failed to confirm relationship");
    }
  }, [personId, onRelationshipsChanged]);

  const handleCancel = useCallback(() => {
    setAdding(false);
    setSelectedPerson(null);
    setSearchQuery("");
    setSearchResults([]);
    setSelectedChoiceIdx(0);
  }, []);

  // Group by context entity
  const ungrouped: PersonRelationshipEdge[] = [];
  const grouped: Record<string, { type: string; name?: string; edges: PersonRelationshipEdge[] }> = {};

  for (const rel of relationships) {
    if (rel.contextEntityId) {
      const key = rel.contextEntityId;
      if (!grouped[key]) {
        grouped[key] = { type: rel.contextEntityType ?? "account", name: rel.contextEntityName, edges: [] };
      }
      grouped[key].edges.push(rel);
    } else {
      ungrouped.push(rel);
    }
  }

  const hasEdges = relationships.length > 0;
  const hasIntel = !!network?.clusterSummary;

  return (
    <section className={s.chapter}>
      <ChapterHeading title={chapterTitle} />

      {network?.clusterSummary && (
        <p className={s.clusterSummary}>{network.clusterSummary}</p>
      )}

      {network?.risks && network.risks.length > 0 && (
        <div className={s.calloutList}>
          {network.risks.map((risk, i) => (
            <div key={i} className={s.calloutRisk}>{risk}</div>
          ))}
        </div>
      )}

      {network?.opportunities && network.opportunities.length > 0 && (
        <div className={s.calloutList}>
          {network.opportunities.map((opp, i) => (
            <div key={i} className={s.calloutOpportunity}>{opp}</div>
          ))}
        </div>
      )}

      {ungrouped.length > 0 && (
        <div className={s.edgeGroup}>
          {ungrouped.map((rel) => (
            <EdgeRow key={rel.id} rel={rel} personId={personId} preset={preset} onDelete={handleDelete} onConfirm={handleConfirm} />
          ))}
        </div>
      )}

      {Object.entries(grouped).map(([contextId, group]) => (
        <div key={contextId} className={s.edgeGroup}>
          <div className={s.groupLabel}>
            {group.type}: {group.name ?? contextId}
          </div>
          {group.edges.map((rel) => (
            <EdgeRow key={rel.id} rel={rel} personId={personId} preset={preset} onDelete={handleDelete} onConfirm={handleConfirm} />
          ))}
        </div>
      ))}

      {!hasEdges && !hasIntel && !adding && (
        <p className={s.emptyState}>No connections yet. Add one to start mapping this person's network.</p>
      )}

      {/* Add connection flow */}
      {adding ? (
        <div className={s.addFlow}>
          {!selectedPerson ? (
            <>
              <input
                className={s.searchInput}
                type="text"
                value={searchQuery}
                onChange={(e) => handleSearch(e.target.value)}
                placeholder="Search by name or email…"
                autoFocus
              />
              {searchResults.length > 0 && (
                <div className={s.searchResults}>
                  {searchResults.slice(0, 8).map((p) => (
                    <button
                      key={p.id}
                      className={s.searchResultItem}
                      onClick={() => {
                        setSelectedPerson({ id: p.id, name: p.name });
                        setSearchResults([]);
                        setSearchQuery("");
                      }}
                    >
                      <span className={s.searchResultName}>{p.name}</span>
                      <span className={s.searchResultMeta}>
                        {p.email}{p.organization ? ` · ${p.organization}` : ""}
                      </span>
                    </button>
                  ))}
                </div>
              )}
              {searchQuery.length >= 2 && searchResults.length === 0 && (
                <p className={s.noResults}>No matching people found</p>
              )}
              <button className={s.cancelBtn} onClick={handleCancel}>Cancel</button>
            </>
          ) : (
            <div className={s.typeSelector}>
              <div className={s.selectedPersonLabel}>
                {selectedPerson.name}
                <button className={s.clearPersonBtn} onClick={() => setSelectedPerson(null)}>
                  <X size={14} />
                </button>
              </div>
              <div className={s.typeSelectorRow}>
                <span className={s.typeLabel}>Relationship</span>
                <Select value={String(selectedChoiceIdx)} onValueChange={(v) => setSelectedChoiceIdx(Number(v))}>
                  <SelectTrigger
                    className=""
                    style={{
                      flex: 1,
                      fontFamily: "var(--font-sans)",
                      fontSize: 13,
                      color: "var(--color-text-primary)",
                      background: "var(--color-paper-warm-white)",
                      border: "1px solid var(--color-paper-linen)",
                      borderRadius: 4,
                      padding: "5px 8px",
                      height: "auto",
                      boxShadow: "none",
                    }}
                  >
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent
                    position="popper"
                    style={{
                      background: "var(--color-paper-warm-white)",
                      border: "1px solid var(--color-paper-linen)",
                      borderRadius: 6,
                      fontFamily: "var(--font-sans)",
                      fontSize: 13,
                      maxHeight: 240,
                    }}
                  >
                    {RELATIONSHIP_CHOICES.map((choice, i) => (
                      <SelectItem
                        key={`${choice.type}-${choice.inverse}`}
                        value={String(i)}
                        style={{ fontFamily: "var(--font-sans)", fontSize: 13 }}
                      >
                        {choice.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className={s.typeSelectorActions}>
                <button className={s.cancelBtn} onClick={handleCancel}>Cancel</button>
                <button className={s.saveBtn} onClick={handleAdd} disabled={saving}>
                  {saving ? "Adding…" : "Add Connection"}
                </button>
              </div>
            </div>
          )}
        </div>
      ) : (
        <button className={s.addBtn} onClick={() => setAdding(true)}>
          <Plus size={14} /> Add connection
        </button>
      )}
    </section>
  );
}

function EdgeRow({
  rel,
  personId,
  preset,
  onDelete,
  onConfirm,
}: {
  rel: PersonRelationshipEdge;
  personId: string;
  preset?: RolePreset;
  onDelete?: (id: string) => void;
  onConfirm?: (rel: PersonRelationshipEdge, choiceIdx: number) => void;
}) {
  const [confirming, setConfirming] = useState(false);
  const [choiceIdx, setChoiceIdx] = useState(() => {
    // Pre-select the AI-suggested relationship type in the dropdown
    const idx = RELATIONSHIP_CHOICES.findIndex(
      (c) => c.type === rel.relationshipType && !c.inverse,
    );
    return idx >= 0 ? idx : 0;
  });

  const isFrom = rel.fromPersonId === personId;
  const otherId = isFrom ? rel.toPersonId : rel.fromPersonId;
  const otherName = isFrom
    ? (rel.toPersonName ?? otherId)
    : (rel.fromPersonName ?? otherId);
  const label = resolveRelationshipLabel(rel.relationshipType, preset, !isFrom);
  const isSuggested = rel.source === "ai_enrichment" || rel.source === "co_attendance";

  if (confirming && onConfirm) {
    return (
      <div className={s.edgeRow} style={{ flexWrap: "wrap", gap: "6px 8px" }}>
        <span className={s.edgeLink} style={{ cursor: "default" }}>{otherName}</span>
        <Select value={String(choiceIdx)} onValueChange={(v) => setChoiceIdx(Number(v))}>
          <SelectTrigger
            className=""
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 12,
              color: "var(--color-text-primary)",
              background: "var(--color-paper-warm-white)",
              border: "1px solid var(--color-paper-linen)",
              borderRadius: 4,
              padding: "3px 8px",
              height: "auto",
              boxShadow: "none",
              width: "auto",
              minWidth: 120,
            }}
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent
            position="popper"
            style={{
              background: "var(--color-paper-warm-white)",
              border: "1px solid var(--color-paper-linen)",
              borderRadius: 6,
              fontFamily: "var(--font-sans)",
              fontSize: 12,
              maxHeight: 240,
            }}
          >
            {RELATIONSHIP_CHOICES.map((choice, i) => (
              <SelectItem
                key={`${choice.type}-${choice.inverse}`}
                value={String(i)}
                style={{ fontFamily: "var(--font-sans)", fontSize: 12 }}
              >
                {choice.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <button
          className={s.confirmBtn}
          onClick={() => {
            onConfirm(rel, choiceIdx);
            setConfirming(false);
          }}
        >
          Confirm
        </button>
        <button className={s.cancelBtn} onClick={() => setConfirming(false)} style={{ padding: "3px 8px" }}>
          Cancel
        </button>
      </div>
    );
  }

  return (
    <div className={s.edgeRow}>
      <Link
        to="/people/$personId"
        params={{ personId: otherId }}
        className={s.edgeLink}
      >
        {otherName}
      </Link>

      <span className={s.edgeBadge}>{label}</span>
      {isSuggested && <span className={s.suggestedBadge}>Suggested</span>}

      <span className={s.edgeMeta}>
        {rel.source !== "user_confirmed" && (
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <span className={s.edgeConfidence}>{confidencePercent(rel.effectiveConfidence)}</span>
              </TooltipTrigger>
              <TooltipContent side="top">
                AI-inferred confidence — decays over time without reinforcing evidence
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}
        {isSuggested && rel.rationale && (
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <span className={s.edgeRationale}>why</span>
              </TooltipTrigger>
              <TooltipContent side="top">{rel.rationale}</TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}
        {rel.lastReinforcedAt && (
          <span className={s.edgeDate}>
            {new Date(rel.lastReinforcedAt).toLocaleDateString()}
          </span>
        )}
      </span>

      {isSuggested && onConfirm && (
        <button className={s.acceptBtn} onClick={() => setConfirming(true)} title="Accept suggestion">
          <Check size={13} />
        </button>
      )}

      {onDelete && (
        <button className={s.deleteBtn} onClick={() => onDelete(rel.id)} title="Remove connection">
          <X size={13} />
        </button>
      )}
    </div>
  );
}
