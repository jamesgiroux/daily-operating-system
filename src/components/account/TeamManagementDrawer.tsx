/**
 * TeamManagementDrawer — Sheet for managing account team members.
 * Search + add existing, create + add new, remove members, resolve import notes.
 */
import { Link } from "@tanstack/react-router";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { AccountTeamMember, AccountTeamImportNote, Person } from "@/types";

const roleSuggestions = ["TAM", "CSM", "RM", "AE", "Champion"];

interface TeamManagementDrawerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  accountTeam: AccountTeamMember[];
  accountTeamImportNotes: AccountTeamImportNote[];
  // Search existing
  teamSearchQuery: string;
  setTeamSearchQuery: (v: string) => void;
  teamSearchResults: Person[];
  selectedTeamPerson: Person | null;
  setSelectedTeamPerson: (v: Person | null) => void;
  teamRole: string;
  setTeamRole: (v: string) => void;
  // Inline create
  teamInlineName: string;
  setTeamInlineName: (v: string) => void;
  teamInlineEmail: string;
  setTeamInlineEmail: (v: string) => void;
  teamInlineRole: string;
  setTeamInlineRole: (v: string) => void;
  // State
  teamWorking: boolean;
  resolvedImportNotes: Set<number>;
  teamError: string | null;
  // Actions
  handleAddExistingTeamMember: () => void;
  handleRemoveTeamMember: (personId: string, role: string) => void;
  handleCreateInlineTeamMember: () => void;
  handleImportNoteCreateAndAdd: (noteId: number, name: string, role: string) => void;
}

const sectionLabel: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 10,
  fontWeight: 600,
  textTransform: "uppercase",
  letterSpacing: "0.06em",
  color: "var(--color-text-tertiary)",
  marginBottom: 8,
};

const inputStyle: React.CSSProperties = {
  width: "100%",
  padding: "6px 10px",
  borderRadius: 4,
  border: "1px solid var(--color-rule-light)",
  background: "var(--color-paper-warm-white)",
  fontFamily: "var(--font-sans)",
  fontSize: 13,
  color: "var(--color-text-primary)",
  outline: "none",
};

export function TeamManagementDrawer({
  open,
  onOpenChange,
  accountTeam,
  accountTeamImportNotes,
  teamSearchQuery,
  setTeamSearchQuery,
  teamSearchResults,
  selectedTeamPerson,
  setSelectedTeamPerson,
  teamRole,
  setTeamRole,
  teamInlineName,
  setTeamInlineName,
  teamInlineEmail,
  setTeamInlineEmail,
  teamInlineRole,
  setTeamInlineRole,
  teamWorking,
  resolvedImportNotes,
  teamError,
  handleAddExistingTeamMember,
  handleRemoveTeamMember,
  handleCreateInlineTeamMember,
  handleImportNoteCreateAndAdd,
}: TeamManagementDrawerProps) {
  const unresolvedNotes = accountTeamImportNotes.filter((n) => !resolvedImportNotes.has(n.id));

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" style={{ width: 440, padding: 32, overflowY: "auto" }}>
        <SheetHeader>
          <SheetTitle
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 22,
              fontWeight: 400,
              color: "var(--color-text-primary)",
            }}
          >
            Account Team
          </SheetTitle>
          <SheetDescription style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-tertiary)" }}>
            Manage the people associated with this account.
          </SheetDescription>
        </SheetHeader>

        {/* Current Members */}
        <div style={{ marginTop: 24 }}>
          <div style={sectionLabel}>
            Current Members ({accountTeam.length})
          </div>
          {accountTeam.length > 0 ? (
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {accountTeam.map((member) => (
                <div
                  key={`${member.personId}-${member.role}`}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    padding: "8px 0",
                    borderBottom: "1px solid var(--color-rule-light)",
                  }}
                >
                  <div>
                    <Link
                      to="/people/$personId"
                      params={{ personId: member.personId }}
                      style={{
                        fontFamily: "var(--font-sans)",
                        fontSize: 14,
                        fontWeight: 500,
                        color: "var(--color-text-primary)",
                        textDecoration: "none",
                      }}
                    >
                      {member.personName}
                    </Link>
                    <div
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 10,
                        textTransform: "uppercase",
                        color: "var(--color-text-tertiary)",
                      }}
                    >
                      {member.role}
                    </div>
                  </div>
                  <Button
                    variant="ghost"
                    size="sm"
                    disabled={teamWorking}
                    onClick={() => handleRemoveTeamMember(member.personId, member.role)}
                    style={{ fontFamily: "var(--font-sans)", fontSize: 11 }}
                  >
                    Remove
                  </Button>
                </div>
              ))}
            </div>
          ) : (
            <p style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-tertiary)" }}>
              No team members yet.
            </p>
          )}
        </div>

        {/* Add Existing Person */}
        <div style={{ marginTop: 24, padding: 16, border: "1px solid var(--color-rule-light)", borderRadius: 4 }}>
          <div style={sectionLabel}>Add Existing Person</div>
          <Input
            value={teamSearchQuery}
            onChange={(e) => {
              setTeamSearchQuery(e.target.value);
              setSelectedTeamPerson(null);
            }}
            placeholder="Search people..."
            style={{ marginBottom: 8 }}
          />
          {teamSearchResults.length > 0 && !selectedTeamPerson && (
            <div
              style={{
                maxHeight: 140,
                overflowY: "auto",
                border: "1px solid var(--color-rule-light)",
                borderRadius: 4,
                marginBottom: 8,
              }}
            >
              {teamSearchResults.slice(0, 6).map((person) => (
                <button
                  key={person.id}
                  type="button"
                  onClick={() => {
                    setSelectedTeamPerson(person);
                    setTeamSearchQuery(person.name);
                  }}
                  style={{
                    display: "flex",
                    width: "100%",
                    justifyContent: "space-between",
                    padding: "6px 10px",
                    background: "none",
                    border: "none",
                    cursor: "pointer",
                    fontFamily: "var(--font-sans)",
                    fontSize: 13,
                    textAlign: "left",
                    color: "var(--color-text-primary)",
                  }}
                >
                  <span>{person.name}</span>
                  <span style={{ fontSize: 11, color: "var(--color-text-tertiary)" }}>{person.email}</span>
                </button>
              ))}
            </div>
          )}
          <div style={{ display: "flex", flexWrap: "wrap", gap: 6, alignItems: "center", marginBottom: 8 }}>
            <input
              value={teamRole}
              onChange={(e) => setTeamRole(e.target.value)}
              placeholder="Role"
              style={{ ...inputStyle, width: 100, flex: "none" }}
            />
            {roleSuggestions.map((role) => (
              <button
                key={role}
                type="button"
                onClick={() => setTeamRole(role)}
                style={{
                  padding: "2px 8px",
                  borderRadius: 12,
                  border: "1px solid var(--color-rule-light)",
                  background: "none",
                  cursor: "pointer",
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  color: "var(--color-text-tertiary)",
                }}
              >
                {role}
              </button>
            ))}
          </div>
          <Button
            size="sm"
            disabled={teamWorking || !selectedTeamPerson || !teamRole.trim()}
            onClick={handleAddExistingTeamMember}
            style={{ fontFamily: "var(--font-sans)", fontSize: 12 }}
          >
            Add to Team
          </Button>
        </div>

        {/* Create Person + Add */}
        <div style={{ marginTop: 16, padding: 16, border: "1px solid var(--color-rule-light)", borderRadius: 4 }}>
          <div style={sectionLabel}>Create Person + Add</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <Input
              value={teamInlineName}
              onChange={(e) => setTeamInlineName(e.target.value)}
              placeholder="Name"
            />
            <Input
              value={teamInlineEmail}
              onChange={(e) => setTeamInlineEmail(e.target.value)}
              placeholder="Email (optional)"
            />
            <div style={{ display: "flex", flexWrap: "wrap", gap: 6, alignItems: "center" }}>
              <input
                value={teamInlineRole}
                onChange={(e) => setTeamInlineRole(e.target.value)}
                placeholder="Role"
                style={{ ...inputStyle, width: 100, flex: "none" }}
              />
              {roleSuggestions.map((role) => (
                <button
                  key={`inline-${role}`}
                  type="button"
                  onClick={() => setTeamInlineRole(role)}
                  style={{
                    padding: "2px 8px",
                    borderRadius: 12,
                    border: "1px solid var(--color-rule-light)",
                    background: "none",
                    cursor: "pointer",
                    fontFamily: "var(--font-mono)",
                    fontSize: 10,
                    color: "var(--color-text-tertiary)",
                  }}
                >
                  {role}
                </button>
              ))}
            </div>
            <Button
              size="sm"
              disabled={teamWorking || !teamInlineName.trim() || !teamInlineRole.trim()}
              onClick={handleCreateInlineTeamMember}
              style={{ fontFamily: "var(--font-sans)", fontSize: 12 }}
            >
              Create + Add
            </Button>
          </div>
        </div>

        {/* Migrated Team Notes */}
        {unresolvedNotes.length > 0 && (
          <div
            style={{
              marginTop: 16,
              padding: 16,
              border: "1px solid var(--color-spice-turmeric)",
              borderRadius: 4,
              background: "rgba(201, 162, 39, 0.05)",
            }}
          >
            <div style={sectionLabel}>Migrated Team Notes</div>
            {unresolvedNotes.map((note) => (
              <div
                key={note.id}
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  gap: 12,
                  padding: "8px 0",
                  borderBottom: "1px solid var(--color-rule-light)",
                }}
              >
                <div>
                  <div style={{ fontFamily: "var(--font-sans)", fontSize: 13, fontWeight: 500, color: "var(--color-text-primary)" }}>
                    {note.legacyValue}
                  </div>
                  <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, color: "var(--color-text-tertiary)", textTransform: "uppercase" }}>
                    {note.legacyField} &middot; {note.note}
                  </div>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={teamWorking}
                  onClick={() => handleImportNoteCreateAndAdd(note.id, note.legacyValue, note.legacyField)}
                  style={{ fontFamily: "var(--font-sans)", fontSize: 11, flexShrink: 0 }}
                >
                  Create + Add
                </Button>
              </div>
            ))}
          </div>
        )}

        {teamError && (
          <p style={{ fontFamily: "var(--font-sans)", fontSize: 12, color: "var(--color-spice-terracotta)", marginTop: 12 }}>
            {teamError}
          </p>
        )}
      </SheetContent>
    </Sheet>
  );
}
