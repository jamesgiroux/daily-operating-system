# DOS-258 — Entity Linking Rewrite (Kickoff)

**Linear:** https://linear.app/a8c/issue/DOS-258
**Target release:** v1.2.1 (slipped to ~2026-04-28 to land with the account-detail-page IA redesign)
**Branch:** `dos-258-entity-linking-rewrite` (to be created from `dev`)

## Read this first

Full spec is in the Linear issue description (v3). It's been through:
1. Investigation (`/investigate`) — found the root cause (fuzzy/keyword primitives + 5 uncoordinated writers).
2. Codex adversarial review — 18 findings, drove the phased-engine refactor (v2).
3. Plan eng review (`/plan-eng-review`) — 13 spec tightens, scored 7.9/10, verdict: SHIP WITH CHANGES. v3.

Do not skim the Linear issue. The acceptance criteria and regression tests are the definition of done. The codex review comment and plan-eng-review comment on the issue contain the "why" for several decisions that look arbitrary in isolation.

## Fixture naming rule (CLAUDE.md)

No customer names in code. All fixtures use: `InternalUser1/2`, `CustomerPerson1/2`, `CustomerAcct`, `OtherCustomerAcct`, `SubsidiaryAcct`, `ParentAcct`, `SiblingAcct`. Domains: `user.com`, `customer.com`, `customer-a.com`, `customer-b.com`, `subsidiary.com`, `parent.com`, `corp.com`, `consulting.com`. Prod-bug validation happens manually against the live encrypted workspace DB, not from checked-in fixtures (acceptance criterion #18).

## Implementation lanes (dependency order)

```
A — Schema migrations                 [start here, blocks everything]
  ↓
B — Service skeleton + primitives     [after A]
  ↓
C — Rule engine + phases              [after B]
  ↓
D — Calendar adapter + strip legacy   ┐
E — Email adapter + strip legacy      │ parallel after C
G — Frontend vocab + pickers          ┘
F — Stakeholder queue UI              [can start after B data contract]
```

Peak parallelism: 3 lanes after C.

## Start here — Lane A

Ship first, reviewable in isolation, zero runtime behavior change:

### Migrations (in this order under `src-tauri/src/migrations/`)

Next migration number: check the highest existing number (currently 109). Use 110+.

1. **`NNN_linked_entities_raw.sql`** — creates `linked_entities_raw` table with CHECK constraints on `owner_type` and `role`, `idx_one_primary` partial unique index (`WHERE role = 'primary'`), and the `linked_entities` view filtering `source != 'user_dismissed'`.
2. **`NNN_linking_dismissals.sql`** — creates `linking_dismissals` table.
3. **`NNN_entity_linking_evaluations.sql`** — creates `entity_linking_evaluations` append-only audit table.
4. **`NNN_entity_graph_version.sql`** — creates singleton counter table + seed row + triggers on `account_domains`, `account_stakeholders`, `entity_keywords` (accounts + projects both), `projects.account_id` UPDATE. Each trigger: `UPDATE entity_graph_version SET version = version + 1 WHERE id = 1;`.
5. **`NNN_account_stakeholders_review_queue_idx.sql`** — adds `(account_id, status)` index on `account_stakeholders`.
6. **`NNN_migrate_meeting_entity_dismissals.sql`** — backfill existing `meeting_entity_dismissals` rows into `linking_dismissals` with `owner_type='meeting'`. Keep old table until the N+1 cutover (separate later migration).

See Linear issue for full SQL. Use exact schema from the "Data model" section.

### Scaffold (no logic yet)

`src-tauri/src/services/entity_linking/mod.rs` — empty module declaration. Creates the directory structure so Lane B lands cleanly.

`src-tauri/src/services/entity_linking/repository.rs` — scaffold:

```rust
//! Raw migration/backfill API. NOT for runtime callers.
//! Production code importing this module fails the pre-commit grep hook.

pub fn raw_rebuild_account_domains(_db: &crate::db::ActionDb) -> Result<(), String> {
    // TODO(Lane-A): implement trusted-source rebuild. See DOS-258 "account_domains trust rebuild".
    unimplemented!("Lane A completion task")
}
```

Add `pub mod entity_linking;` to `src-tauri/src/services/mod.rs`.

### Pre-commit hook (scope bypass enforcement)

Add to `.git/hooks/pre-commit` or document in CONTRIBUTING — a grep check that fails if `services::entity_linking::repository` is imported from any file outside `services/entity_linking/` or `tests/`. Can be added later; note it in a follow-up TODO.

### Lane A acceptance

- `cargo test` green
- `cargo clippy -- -D warnings` green
- `pnpm tsc --noEmit` green
- Migrations run cleanly on a fresh DB AND on an existing DB (copy `~/Library/Application Support/com.dailyos.app/dailyos.db` to a throwaway location and test against it)
- No behavioral change (new tables exist but nothing reads/writes them yet)

Commit message convention: `feat(DOS-258): Lane A — entity linking schema`.

## Baseline capture (parallel with Lane A)

Before any code lands, capture a snapshot of the current resolver's output so we have something to diff against later:

```bash
# From repo root, after pulling latest dev:
cargo run --bin baseline-entity-linking -- \
  --db ~/Library/Application\ Support/com.dailyos.app/dailyos.db \
  --days-meetings 90 --days-emails 30 \
  --output .docs/migrations/entity-linking-baseline.json
```

If that binary doesn't exist yet, write a quick one-off script in `src-tauri/examples/baseline_entity_linking.rs` that iterates the last 90d meetings + 30d emails and dumps `(id, primary_entity_id, primary_entity_type, source)` per row. ~100 lines.

Commit the JSON under `.docs/migrations/` (gitignored if it's large — check size first). This becomes the baseline for the `entity-linking-v2-diff.md` dry-run in Lane D/E.

## Resources

- **Linear:** DOS-258 full spec + comments (codex review + plan-eng-review).
- **ADRs:** 0100, 0101, 0105, 0107, 0113, 0114, 0115, 0118 all touch this work. ADR-0114 is explicitly superseded for this surface.
- **Existing primitives to re-export (not rewrite):**
  - `find_or_create_person_by_email` — `src-tauri/src/db/people.rs:317`
  - `get_person_by_email_or_alias` — `src-tauri/src/db/people.rs:185`
  - `lookup_account_candidates_by_domain` — `src-tauri/src/db/accounts.rs`
- **Current broken code (to remove/gut, not read for patterns):**
  - `src-tauri/src/prepare/entity_resolver.rs` — fuzzy/keyword signals to delete
  - `src-tauri/src/services/meetings.rs:490` — `cascade_meeting_entity_to_people` to remove
  - `src-tauri/src/services/meetings.rs:~1628-1774` — five writer functions to collapse into service
  - `src-tauri/src/services/emails.rs:1005` — `update_email_entity` direct writer to delegate

## Don't

- Don't start Lane C before Lane B compiles.
- Don't write stakeholders to `account_stakeholders` from anywhere outside `services::entity_linking::confirm_stakeholder_suggestion`. That's C2's whole point.
- Don't use name-based matching anywhere in the linking path. Only email.
- Don't skip the dry-run diff step before flipping the feature flag.
- Don't carry prod names (real customer / contact / company names) into tests or comments.

## First-session kickoff prompt

Paste this into a fresh Claude Code session in this repo:

> I'm starting work on DOS-258 — the entity linking rewrite. Full spec is at
> `.docs/plans/DOS-258-entity-linking-rewrite.md` and in Linear issue DOS-258
> (use the Linear MCP to read the full issue description, the codex review
> comment, and the plan-eng-review comment before starting — they contain
> critical context not in the kickoff doc).
>
> Please do the following in order:
> 1. Read the kickoff doc and the Linear issue in full. Confirm back to me
>    the total migration count for Lane A (should be 6) and the branch name
>    you'll use.
> 2. Check that `dev` is clean of uncommitted in-progress work OR that any
>    dirty files do not overlap with Lane A's scope. If there's a conflict,
>    stop and ask.
> 3. Create branch `dos-258-entity-linking-rewrite` from `dev`.
> 4. Implement Lane A in full per the kickoff doc: six migrations + service
>    scaffold. No behavioral change.
> 5. Verify Lane A acceptance: `cargo clippy -- -D warnings && cargo test &&
>    pnpm tsc --noEmit` all clean, migrations run cleanly on a fresh DB.
> 6. Commit with message `feat(DOS-258): Lane A — entity linking schema`.
> 7. Run the baseline-capture script per the kickoff doc, commit the output
>    to `.docs/migrations/entity-linking-baseline.json` if size permits.
> 8. Stop and report. Do NOT start Lane B until I confirm.
>
> Use fixture names only (see kickoff doc's "Fixture naming rule"). No prod
> customer names in any code or comment. If at any point you hit
> confusion/ambiguity, stop and ask rather than guess on data model or
> architecture. Auto mode is fine for routine decisions.
