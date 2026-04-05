# I505 — Glean Stakeholder Intelligence: Contact Discovery, Profile Enrichment, and Structural Relationships

**Priority:** P1
**Area:** Backend / Connectors + Entity + Intelligence
**Version:** 1.1.0
**Depends on:** ADR-0098 infrastructure (`data_lifecycle.rs` with `purge_source()` — does NOT exist yet, must be built before I505 can ship purge-on-revocation AC)
**ADR:** 0098 (Data Governance), 0095 (Dual-Mode Context)
**Absorbs:** I486 (Glean person writeback — profile enrichment is part of this issue's scope)
**Supersedes:** I485 (Glean relationship part)

## Problem

Glean surfaces a **full account contact roster** from the org's CRM, support, and communication systems. Querying Glean for JHI (Meridian Asset Investors) returns 72 contacts with:

- **Customer-side contacts** with names, emails, roles, and context ("Primary contact", "Web Developer", "Strategy stakeholder")
- **Internal team assignments** (RM, AE, Division Lead, TAM) with named people and role labels
- **Org chart data** — the `manager` field on `GleanPersonResult` provides direct reporting relationships
- **Cross-system provenance** — contacts from Salesforce, Zendesk, VIP Dashboard, Gong merged into one view

DailyOS currently does **nothing** with this data:

1. **No contact discovery.** The 72 contacts Glean surfaces for an account are never created in the `people` table. I486's approach ("look up by email, update if found") would silently skip contacts we don't already know about.

2. **No entity linkage.** Contacts returned by Glean are associated with the account in the CRM, but DailyOS never creates `entity_people` links from this data. The account's "The Room" chapter only shows people we've manually linked or seen in meetings.

3. **No role context.** Glean tells us someone is "Primary contact" or "Support contact" — stakeholder role data that should inform coverage assessment. This is discarded.

4. **No internal team sync.** Glean returns the internal account team (RM, AE, TAM) which should populate `account_team`. Currently only user-entered team members appear.

5. **No manager relationships.** `GleanPersonResult.manager` is a name field providing direct reporting lines from the org's HRIS/Salesforce. Never written to `person_relationships`.

6. **No profile enrichment for existing people.** Even when a Glean contact matches an existing person by email, their title and department are not written back to the `people` table. (This is what I486 addressed in isolation.)

## Design

### Overview

Single Glean query returns the contact roster. DailyOS processes it in one pass:

```
Glean contact roster query
    │
    ├─ For each contact:
    │   ├─ Match to existing person (by email) → update profile (what I486 did)
    │   ├─ No match → create new person record with data_source="glean"
    │   ├─ Create entity_people link with role context and data_source="glean"
    │   └─ If manager field present → create person_relationships edge
    │
    ├─ For internal team contacts:
    │   └─ Upsert account_team row with role from Glean
    │
    └─ Emit signals per discovery/update
```

### 1. Enhanced Glean Contact Query

The current `search_people("people: {entity_name}", 20)` returns `GleanPersonResult` with 7 fields. This may not capture the full contact roster. Investigate whether a more targeted query (e.g., `"contacts for account {name}"` or `"salesforce contacts {name}"`) returns richer data, including:

- Contact role on the account (Decision Maker, Economic Buyer, Champion)
- Whether the contact is internal (Automattic/VIP) or external (customer)
- The contact's relationship to the account (primary, support, technical)

**Validation gate:** Before implementation, query Glean with several query formats and document what comes back. The acceptance criteria will be adjusted based on what Glean actually provides. The JHI example showed 72 contacts with role context — we need to confirm the API returns comparable structure programmatically.

### 2. Contact Discovery: Create New People

For each Glean contact with a non-empty email that does NOT match an existing person:

```rust
let person_id = format!("p-glean-{}", uuid::Uuid::new_v4());
db.create_person(&CreatePerson {
    id: &person_id,
    email: &email,
    name: name.as_deref(),
    organization: department.as_deref(),
    role: title.as_deref(),
    // ... other available fields
})?;

// Track provenance per ADR-0098
db.set_enrichment_sources(&person_id, &json!({
    "name": {"source": "glean", "at": now},
    "role": {"source": "glean", "at": now},
    "organization": {"source": "glean", "at": now},
}))?;
```

Discovered contacts are lightweight records — name, email, title, department from Glean. Full enrichment (Clay, Gravatar, AI) happens on subsequent enrichment cycles if the person appears in meetings.

### 3. Profile Enrichment for Existing People (absorbs I486)

For each Glean contact that matches an existing person by email:

- Build `ProfileUpdate` from Glean fields (title → role, department → organization, location → company_hq)
- Call `db.update_person_profile(person_id, &update, "glean")`
- Source priority: Glean = 2 (same as Gravatar). User (4) and Clay (3) overrides preserved.
- Emit `"profile_enriched"` signal per update

This is exactly what I486 specified — absorbed here because discovery and enrichment are the same Glean query.

### 4. Entity-People Linkage with Role Context

For each contact (new or existing), create or update `entity_people` linkage:

```rust
db.link_person_to_entity(&LinkPersonToEntity {
    entity_id: &entity_id,
    person_id: &person_id,
    relationship_type: &role_context,  // "primary_contact", "support_contact", "web_developer", etc.
    data_source: "glean",              // ADR-0098: source tagging for purge-on-revocation
})?;
```

The `entity_people` table needs the `data_source` column added per ADR-0098 before this can ship.

**Role context mapping:** Glean returns free-text role descriptions. Map to standardized relationship types where possible:
- "Primary contact" → `"primary_contact"`
- "Support contact" → `"support_contact"`
- "Web Developer" / technical roles → `"technical"`
- Unknown / unclassifiable → `"associated"` (existing default)

Don't over-classify — free-text roles are more useful than wrong categories. Store the original Glean role text alongside the standardized type.

### 5. Internal Team Sync

Glean returns internal team assignments: RM, Expansion AE, Division Lead, TAM. These map to `account_team` rows:

```rust
// Distinguish internal vs. external contacts
// Internal: email domain matches user's org domain
// External: email domain matches account's domain
if is_internal_contact(&contact.email, &user_domain) {
    db.upsert_account_team(&UpsertAccountTeam {
        account_id: &entity_id,
        person_id: &person_id,
        role: &team_role,  // "RM", "Expansion AE", "TAM", etc.
    })?;
}
```

Internal team roles from Glean should NOT overwrite user-set team roles (the user may have more nuanced role descriptions).

### 6. Manager Relationships

When `GleanPersonResult.manager` is non-empty:

1. Fuzzy match the manager name to existing `people.name` records (case-insensitive, trim whitespace)
2. If matched, create `person_relationships` edge:
   - `relationship_type`: `"manager"`
   - `direction`: `"directed"` (from person → to manager)
   - `confidence`: 0.8 (high — org chart data from HRIS/Salesforce is authoritative)
   - `source`: `"glean"`
   - `context_entity_id`: the account where this was discovered
3. If no match found, log and skip — don't create phantom people just for the manager edge

**Ambiguity note:** Name-only fuzzy matching is unreliable. "John Smith" could match multiple people. Mitigation: require exact name match (case-insensitive, trimmed) AND same account context. If multiple matches found, skip the edge rather than guessing wrong. Consider email-based matching if Glean provides manager email alongside name.

Guard: don't overwrite user-confirmed manager relationships (same guard as I504).

### 7. Purge-on-Revocation (ADR-0098)

All data written by this issue carries `source = "glean"` or `data_source = "glean"`:
- `people` enrichment_sources with `source: "glean"`
- `entity_people` with `data_source = "glean"`
- `person_relationships` with `source = "glean"`
- `account_team` entries (need source tracking — may require column addition)

When Glean auth is revoked, `purge_source(DataSource::Glean)` clears:
- Glean-sourced fields on people (role, organization where source = glean)
- entity_people rows with data_source = glean
- person_relationships rows with source = glean
- People records created entirely from Glean (all enrichment_sources are glean) — these should be archived, not deleted, to preserve any user-added notes

### 8. Staleness and Refresh

Glean contact data is refreshed each enrichment cycle. Contacts that Glean no longer returns for an account are NOT auto-deleted — they may have moved roles or left, which is itself a signal. Instead, track `last_seen_in_glean` timestamp on entity_people rows:
- Present in Glean this cycle → update timestamp
- Absent for 30+ days → flag as "may have departed" in stakeholder coverage

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/context_provider/glean.rs` | Enhanced contact query. Discovery loop: match or create people, link to entity, sync account_team, create manager relationships. Profile enrichment (absorbs I486). |
| `src-tauri/src/db/people.rs` | Add `"glean"` to `source_priority()` at level 2. Add `create_person_minimal()` for lightweight Glean-discovered contacts. |
| `src-tauri/src/db/entity_people.rs` or equivalent | Add `data_source` column support. Add `link_person_to_entity_with_source()`. |
| `src-tauri/src/migrations/` | New migration: add `data_source TEXT DEFAULT 'user'` to `entity_people`. Add `last_seen_in_glean TEXT` to entity_people. |
| `src-tauri/src/data_lifecycle.rs` (from ADR-0098) | Glean purge covers entity_people, person profile fields, person_relationships. |

## Acceptance Criteria

1. With Glean connected, enrich account. People NOT previously in the system appear in `people` table with `enrichment_sources` showing `source: "glean"`.
2. New Glean-discovered people are linked to the entity via `entity_people` with `data_source = "glean"` and role context from Glean.
3. Existing people matched by email have profile fields updated from Glean (title → role, department → organization) respecting source priority. User-set values NOT overwritten.
4. Internal team members (RM, AE, TAM) appear in `account_team` with roles from Glean.
5. Manager relationships appear in `person_relationships` with type `"manager"`, source `"glean"`, confidence 0.8, direction `"directed"`.
6. Account detail "The Room" chapter shows Glean-discovered contacts with title/department.
7. Disconnecting Glean in Settings purges: entity_people rows with `data_source = "glean"`, Glean-sourced profile fields on people, Glean-sourced person_relationships. User-entered data preserved.
8. Signal emitted per new person discovery and per profile update.
9. Re-enrichment with Glean updates existing contacts (upsert, not duplicate).

## Validation Gate

**Before implementation**, query Glean programmatically with multiple query formats:
- `"people: {account_name}"` (current format)
- `"contacts for {account_name}"`
- `"salesforce contacts {account_name}"`
- `"{account_name} account team"`

Document what comes back for each format. The spec assumes we get 72 contacts with role context per the JHI example — if the API returns less, adjust scope to match reality.

## Out of Scope

- AI-inferred relationships (I504 — separate source with different confidence)
- Co-attendance inference (I506 — algorithmic, no Glean dependency)
- Writing back to Glean — DailyOS is a consumer, never a writer
- Contact deduplication across sources — if Clay and Glean both know a person, the existing `update_person_profile` source priority handles field conflicts. Separate person records are prevented by email matching.
- Full purge-on-revocation UX (grace period, confirmation dialog) — that's an ADR-0098 implementation detail
