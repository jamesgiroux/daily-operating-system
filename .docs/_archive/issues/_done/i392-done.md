# I392 — "Their Network" Chapter on Person Detail + "Their Orbit" Rename — Cluster View, Preset Vocabulary, Risks/Opportunities

**Status:** Open (0.13.5)
**Priority:** P1
**Version:** 0.13.5
**Area:** Frontend / UX

## Summary

The PersonDetailEditorial page gains a **"Their Network"** chapter that surfaces the person's relationship graph intelligence (I391). The chapter shows: a compact list of typed relationship edges grouped by context (Account X relationships, Project Y relationships), each with person name, relationship type badge, confidence indicator, and last-reinforced date; the AI-generated cluster summary paragraph; and network risks and opportunities as callout-style components. Relationship type labels adapt to the active role preset's vocabulary via a new `relationshipVocabulary` field on `RolePreset`.

The existing Chapter 3 "The Network" (linked accounts/projects) is renamed to **"Their Orbit"**. The "Their" prefix becomes a design pattern exclusive to person entity pages, subtly signaling that person pages are *about someone* rather than a system concept.

The "Their Network" chapter does not render if the person has no relationship edges — absence communicates "no known network" cleanly without an empty state.

## Acceptance Criteria

1. The existing Chapter 3 "The Network" (linked accounts/projects) is renamed to **"Their Orbit"**. The chapter content and functionality are unchanged — only the heading text changes.

2. Open any person detail page for a person with ≥1 relationship edge. A **"Their Network"** chapter appears on the page as a visually distinct section, positioned after "Their Orbit" and before "The Landscape."

3. Each relationship edge renders: connected person's name (clickable link to their detail page), relationship type badge displaying the appropriate preset-sensitive vocabulary, confidence indicator (visual or numeric), and last-reinforced date.

4. If edges have `context_entity_id` set, they are grouped under a header showing that entity's name (e.g., "Acme Corp" for edges scoped to Account X).

5. The AI-generated `cluster_summary` from `intelligence.json.network.cluster_summary` renders as a prose paragraph in the "Their Network" chapter.

6. `network.risks` renders as callout/signal prose components using the existing callout pattern from the design system (no new component). `network.opportunities` renders the same way.

7. Switch the active role preset to CS/Sales. Open a person detail page. Relationship type badges render with CS vocabulary (e.g., `champion` displays as "Champion", `executive_sponsor` displays as "Economic Buyer", `blocker` displays as "Blocker"). Switch to Product/Engineering preset. Labels adapt (e.g., `champion` displays as "Advocate", `executive_sponsor` displays as "Executive Sponsor", `blocker` displays as "Dependency Risk"). No app restart required.

8. Open a person detail page for a person with zero relationship edges. The "Their Network" chapter does not render — no heading, no empty state, no placeholder. "Their Orbit" still shows if the person has linked accounts/projects.

## Implementation Notes

- **Chapter structure**: `PersonDetailEditorial.tsx` uses `buildChapters()` at line 98 to define chapters. Update: rename Chapter 3 from "The Network" to "Their Orbit", insert new Chapter 4 "Their Network" between "Their Orbit" and "The Landscape".
- **Existing component**: `PersonNetwork.tsx` renders entity links for "Their Orbit" — no changes needed to that component, only its chapter heading.
- **New component**: `PersonRelationships.tsx` (or similar) for "Their Network". Renders edges from `person_relationships` + `intelligence.json.network`. Uses existing `StateBlock` for cluster summary, `BriefingCallouts` pattern for risks/opportunities.
- **Preset vocabulary**: Add `relationshipVocabulary?: Record<string, string>` to `RolePreset` in `types/preset.ts`. Resolve at render time. Fallback: title-case the internal type name if no vocabulary entry exists.
- **Person navigation**: Use React Router `<Link to="/people/$personId">` pattern, same as entity navigation in `PersonNetwork.tsx:160`.
- **Data loading**: `usePersonDetail.ts` already loads `EntityIntelligence` which will include the new `network` field via `Option<NetworkIntelligence>` — no hook changes needed.
- **Design system**: Use existing callout pattern (severity-coded left border), `StateBlock` (colored label + prose), and editorial section rules. No new design patterns.

## Dependencies

- Blocked by I391 (network intelligence must populate `intelligence.json.network` first; edges must exist in `person_relationships`).
- See ADR-0088 decisions 5 and 6.

## Notes / Rationale

The "Their Network" chapter closes the loop between the intelligence system and the user: I390 stores the graph, I391 generates network intelligence from it, I392 makes it visible. A user before a meeting with Jack can now see not just Jack's individual profile, but how Jack relates to the other people in the room — who's the champion, who's the blocker, what the influence path looks like. That's the chief-of-staff experience ADR-0088 describes.

The "Their" naming convention for person pages is a deliberate editorial choice — it individualizes chapters that are about a specific person's world ("Their Orbit" = their linked entities, "Their Network" = their people relationships), distinct from the system-level "The" convention used on account and project pages.
