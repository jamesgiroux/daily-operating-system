# ADR-0098: Data Governance — Source-Aware Lifecycle and Purge-on-Revocation

**Date:** 2026-02-28
**Status:** Proposed
**Target:** v1.1.0 (prerequisite for I505, I486, I487)
**Extends:** ADR-0094 (Audit Log), ADR-0095 (Dual-Mode Context Architecture)
**Research:** `.docs/research/2026-02-28-health-scoring-research.md`

## Context

### The Problem

DailyOS stores sensitive data locally: meeting transcripts, email content, calendar events, entity intelligence, and person profiles. With Glean integration (v0.15.2), DailyOS can now access the org's full knowledge graph — contact rosters (72 contacts for a single account), org chart data, CRM fields, support history. The question is whether this data should be cached locally, and if so, under what governance.

Glean is governed by design — access controls, audit trails, DLP, retention policies. The org chose Glean partly because data stays within their security perimeter. Copying Glean data into an unmanaged SQLite file on a laptop moves it outside that perimeter.

But DailyOS already stores data of comparable or greater sensitivity:
- Full email bodies with customer names, financials, and commitments
- Meeting transcripts with verbatim conversations
- Calendar events with attendee lists and meeting purposes
- LLM-generated intelligence synthesizing all of the above

Creating a special governance tier for Glean contact data while emails containing the same people's names and contexts sit in the same database would be inconsistent. The real governance gap is not "should Glean data be local" — it's **"what happens to ALL local data when access is revoked."**

### The Risk

Someone leaves the company. Their laptop has:
- Every meeting transcript from the last 6 months
- Email intelligence for every account in their book
- AI-generated strategic assessments of account health
- The full contact roster for every account, pulled from Salesforce via Glean

Today, none of this is purged when they lose access. Google OAuth expires silently. Glean tokens stop working. But the local data persists indefinitely.

## Decision

### Source-Aware Data Lifecycle

Every record in DailyOS tracks its provenance source. When a source's authorization is revoked, all data from that source is purged. This applies uniformly across all data sources — not just Glean.

### Principle 1: Persist Locally, Tag Provenance

All data — including Glean-sourced contacts, org health data, and relationship edges — is stored in the local SQLite database. No ephemeral-only tier. This preserves:
- Offline access (the app works on a plane)
- Cross-query intelligence (stakeholder coverage tracking over time)
- Meeting prep quality (contact context available without re-querying)
- The "intelligence that compounds" thesis

Every record tracks its source via existing provenance mechanisms:
- `enrichment_sources` JSON on `people` table (per-field source tracking)
- `source` column on `signal_events`, `person_relationships`
- `source` field on `entity_intelligence` provenance
- New: `data_source` column on `entity_people` linkage records

Source values: `"user"`, `"google"`, `"glean"`, `"clay"`, `"ai"`, `"co_attendance"`

### Principle 2: Source-Aware Purge on Auth Revocation

When a data source's authorization is revoked (detected by token refresh failure, explicit disconnect in Settings, or OAuth callback revocation), DailyOS purges all data attributed to that source.

| Source Revoked | Data Purged |
|----------------|------------|
| **Google OAuth** | Calendar events, email content, email signals, transcript data, meeting records. Entity intelligence is re-enriched without Google data on next cycle (if other sources remain). |
| **Glean auth** | Glean-sourced person profiles (fields where `enrichment_sources.*.source = "glean"`), Glean-sourced entity_people links, org health data, Glean signals, Glean document cache. |
| **Clay/Smithery** | Clay-sourced person profiles, Clay signals. |
| **User account deletion** | Everything. Full database wipe. |

**What is NOT purged on source revocation:**
- User-entered data (`source = "user"`) — this is the user's own work product
- AI-generated intelligence — this is DailyOS's analysis, not raw source data. However, intelligence that was primarily derived from purged source data should be flagged for re-enrichment
- Entity records (accounts, projects) — the user created these, they persist
- Actions and decisions — the user's work product

**Purge is cascading but respects source priority:** If a person's `role` field has `enrichment_sources.role.source = "glean"` and Glean is revoked, the role field is cleared. But if the user subsequently set the role manually (`source = "user"`), the user's value is preserved — the purge only clears fields still attributed to the revoked source.

### Principle 3: Encryption at Rest

SQLCipher (planned v0.15.1, I462) encrypts the entire database. This covers all locally stored data including Glean-sourced records. Combined with macOS Keychain for token storage, the data-at-rest protection is consistent regardless of source.

### Principle 4: Audit Trail

ADR-0094's audit log already tracks data access events. Extend it to cover:
- Glean data ingestion events (contact roster synced, N records written)
- Source revocation events (auth revoked, N records purged)
- Data export events (what was included in the export ZIP)

### Principle 5: TTL Refresh for External Sources

Glean-sourced data carries a `gathered_at` timestamp. Data older than 30 days without refresh is flagged as stale:
- Stale contact data shows a visual indicator ("Last synced 45 days ago")
- Stale org health data reduces confidence in the health scoring engine (I499)
- Stale data is NOT auto-purged — staleness is informational, not destructive
- Re-sync happens automatically on next enrichment cycle when Glean is available

### Implementation: Purge Mechanics

New module `src-tauri/src/data_lifecycle.rs`:

```rust
pub enum DataSource {
    User,
    Google,
    Glean,
    Clay,
    Ai,
    CoAttendance,
}

pub fn purge_source(db: &ActionDb, source: DataSource) -> Result<PurgeReport, DbError> {
    // 1. Identify all records attributed to this source
    // 2. For people table: clear fields where enrichment_sources.*.source matches
    // 3. For entity_people: delete rows where data_source matches
    // 4. For signal_events: delete rows where source matches
    // 5. For person_relationships: delete rows where source matches
    // 6. For entity_intelligence: flag for re-enrichment (don't delete — re-enrich without the purged source)
    // 7. Log purge event to audit trail
    // 8. Return PurgeReport with counts
}

pub fn detect_revocation(source: DataSource) -> bool {
    // Check if the source's auth token is still valid
    // Google: attempt token refresh
    // Glean: attempt a lightweight search query
    // Clay: attempt Smithery API ping
}
```

Purge is triggered by:
1. **Explicit disconnect** in Settings → Connectors (user clicks "Disconnect Glean")
2. **Token refresh failure** after N retries (detected in background scheduler)
3. **Settings → Data → "Clear [source] data"** (manual purge without disconnecting)

### Implementation: Source Tagging

**entity_people table** — add `data_source` column:
```sql
ALTER TABLE entity_people ADD COLUMN data_source TEXT NOT NULL DEFAULT 'user';
```

**Existing tables already have source tracking:**
- `people.enrichment_sources` — JSON per-field provenance (already exists)
- `signal_events.source` — already tracks signal origin
- `person_relationships.source` — already tracks relationship origin
- `entity_intelligence` — add `sources_used` JSON column tracking which data sources contributed

### What This Means for Glean Data (I505, I486, I487)

With this governance model in place:

1. **I505 (Glean stakeholder intelligence)** can persist contact roster data locally with `data_source = "glean"`. If Glean auth is revoked, all Glean-sourced entity_people links and person profile fields are purged.

2. **I486 (Glean person writeback)** writes profile fields with `source = "glean"` in enrichment_sources. Source priority (User > Clay > Glean > AI) ensures user corrections are never overwritten. Glean fields are purged on revocation.

3. **I487 (Glean signal emission)** emits signals with `source = "glean"`. Purge-on-revocation deletes Glean signals but doesn't cascade to intelligence derived from those signals (intelligence is flagged for re-enrichment instead).

4. **I500 (Glean org-score parsing)** stores OrgHealthData with source provenance. Purge clears the baseline score, and the health engine falls back to computed baseline.

## Consequences

### Positive
- Consistent governance across all data sources — no special cases
- User trust: "disconnect a source and its data is gone" is a clear, understandable promise
- Enables Glean stakeholder intelligence (I505) without governance ambiguity
- Offline access preserved — data is local until explicitly purged
- Builds on existing provenance infrastructure (enrichment_sources, signal source, audit log)

### Negative
- Purge-on-revocation is destructive — accidentally disconnecting Google would wipe calendar/email data. Mitigation: confirmation dialog with data impact summary ("This will remove 1,247 email records and 89 meeting transcripts. Your accounts, intelligence, and personal data will be preserved.")
- Source attribution must be maintained retroactively — existing records lack `data_source` on `entity_people`. Migration needed.
- Intelligence re-enrichment after purge may produce lower-quality results if the purged source was the primary context provider. This is acceptable — the alternative is retaining unauthorized data.

### Risks
- **Accidental purge:** User disconnects Google temporarily for troubleshooting, loses all email data. Mitigation: grace period ("Data will be purged in 7 days unless you reconnect") or soft-delete with recovery window.
- **Incomplete source tracking:** If a record's source isn't tagged (legacy data, migration gaps), it survives purge when it shouldn't. Mitigation: conservative default — untagged data is treated as `"user"` (never auto-purged).
- **Re-enrichment cost:** Purging Glean data and re-enriching without it uses LLM tokens for lower-quality output. Mitigation: skip re-enrichment for accounts with no remaining signals — just clear the intelligence and show empty state.

## Dependencies

This ADR must be implemented (at minimum: source tagging on entity_people, purge_source function, Settings UI for disconnect-with-purge) before:
- I505 (Glean stakeholder intelligence)
- I486 (Glean person writeback) — already in v1.1.0 scope
- I487 (Glean signal emission) — already in v1.1.0 scope

The full purge-on-revocation with grace period and Google OAuth lifecycle can ship incrementally, but the source tagging infrastructure must land first.
