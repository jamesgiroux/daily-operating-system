# I378 — Intelligence Schema Alignment — intelligence.json ↔ entity_intel ↔ Frontend Types

**Status:** Open (0.13.2)
**Priority:** P1
**Version:** 0.13.2
**Area:** Code Quality / Architecture

## Summary

The `intelligence.json` schema produced by AI enrichment, the `entity_intel` table columns in the DB, and the TypeScript types in the frontend are three representations of the same data. These may have drifted — fields added to the prompt may not be in the DB, fields in the DB may not be consumed by the frontend, fields in the TypeScript types may describe a richer product than the backend delivers. This issue creates a field-level alignment document and removes or wires any orphaned fields.

The pre-0.13.0 audit noted: "frontend types describing a richer product than the backend delivers" — this is the systematic investigation of that gap.

## Acceptance Criteria

From the v0.13.2 brief, verified in the codebase and running app:

1. A field-level comparison exists at `.docs/research/i378-schema-alignment.md` between: the `intelligence.json` schema produced by the AI prompt (for accounts, people, and projects), the `entity_intel` table columns in the DB, and the TypeScript types/hooks that consume them on the frontend. Every field is classified as "live" (produced AND consumed), "write-only" (produced, never read by frontend), or "dead" (in schema but never written or always null in real data).
2. Write-only fields are either (a) wired to a frontend consumer that surfaces them, or (b) removed from the AI prompt, schema, and DB column.
3. Dead fields are removed from prompt, schema, and DB (via migration if needed, or just schema/prompt change if the column is nullable and always null).
4. `entity_intel` table structure and `intelligence.json` schema are structurally consistent — no field exists in one but not the other without a documented reason in the alignment doc.
5. For any account, person, or project with real intelligence data: `SELECT * FROM entity_intel WHERE entity_id = '<id>'` and the corresponding `intelligence.json` on disk contain the same data at the same fields. No field is populated in one but null in the other without explanation.

## Dependencies

- May require a DB migration to drop dead columns (trivially small scope).
- Informs I376 (enrichment audit) — the audit may find AI prompts producing dead fields.

## Notes / Rationale

Schema drift is a maintenance cost that compounds over time: AI calls spend tokens producing fields no one reads, migrations accumulate dead columns, frontend type definitions drift from backend reality. A one-time alignment pass with a living document (`i378-schema-alignment.md`) creates a reference that prevents future drift.
