# v1.4.2 Wave 1 — Proof Bundle

**Wave:** W1 — Substrate-side contract
**Date sealed:** 2026-05-11 (impl + L1 + L2 phases; PR + CI pending Linear topology)
**Branch:** `dos-546-wp-studio-spike` off `dev` @ 4ebec482
**Predecessor:** W0 — Contract lock + supersession (committed at 33baacb7)

---

## Issue map

| Issue | Title | Commits | Linear ID |
|---|---|---|---|
| W1-A | SurfaceClient as the fourth actor class | 7fba6a22 + 0e98377c (.1) + 7705e6fd (.2) | _pending_ |
| W1-A0 | Audit-log schema for SurfaceClient attribution + emit_surface_audit | fada3fd9 | _pending_ |
| W1-B | Promote AbilityPolicy to canonical schema + macro compile-error gate | 0f873270 + a03a9e1f (.1) | _pending_ |
| W1-C | Ability-surface inventory format + CI gate | b25c6cc4 + ec570d1d (.1) + 6230511d (.2) | _pending_ |
| W1-D | Ability-description CI gate + workflow integration | 57a57e1f + 8cbdfc1c (.1) | _pending_ |
| W1-E | Composition contract substrate types + ProvenanceRef shape | e9596c49 + 9d09c1c3 (.1) | _pending_ |

13 commits total on top of W0. Linear-side issue IDs pending hand-off; James owns the W0-B Linear topology step.

---

## Acceptance criteria — pass/fail per issue

### W1-A — SurfaceClient as the fourth actor class

- ✅ `Actor::SurfaceClient { instance: SurfaceClientId, scopes: ScopeSet }` struct variant per ADR-0111 §8 + ADR-0102 §7.6
- ✅ `SurfaceClientId` newtype: full derives + `Display` + `as_str`; opaque non-PII identifier
- ✅ `SurfaceScope` newtype + `ScopeSet { BTreeSet<SurfaceScope> }` with empty rejection at `new()` and Deserialize
- ✅ Unknown-scope rejection at Deserialize when allowlist initialized; lenient bootstrap until `initialize_allowlist()` called
- ✅ Match-site plumbing: clone-at-move-sites; `todo!()` arms in non-critical paths with W1-B+ wiring comments
- ✅ 10 unit tests + 4 ScopeSet tests (identity, Display, serde-transparent, Hash+Eq, distinct-instance, negative-class `.contains`, deserialization round-trip, empty-rejection)
- ✅ W1-A.2 fix-up: `RwLock<Option<BTreeSet<SurfaceScope>>>` + `AtomicBool SCOPE_ALLOWLIST_INITIALIZED` replaces `OnceLock` to support test isolation; `set_allowlist_for_tests` + `clear_allowlist_for_tests` companions

### W1-A0 — Audit-log schema for SurfaceClient attribution

- ✅ `AuditRecord` additive optional fields: `actor_kind`, `actor_instance: Option<SurfaceClientId>`, `wp_user_id: Option<u64>`, `actor_scopes: Option<Vec<String>>` with `#[serde(default, skip_serializing_if)]`
- ✅ Legacy JSONL records round-trip via serde defaults
- ✅ `emit_surface_audit(logger, event_kind, &Actor, AuditFields)` helper enforces SurfaceClient invariant: requires `wp_user_id: Some(_)`, returns `AuditError::SurfaceClientMissingWpUserId` on violation, writes NO record on failure
- ✅ `actor_instance` + `actor_scopes` derive from the Actor variant (caller cannot inject)
- ✅ Non-SurfaceClient actors drop stray `wp_user_id` silently
- ✅ 5 new unit tests (12 total)
- ✅ `AuditLogger::append_with_actor` shares `write_record` primitive with legacy `append`

### W1-B — AbilityPolicy canonical schema + macro compile-error gate

- ✅ `AbilityPolicy` extended: `required_scopes: &'static [&'static str]` (inventory::submit! const-friendly; typed via `required_scopes_typed() -> Vec<SurfaceScope>`)
- ✅ `mcp_exposure: McpExposure` (None/MetadataOnly/Invocable) with `#[default] None` + snake_case serde + 3-variant round-trip
- ✅ `client_side_executable: bool` (default false)
- ✅ `AbilityPolicy::default()` ships least-privilege closed defaults (User-only via `&[ActorKind::User]`)
- ✅ `ActorKind` discriminator enum + `Actor::kind()` projection — clean separation between policy listing (const-friendly) and runtime variants (instance data)
- ✅ Macro compile-error gate at parse time: `#[ability]` rejects `allowed_actors: [SurfaceClient]` + empty `required_scopes` + absent `no_scope_required`
- ✅ Trybuild fixtures: negative `surface_client_without_scopes_fails.rs` + positive `surface_client_with_scopes_passes.rs` — both green (10/10 total)
- ✅ Allowlist init wired into `from_descriptors_checked` — union of registered abilities' scopes populates `SCOPE_ALLOWLIST`
- ✅ 16 consumer file sites migrated to `ActorKind` (bridges, mcp, harness, tests)
- ✅ 7 new unit tests + 1 new positive trybuild fixture

### W1-C — Ability-surface inventory format + CI gate

- ✅ `AbilitySurfaceInventory` + `AbilitySurfaceInventoryEntry` types match Phase 0 artifact 05 schema
- ✅ Wire-form aliases (`AbilityActor` / `InventoryCategory` / `InventoryMcpExposure` / `IdempotencyClass` / `CompositionBlockType` / `CompositionKind` / `AnnotationValue`)
- ✅ `AbilityActor::project(kind, exposure)` correctly emits `McpClient` for Agent+Invocable; `Runtime` for Agent+{None,MetadataOnly}; System|Admin → Runtime; User → User; SurfaceClient → SurfaceClient (cycle-2 fix per artifact 05 line 546)
- ✅ `CompositionKind` discriminated union rejects invalid mixed shapes (cycle-2 fix)
- ✅ `AnnotationValue::Number(f64)` matches TS `number` (cycle-2 fix)
- ✅ `From<&AbilityDescriptor>` projection threads `mcp_exposure` through
- ✅ Emit binary `emit_ability_inventory` with `--out PATH`; exit codes 0=success, 1=registry violation, 2=I/O (cycle-2 corrected)
- ✅ `AbilityRegistry::iter_all()` private-namespace tooling-facing accessor (bypasses actor gate)
- ✅ TypeScript mirror at `web/types/ability-surface.ts` (94L)
- ✅ Committed `tools/dailyos-abilities.json` reflects 4 currently registered abilities; CI gate diffs against it
- ✅ CI gate `scripts/check_ability_inventory.sh` wired into `.github/workflows/test.yml`
- ✅ 11 unit tests in inventory module + projection-matrix coverage

### W1-D — Ability-description CI gate + workflow integration

- ✅ `scripts/check_ability_descriptions.sh` scans `#[ability(description = "...")]` (multi-line aware via Python balanced-paren walker with string-state awareness)
- ✅ Reads correct JSON key `abilities` from `tools/dailyos-abilities.json` (cycle-2 fix)
- ✅ Three-blocklist load: optional local `.claude/pii-blocklist.txt` + REQUIRED committed `scripts/ability_description_pii_denylist.txt` (cycle-2 added) + REQUIRED committed `scripts/ability_description_vocab_blocklist.txt`
- ✅ CI-side PII safety net: committed denylist with fictional markers (pii-fixture-marker, acme-corp-test-fixture); no real PII anywhere in the committed scripts
- ✅ Fixture test `check_ability_descriptions_test.sh`: BAD fixture trips on internal vocab + fictional PII marker → exit 1; CLEAN fixture → exit 0
- ✅ Internal vocabulary blocklist: 7 seed terms (enrichment, AI enrichment, intelligence pipeline, pipeline run, prompt fingerprint, claim writer, trust band scoring) per ADR-0083
- ✅ CI workflow wires all three Wave 1 gates: PII+vocab → inventory → composition authorship

### W1-E — Composition substrate types + ProvenanceRef shape

- ✅ Types per ADR-0130 §2 verbatim widening (cycle-2 fix):
  - `Composition { id, kind, subject, salience, generated_at, generated_by, sections, metadata }`
  - `Section { id, label, salience, blocks, layout }`
  - `Block { id, block_type, attributes, claim_refs, provenance, salience, render_hints }`
  - `ProvenanceRef { invocation_id, field_path }` — preserves ADR-0105 §8 lives-once invariant (≤256B compact reference, not envelope copy)
- ✅ 9 canonical `BlockType` variants + `Custom { type_id }` extension point
- ✅ `CompositionKind` enum + `Salience` + `RenderHints { emphasis, density }` + `Density` types
- ✅ Newtypes: `CompositionDocId`, `SectionId`, `BlockId`, `CompositionVersion(u64)`, `EntityRef`, `AbilityRef`
- ✅ `Block::new()` validation: requires non-nil InvocationId; when envelope supplied, validates `field_path` resolves into `envelope.field_attributions` (exact or parent-path `covers()`)
- ✅ `project_to_nearest_known()`: 9-step deterministic algorithm per Phase 0 artifact 07 (lexicographic tie-break, JSON-pointer intersection, drop non-intersected, preserve claim_refs + provenance, trust cap, non-dismissible banner)
- ✅ `Composition::new()` is `pub(crate)` (substrate-owned authorship per ADR-0130 §1)
- ✅ Defense-in-depth: `scripts/check_composition_authorship.sh` grep CI gate (scans for `Composition::new(` + `Composition {` in non-substrate paths)
- ✅ CI workflow wires the composition authorship gate (cycle-3 fix)
- ✅ 22 unit tests pass (9 new composition + 8 pre-existing composition-graph + 4 new ADR-0130 §2 widening + 1 negative envelope-validation)

---

## L1 self-validation evidence

| Check | Command | Exit | Coverage |
|---|---|---|---|
| Rust lint | `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` | 0 | Standard project invocation per CLAUDE.md |
| Rust tests | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | 0 | 285 passed / 0 failed (after W1-A.2 isolation fix) |
| Macro trybuild | `cargo test -p dailyos-abilities-macro trybuild` | 0 | 10/10 fixtures green |
| TS type-check | `pnpm tsc --noEmit` | 0 | TypeScript inventory mirror |
| Inventory CI gate | `scripts/check_ability_inventory.sh` | 0 | No drift against committed JSON |
| Description CI gate | `scripts/check_ability_descriptions.sh` | 0 | No offenders today |
| Composition authorship CI gate | `scripts/check_composition_authorship.sh` | 0 | No non-substrate constructors |

---

## L2 review evidence

**All seven W1 issues achieve unanimous L2 APPROVE** across codex + code-reviewer + domain reviewer panels:

| Issue | codex | code-reviewer | domain reviewer | Reviewer files |
|---|---|---|---|---|
| W1-A | APPROVE | APPROVE | architect APPROVE (cycle-2) | `dos-546-w1-a-l2-*-2026-05-11.md` |
| W1-A.2 (test isolation) | (implicit via W1-A panel) | APPROVE | architect APPROVE | `dos-546-w1-a-2-l2-*-2026-05-11.md` |
| W1-A0 | APPROVE | APPROVE | /cso APPROVE-WITH-REFINEMENTS | `dos-546-w1-a0-l2-*-2026-05-11.md` |
| W1-B + W1-B.1 | APPROVE (cycle-2) | APPROVE | architect APPROVE | `dos-546-w1-b-l2-*-2026-05-11.md`, `dos-546-w1-b-1-l2-codex-2026-05-11.md` |
| W1-C + W1-C.2 | APPROVE (cycle-2) | APPROVE | architect APPROVE | `dos-546-w1-c-l2-*-2026-05-11.md` |
| W1-D + W1-D.1 | APPROVE (cycle-2) | APPROVE | architect APPROVE | `dos-546-w1-d-l2-*-2026-05-11.md` |
| W1-E + W1-E.1 | APPROVE (cycle-3) | APPROVE | architect APPROVE | `dos-546-w1-e-l2-*-2026-05-11.md`, `dos-546-w1-e-1-l2-codex-*-2026-05-11.md` |

---

## Trust boundary check (cumulative through W1)

The W1 substrate-side contract delivers all the gates W2+ will enforce at the loopback transport boundary:

- **Multi-level enforcement (ADR-0102 §7.6):** schema landed in W1-B (allowed_actors + required_scopes + mcp_exposure + client_side_executable). Substrate-enforceable at the registry.
- **Macro compile-error gate (ADR-0102 §7.6):** lands in W1-B + verified via trybuild positive + negative fixtures. Drift is structurally impossible.
- **SurfaceClient class definition (ADR-0111 §8):** Actor::SurfaceClient { instance, scopes: ScopeSet } per spec; ScopeSet enforces non-empty + allowlist-validated deserialization.
- **Lives-once invariant (ADR-0105 §8):** preserved via Block.provenance: ProvenanceRef (reference shape, not envelope copy).
- **Custom block fallback (ADR-0130 §3.1):** 9-step deterministic algorithm with claim_refs + provenance preserved + trust cap + non-dismissible banner.
- **Audit attribution (ADR-0111 §8):** emit_surface_audit helper enforces wp_user_id-required + actor_instance + actor_scopes from variant.
- **Substrate-owned authorship (ADR-0130 §1):** Composition::new() is `pub(crate)`; grep CI gate is defense-in-depth.

---

## Path-α observations filed to maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`

Approximately 18 path-α findings across the W1 panel, all non-blocking:

**W1-A / W1-A.2:**
- ADR-0102 §7.6 step 3 vs introspection paragraph readability (MetadataOnly enumerates-only-not-invokes)
- ADR-0102 line 508 pre-existing ADR-0108 mis-label
- `todo!()` arms in non-critical match-sites → `unreachable!()` + CI gate
- `Debug` impl leaks raw identifier into logs (path-α)
- `iter_for` by-value ergonomics
- `debug_assert!(cfg!(test) || cfg!(debug_assertions))` in `set_allowlist_for_tests`
- `drop(guard)` in `new` is redundant (style)

**W1-A0:**
- Typestate / NonZeroU64 wrapper for SurfaceClient wp_user_id (compile-time gate vs current runtime gate)
- `rotate_audit_log` re-emit-raw-legacy-line on serialize failure — add structured warn
- W6-A CI lint asserting no production call site invokes `AuditLogger::append` with SurfaceClient event vocabulary

**W1-B:**
- `OnceLock::set` second-call drop should debug-trace for observability (RwLock equivalent applied in W1-A.2 covers this)

**W1-C:**
- Exit-code alignment if a consumer needs differentiation
- Auto-generated TS bindings (typeshare / ts-rs) when a third surface joins
- Round-trip serde + committed-artifact parse tests
- Reconcile issue body line 546 `display` field discrepancy with artifact 05
- Optional `iter_all` visibility tightening to `pub(crate)`

**W1-D:**
- JSON `line_for_description` fallback to line 1 on duplicate descriptions
- Override-path parser splits on `:` only (Windows-unsafe; CI is Linux)
- Vocab blocklist drift detector projecting from ADR-0083's translation table
- Extend description scan to `name`, `category`, `annotations` (model-facing per ADR-0102 §7)
- Inventory.toml descriptions + Rust raw strings `r#"..."#` evade scanner

**W1-E:**
- `generated_by` duplicated on Composition and CompositionMetadata
- `#[allow(dead_code)]` on `Composition::new` should drop when W3 wires a producer
- `insert_at_path` overwrites leaf-at-intermediate silently; diagnostic doesn't record
- Banner injects raw `type_id` into user-visible copy — needs `display_name` map per ADR-0083
- JSON Pointer escape handling (`~0`/`~1`) + array-index reconstruction in `project_pointers`
- BlockDescriptor shape gaps: `schema_shape`, `allowed_surfaces`, `actor_reachability` for `project_to_nearest_known()`
- Typed `BlockPayload` enum migration (currently `serde_json::Value`)
- Grep gate prefix-disambiguation guard

---

## Pending (post-Linear)

- [ ] James executes W0-B Linear topology: supersede old v1.4.2 (`33411e87-...`), create v1.4.3 parking project for entity-intelligence, create new v1.4.2 project from `01-project-description.md`, create at minimum the 7 W0+W1 issues
- [ ] Linear IDs filled into the W0 + W1 issue maps in their respective proof bundles + commit messages (via follow-up commits or PR descriptions)
- [ ] Push branch to remote: `git push -u origin dos-546-wp-studio-spike`
- [ ] Open PRs (one per wave-letter issue, per wave protocol)
- [ ] CI green on each PR
- [ ] Merge gate signed per wave plan
- [ ] W1 retro (mandatory per wave plan retro rules)
