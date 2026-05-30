# W4-A L0 packet - dailyos/account-overview Gutenberg block renderer

## 1. Header

Date: 2026-05-13
Project: v1.4.2 - Personal Intelligence Engine: WordPress Foundation
Parent: DOS-546
Wave: W4-A, stage-3 renderer
Issue: DOS-572
Surface: WordPress Gutenberg dynamic block
Block name: `dailyos/account-overview`
Output path: `wp/dailyos/blocks/account-overview/`
Primary runtime dependency: W3-B `DailyOS_Runtime_Client`
Primary producer dependency: W4-A0 `AbilityOutput<Composition>`
Primary projection dependency: W4-D `project_composition_for_surface(composition, ctx)`
Primary trust-boundary dependencies: W4-B, W4-C, W4-E
Downstream dependencies: W5-A feedback router, W5-B magazine theme, DOS-589 outbox delivery

This packet captures the W4-A renderer contract resolved at L0.
The Linear issue remains the canonical execution contract.
This packet supersedes it only where the issue leaves a boundary implicit.
The implementing agent should treat W4-A as thin PHP/JS rendering glue.
The block must not become a second producer, trust scorer, signer, projector, nonce issuer, or feedback persistence layer.

DailyOS product framing applies.
The block renders account context with trust, provenance, and visible uncertainty.
It must feel editorial under the DailyOS theme and remain legible under stock WordPress themes.
It must not expose internal vocabulary in user-facing strings.

## 2. Changelog

- **V3.2 (2026-05-13):** Cycle 4 codex confirmation pass — two contract drifts closed:
  - V2 changelog entry at line 47 marked SUPERSEDED BY V3 (V2 wp_cache + scope_set_hash mechanism replaced by V3 runtime-side cache + opaque cache_hint_token). The cached_projection-removal property is preserved; the V2 mechanism is replaced.
  - §6.3.1 W3-B method signature + AC #13 now name `cache_hint_token` round-trip explicitly: optional 4th parameter on the PHP-side wrapper, return shape is `['projection' => ProjectedCompositionDTO, 'cache_hint_token' => string]`. This closes the contract anchor for the V3 §6.2 round-trip.

- **V3.1 (2026-05-13):** Cycle 3 codex confirmation pass — class-wide sweep of remaining stale cache-class references after V3 named-section fix landed. Class fixes:
  - §6.3 step 3: PHP no longer "checks `wp_cache`"; render.php calls the runtime on every render (PHP-side cache fully removed). Step 4 shows the runtime as cache authority with `cache_hint_token` round-trip.
  - §6.3 audit-emission paragraph deduplicated; the duplicate V2-era paragraph that said "Cache hit → emit projection_cache_served audit (no runtime call)" removed — runtime is sole emitter.
  - §6.7 editor preview/refresh rewritten: every preview render calls the runtime; cache lives substrate-side; failed refresh keeps DOM stale-marked without a PHP-side cached projection.
  - AC #44 (editor reload) rewritten: success/failure paths describe `watermarks` + optional `cache_hint_token`; explicitly no `cached_projection` attribute exists; PHP serves no local cached projection.
  - CI invariant #21 rewritten as a CI grep banning `cached_projection`/`actor_scope_fingerprint`/`actor_context_hint` attributes from `edit.js`/`render.php`.
  - Open Question #6 resolved: runtime-side cache is canonical; no PHP-side cache and no WP transient; substrate is sole scope-identity authority.

- **V3 (2026-05-13):** Cycle 2 fold — codex BLOCK verdict. Material changes:
  - **CRITICAL fix (codex C2):** Acceptance criteria fully reconciled with V2 §6.2 — removed all residual `cached_projection` / `actor_context_hint` / `actor_scope_fingerprint` references from AC list, fixture names, and CI invariants. AC #45 reload expectation now describes `watermarks` + optional `cache_hint_token` (NO `cached_projection`). Fixture #34 renamed to `dos572_fixture_runtime_cache_hit_drops_raw_error.php`.
  - **HIGH fix (codex C2):** AC #13-20 PHP render path tightened — PHP calls `DailyOS_Runtime_Client::project_composition_for_surface($composition_id, $composition_version, $actor_ctx)` directly; never receives `AbilityOutput<Composition>`, never reaches into W4-D directly. Eliminates AC-vs-§6.3 drift codex flagged.
  - **HIGH fix (codex C2):** §6.2 cache layer moved RUNTIME-SIDE (V2 attempted a PHP `wp_cache` keyed by `scope_set_hash`; codex correctly flagged this as impossible because §6.12 forbids PHP from asserting scope identity — the substrate is the sole scope authority). V3: runtime owns the cache keyed by `(composition_id, composition_version, scope_set_canonical_id)` derived inside the substrate from the authenticated SurfaceClient session. Runtime returns an opaque `cache_hint_token` (non-secret, advisory only); PHP echoes it back so the runtime can short-circuit cache lookup. Block attribute schema (§6.1) carries `cache_hint_token` instead of `actor_scope_fingerprint` — no scope material in `post_content`.
  - **Class-wide sweep:** all attribute discipline negative rules now also forbid scope-derived material (hashes, fingerprints, scope set identifiers). The post_content scope-leak class is closed at the schema layer, not by case-by-case patching.

- **V2 (2026-05-13):** Cycle 1 fold — 5 reviewer panels (eng + cso + devex + ui-designer CONDITIONAL APPROVE, codex BLOCK). Material changes:
  - **CRITICAL fix (codex C1; SUPERSEDED BY V3 — see V3.1 above):** `cached_projection` REMOVED from block attributes (was a scope-leak vector — block attributes serialize into publicly-readable `post_content`). V2 placed perf cache in PHP `wp_cache` keyed by `(composition_id, composition_version, scope_set_hash)`. Codex Cycle 2 BLOCK correctly flagged this approach as impossible since §6.12 forbids PHP from asserting scope identity (substrate is sole scope authority). V3 supersedes this: cache lives RUNTIME-SIDE; block attributes carry only `cache_hint_token` (opaque, advisory); no PHP-side cache, no scope-derived material in `post_content`. The cached_projection-removal property is preserved; the V2 wp_cache mechanism is REPLACED by the V3 runtime-side cache.
  - **HIGH fix (codex):** §6.3 PHP→W4-D contract — PHP does NOT call W4-D directly. W3-B exposes new `DailyOS_Runtime_Client::project_composition_for_surface()` method wrapping the Rust API; PHP renders the DTO only.
  - **HIGH fix (codex):** §6.7 enforcement_mode trust-gate — explicit rules per `enforce`/`shadow`/`disabled` plus tamper/rollback/missing-signature states. Trusted affordances only when `enforce` + verification + currentness all pass.
  - **HIGH fix (codex):** §6.1 block.json `attributes` schema pinned verbatim with per-field types/defaults; explicitly no `cached_projection`, no claim text, no provenance.
  - **/cso H1 (closed by C1):** cached_projection scope-revalidation — closed by dropping the attribute entirely.
  - **/cso MEDIUM:** PHP escaping discipline ac + CI grep; audit emission split (`ability_invoked` vs `projection_cache_served`).
  - **/cso LOW:** §6.12 renderer never asserts wp_user_id — only forwards SurfaceClient session token; substrate is sole authority.
  - **Eng P1×3:** block-registration hook ownership pinned (`dailyos_register_blocks()` from `init`); stale banner version state surface pinned (`wp_options.dailyos_composition_versions[composition_id]`); W3-B method signature referenced via DOS-565 Linear, not hardcoded.
  - **Eng P2:** server-render perf budget p95 ≤500ms cold / ≤200ms warm.
  - **Devex P1×2:** build toolchain pinned (`@wordpress/scripts`); PHP testing harness pinned (`wp-env` + `Brain\Monkey`).
  - **Devex P2:** W3-B contract §6.3.1; editor dev loop `wp-scripts start` HMR; account selection via `InspectorControls` + `ComboboxControl`; nonce helper from W4-E PHP layer.
  - **UI-design P1×3:** §6.5 binds to `AboutThisIntelligencePanel` pattern; §6.6/§6.7 banners bind to `StaleReportBanner`/`ConsistencyFindingBanner` (or new `VerificationBanner` variant); §6.4 full `data-ds-*` attributes per `TrustBandBadge`.
  - **UI-design P2×3:** editorial-vs-dashboard ac; FinisMarker; empty state §6.13.
  - **UI-design P3:** copy revisions to magazine voice.

- **V1 (2026-05-13):** Initial L0 plan packet for the first DailyOS Gutenberg block.
- Captures dynamic block shape, file layout, render path, editor preview path, and fallback behavior.
- Inherits W4-B V9 concurrency and version contracts, including `commit_composition`, `ClaimRef::with_field`, `Block.field_bindings`, `BridgeSurfaceError` precedence, `wp_user_id` session binding, SurfaceClient route ownership, and `version_events`.
- Inherits W4-A0 V5 producer shape, claim-type to `BlockType` mapping, and trust-band fallback table.
- Inherits W4-C V3 projection envelope, Ed25519 verification-on-read, quarantine rendering, and shadow-trust enforcement flag.
- Inherits W4-D V3 composition-level projection API and unknown-block fallback rules.
- Inherits W4-E V2 nonce issue/verify endpoints for feedback affordances.
- Inherits W3-B runtime client and HMAC signer path.
- Records that the current W4-A prep worktree has no `wp/dailyos` plugin directory yet.
- Records that `src/styles/design-tokens.css` already ships the three trust-band color aliases.
- Records that runtime components already expose `TrustBandBadge`, `TrustBand`, `ProvenanceTag`, and design-system metadata conventions.
- Establishes no commit, no migration, and no new substrate schema ownership for W4-A.

## 3. Status Snapshot

- W4-A is W4 stage-3: a renderer after stage-1/stage-2 substrate contracts land.
- W4-A is gated on W4-A0 merging because the renderer needs the real `AbilityOutput<Composition>` producer.
- W4-A is gated on W4-C merging because signed projection state and quarantine rendering are trust-boundary inputs.
- W4-A is gated on W4-D merging because the block consumes `ProjectedComposition`, not raw `Composition` blocks.
- W4-A is gated on W4-E merging before feedback affordances are enabled.
- W4-A is gated on W3-B merging because PHP must call the runtime through the existing HMAC-signed client.
- W4-B V9 is a hard dependency for composition watermarks, `version_events`, and SurfaceClient session binding.
- DOS-589 is an interlock for delivering version events into WordPress stale-detection state.
- W5-A consumes W4-A save-handler attribute changes and edit-route metadata for feedback routing.
- W5-B supplies the full DailyOS magazine theme shell; W4-A must still render safely without it.
- The current target worktree contains `.docs/plans/dos-546/v1.4.2-project/` but no W4-A packet file yet.
- The current target worktree contains no `wp/dailyos/` directory.
- Therefore W4-A implementation will create the first block directory and likely the first concrete block registration path.
- That creation does not authorize inventing a parallel plugin architecture.
- W3-A/W3-B plugin registration conventions remain the owner for main plugin bootstrap.
- W4-A only adds the account-overview block files and registration hook required for this block.
- No database migration is planned for W4-A.
- No Rust schema change is planned for W4-A.
- No new ability is planned for W4-A.
- No new `BlockType` is planned for W4-A.
- No new trust-band vocabulary is planned for W4-A.

## 4. Pre-work - substrate reuse audit + WP block conventions

### Substrate reuse audit

- `wp/dailyos/` does not exist in the target W4-A prep worktree.
- W4-A should create `wp/dailyos/blocks/account-overview/` only when implementation begins.
- W4-A should not create a broad plugin framework unless W3-A/W3-B did not already define one on merge.
- W4-A must inspect merged W3-A/W3-B plugin bootstrap before coding.
- Block registration must attach to the plugin owner established by W3-A/W3-B.
- Existing DailyOS design tokens live in `src/styles/design-tokens.css`.
- Existing trust aliases are `--color-trust-likely-current`, `--color-trust-use-with-caution`, and `--color-trust-needs-verification`.
- Existing trust alpha aliases exist at 8, 10, 12, and 15 stops.
- Existing design docs mirror those tokens in `.docs/design/tokens/color.md`.
- Existing `TrustBandBadge` exposes the three visible wire bands.
- Existing `TrustBandBadge` uses product labels `Likely current`, `Use with caution`, and `Needs verification`.
- Existing `TrustBandBadge` exposes `data-ds-name`, `data-ds-tier`, and `data-ds-spec`.
- Existing `TrustBand` composes trust, provenance, and freshness for native app surfaces.
- Existing `ProvenanceTag` exposes design-system metadata and source display conventions.
- Existing `MagazinePageLayout`, `FolioBar`, `FloatingNavIsland`, `AtmosphereLayer`, and `FinisMarker` define the DailyOS magazine shell.
- WordPress rendering cannot import the React app components directly.
- WordPress CSS should mirror token semantics and product copy rather than duplicating React internals.
- Under the DailyOS theme, CSS variables should resolve to theme-provided shell tokens.
- Under stock themes, W4-A must define safe local fallbacks for paper, text, rule, account, and trust colors.
- No customer-specific data may appear in fixtures, snapshots, comments, or examples.

### WordPress block conventions

- The block uses WordPress block metadata in `block.json`.
- The block uses `apiVersion: 3`.
- The block is dynamic and server rendered.
- `block.json` points `render` to `file:./render.php`.
- `save.js` returns `null`.
- PHP registration uses `register_block_type()` against the block directory or metadata.
- Editor markup uses `useBlockProps()`.
- Front-end PHP markup uses `get_block_wrapper_attributes()`.
- Attributes are stored in the block comment delimiter, not in rendered HTML.
- `supports.html` is false so generated HTML is not user-edited.
- W4-A does not use InnerBlocks for DailyOS-authored child content in v1.
- W4-A may allow adjacent free-form WordPress blocks through normal editor behavior.
- W4-A does not store full ability output JSON in `post_content`.
- W4-A does not store raw provenance envelopes in `post_content`.
- W4-A does not store HMAC material, nonce material, or runtime secrets anywhere in attributes.
- W4-A does not expose direct browser-to-runtime calls.
- Browser JS calls WordPress REST or editor preview routes.
- PHP calls the runtime through the W3-B client.
- The editor preview must remain close to front-end output.
- Registration and asset handles should follow WordPress metadata loading instead of manually enqueuing global assets.
- Any REST preview endpoint must use WordPress capability checks plus DailyOS SurfaceClient scope checks.

## 5. What W4-A authors net-new

| Surface | Status | Authoring scope |
|---|---|---|
| `wp/dailyos/blocks/account-overview/block.json` | Missing | Metadata for `dailyos/account-overview`, attributes, dynamic render, editor/front assets |
| `wp/dailyos/blocks/account-overview/render.php` | Missing | PHP render adapter from attributes to runtime invocation to projected HTML |
| `wp/dailyos/blocks/account-overview/edit.js` | Missing | Gutenberg editor preview, account selection handoff, reload button, preview state |
| `wp/dailyos/blocks/account-overview/save.js` | Missing | Null save for dynamic block |
| `wp/dailyos/blocks/account-overview/style.scss` | Missing | Front-end editorial styling with DailyOS token fallbacks |
| `wp/dailyos/blocks/account-overview/editor.scss` | Missing | Editor-only chrome, loading, preview, inspector affordances |
| Block registration hook | Missing or pending W3 merge | Register this block via the plugin bootstrap using metadata |
| Runtime invocation adapter | Missing | Use `DailyOS_Runtime_Client::invoke_ability()` from W3-B |
| Projection render adapter | Missing | Render W4-D `ProjectedComposition` into Gutenberg-safe HTML |
| Trust-band markup | Missing | Render visible band labels and `data-ds-trust-band` attributes |
| Provenance side panel | Missing | Vanilla DOM island for field attribution detail already scope-filtered upstream |
| Stale-projection banner | Missing | Compare attributes against DOS-589 delivered version state |
| Tamper/quarantine banner | Missing | Render W4-C quarantine state without re-verifying signatures |
| Editor preview route | Missing | Proxy preview through PHP/server render path; no direct runtime call from JS |
| Feedback hook attributes | Missing | Persist save-handler observed changes for W5-A; no direct feedback write |
| Negative fixtures | Missing | PHPUnit/JS fixtures named below |

| Surface | W4-A ownership |
|---|---|
| New ability output shape | No - W4-A0 owns it |
| `ProjectedComposition` shape | No - W4-D owns it |
| Bridge error variants | No - W4-B/W4-C own them |
| Projection envelope verification | No - W4-C owns it |
| User-presence nonce lifecycle | No - W4-E owns it |
| HMAC signing implementation | No - W3-B owns it |
| Feedback persistence | No - W5-A owns it |
| Theme shell implementation | No - W5-B owns it |
| Outbox delivery substrate | No - DOS-589 owns it |

## 6. Directional decisions resolved at L0

### 6.1. Block metadata and registration

- `block.json` uses `apiVersion: 3`.
- `name` is `dailyos/account-overview`.
- The block is dynamic.
- `render` is `file:./render.php`.
- `editorScript` points to `file:./edit.js`.
- `style` points to the compiled `style.css`.
- `editorStyle` points to the compiled `editor.css`.
- `supports.html` is false.
- `supports.reusable` stays false until duplicate composition identity is resolved.
- `supports.inserter` is true for paired/editor users.
- Registration happens in PHP via `register_block_type()`.
- Registration must be attached to the existing DailyOS plugin bootstrap after W3-A/W3-B merge.
- W4-A does not register a second plugin or namespace owner.
- W4-A does not add a static `save()` markup contract.

Block-registration hook ownership pinned: W3-A's plugin bootstrap reserves a `dailyos_register_blocks()` hook invoked from WordPress `init`. W4-A's first commit registers `dailyos/account-overview` inside that hook. W4-A start is gated on W3-A V-next having reserved the hook (else W4-A would invent a parallel registration path).

Minimal metadata shape with **V2-pinned attribute schema** (codex HIGH — load-bearing for the saved contract):

```json
{
  "$schema": "https://schemas.wp.org/trunk/block.json",
  "apiVersion": 3,
  "name": "dailyos/account-overview",
  "title": "Account Overview",
  "category": "dailyos",
  "supports": {
    "html": false,
    "reusable": false,
    "inserter": true
  },
  "attributes": {
    "composition_id": { "type": "string" },
    "composition_version": { "type": "integer", "default": 0 },
    "block_id": { "type": "string" },
    "account_id": { "type": "string" },
    "watermarks": { "type": "object", "default": {} },
    "cache_hint_token": { "type": "string", "default": "" },
    "block_instance_id": { "type": "string" }
  },
  "render": "file:./render.php",
  "editorScript": "file:./edit.js",
  "style": "file:./style.css",
  "editorStyle": "file:./editor.css"
}
```

**Critical (codex C1 + V3 codex HIGH #3):** the `attributes` schema deliberately has NO `cached_projection` AND NO `actor_scope_fingerprint`. Block attributes serialize into `post_content` which is publicly-readable / portable — storing scope-filtered projection bodies OR scope-fingerprint there is a scope-leak vector. The schema carries only the watermarks needed to identify which composition this block instance refers to plus an opaque `cache_hint_token` (non-secret, opaque, runtime-derived). The actual `ProjectedComposition` is fetched fresh on every render via the runtime (per §6.3); the runtime maintains a server-side scope-bound cache that PHP NEVER interprets (per §6.2 V3). The `cache_hint_token` is what PHP echoes back to enable fast cache lookup; the substrate is the sole interpreter.

### 6.2. Attribute set and cache watermarking (V2 REWRITE per codex C1)

V1 allowed `cached_projection` in block attributes "for portability." Codex Cycle 1 CRITICAL flagged this as a claim-body leak vector: block attributes serialize into `post_content` which is publicly readable via WP REST and exportable. Storing scope-filtered projection bodies there leaks to any lower-scope render or REST consumer. V2 drops the cache from attributes entirely.

**Attribute discipline rules:**

- Block attributes carry ONLY: `composition_id`, `composition_version`, `block_id`, `account_id` (opaque subject ref), `watermarks` (the §6.6 stale-detection pair extracted per claim ref), optional `cache_hint_token` (opaque, runtime-issued; non-secret; §6.2 cache layer), optional `block_instance_id` for W5-A correlation.
- **No `cached_projection` attribute.** Block attributes MUST NOT carry projected payload bytes, claim text, source excerpts, provenance envelopes, signing material, nonce values, OR any scope-derived material (no scope hashes, scope fingerprints, scope set identifiers).
- `composition_id` is a string (UUID per W4-B V9 convention).
- `composition_version` is integer-typed in the schema (per §6.1); WP serialization tolerates string→int but block.json `default: 0` enforces the type at deserialize.
- `account_id` is an opaque subject reference; non-authoritative for actor binding (actor context derives from current WP user per §6.12).
- `watermarks` is a structured object per Phase 0 artifact 02 lines 128-157: `{ claims: { [claim_id]: { claim_version }}, composition: { composition_version }}` — used solely for stale detection.
- `cache_hint_token` is an opaque, non-secret string issued by the runtime on a successful project call. PHP echoes it back on the next call so the runtime can short-circuit cache lookup. PHP NEVER derives, interprets, or asserts anything from this token. The runtime treats the token as advisory and re-validates scope server-side on every call (see §6.2 cache layer).

**Per-render fetch is canonical — cache lives RUNTIME-SIDE (V3 per codex C2 HIGH #3):**

V2 attempted a PHP-side `wp_cache` keyed by `scope_set_hash = SHA256(Actor::SurfaceClient { scopes })`. Codex correctly flagged the impossibility: §6.12 forbids PHP from asserting scope identity (substrate is sole actor authority); PHP cannot compute the scopes hash without duplicating that authority. V3 moves the cache server-side:

- `render.php` calls `DailyOS_Runtime_Client::project_composition_for_surface($composition_id, $composition_version, $actor_ctx)` (per §6.3.1) on EVERY render.
- The RUNTIME (substrate side) maintains the cache, keyed by `(composition_id, composition_version, scope_set_canonical_id)` — where `scope_set_canonical_id` is derived inside the substrate from the authenticated SurfaceClient session (the substrate IS the scope authority).
- The runtime returns a `ProjectedComposition` plus an opaque `cache_hint_token` (non-secret, opaque string).
- PHP MAY echo the `cache_hint_token` back in subsequent calls so the runtime can short-circuit faster on cache hits, but PHP NEVER derives or interprets the token.
- TTL is enforced server-side (60 seconds, matching W4-E nonce lifetime).
- Cache invalidation on scope-set change happens server-side automatically — the substrate's `scope_set_canonical_id` changes when the actor's scopes change, miss-then-recompute is natural.
- No PHP-side cache. No `actor_scope_fingerprint` in block attributes.

Block attribute schema (per §6.1) carries no scope-fingerprint field. The runtime is the cache authority.

**Audit emission split (V2 NEW per /cso M-2):**
- Runtime cache miss → runtime emits `ability_invoked` (live producer call).
- Runtime cache hit → runtime emits `projection_cache_served` (cache hit). Both carry `composition_id` + `composition_version` + actor.
- PHP does NOT emit audit events directly; the runtime side is the only emitter (per §6.3 audit-intent drain).

W4-B §3 watermarks make `composition_id` + `composition_version` the stale-detection pair; §6.6 reads from `wp_options.dailyos_composition_versions[composition_id]` (the DOS-589-published store) to detect stale.

Attribute discipline NEGATIVE rules (CI-enforced per §9):
- Attributes must not carry raw claim text.
- Attributes must not carry full provenance envelopes.
- Attributes must not carry signing material (signatures, key IDs, HMAC tokens).
- Attributes must not carry nonce values.
- Attributes must not carry projected payload bodies (the V1 `cached_projection` field is removed entirely).

### 6.3. PHP render contract (V2 REWRITE per codex HIGH)

V1 said PHP invokes the producer ability then "passes" the Composition to W4-D. That was unimplementable: W4-D is a Rust composition-level API; PHP cannot directly call it without reimplementing the projection rules. V2 routes through a single runtime-side method.

**Canonical call chain:**

1. `render.php` reads sanitized attributes (per §6.2 attribute schema), including the optional opaque `cache_hint_token`.
2. `render.php` derives actor context from `wp_get_current_user()` and the paired SurfaceClient session (per §6.12).
3. `render.php` calls **`DailyOS_Runtime_Client::project_composition_for_surface(composition_id, composition_version, actor_ctx, cache_hint_token?)`** on EVERY render — a NEW W3-B-exposed method (see §6.3.1) that wraps the Rust composition-level API end-to-end. PHP performs NO local cache check; the runtime is the cache authority (per §6.2 V3).
4. The runtime side internally: looks up its server-side cache keyed by `(composition_id, composition_version, scope_set_canonical_id)` (substrate-derived scope identity); on hit, returns the cached `ProjectedComposition` + a refreshed `cache_hint_token` and emits a `projection_cache_served` audit. On miss, invokes W4-A0 producer ability → receives committed `AbilityOutput<Composition>` → calls W4-D `project_composition_for_surface(composition, ctx)` → drains `Vec<AuditIntent>` via `emit_surface_audit` (including `ability_invoked`) → caches the result → returns `ProjectedComposition` DTO + new `cache_hint_token` to PHP.
5. `render.php` renders the `ProjectedComposition` DTO and stashes the returned `cache_hint_token` back to block attributes (advisory; never interpreted by PHP).

**Critical rules (codex HIGH):**

- PHP NEVER directly calls W4-D; the bridge layer wraps it.
- PHP NEVER reimplements block-level helpers (per W4-D V3 V8 §1.1 those are `pub(crate)`).
- PHP NEVER inspects raw unknown payloads.
- PHP NEVER recomputes trust bands; W4-A0 emits them, W4-D preserves them, W4-A renders them.
- PHP NEVER mints claim refs or mutates canonical DailyOS data.
- The runtime call uses W3-B HMAC signing per W2-B contract; no hand-rolled canonicalization.
- All projected text passes through `esc_html` or a documented `wp_kses` allowlist per /cso MEDIUM (CI invariant grep against unescaped `<?= $var ?>` patterns in render.php).

### 6.3.1. W3-B contract this packet assumes (V2 NEW per devex P2)

W4-A binds to W3-B's runtime client (DOS-565). V2 pins the assumed method signature; if W3-B ships with a different signature, W4-A V3 amends (no local shim):

```php
class DailyOS_Runtime_Client {
    /**
     * Composition-level projection for a SurfaceClient render path.
     *
     * Internally invokes the named producer ability (W4-A0), drains
     * audit intents via emit_surface_audit, and returns a scope-filtered
     * ProjectedComposition DTO together with a refreshed runtime-issued
     * opaque cache_hint_token. PHP does NOT call W4-D directly.
     *
     * @param string      $composition_id        composition id (UUID; from block attribute)
     * @param int         $composition_version   expected composition version (W4-B watermark)
     * @param array       $actor_ctx             ['surface_client_id' => string, 'session_token' => string]
     * @param string|null $cache_hint_token      optional opaque cache_hint_token from a prior render (per §6.2 V3); runtime treats as advisory, never client-asserted authority
     * @return array      ['projection' => array (ProjectedComposition DTO per W4-D V3 §1.1), 'cache_hint_token' => string (opaque, base64; substrate-issued)]
     * @throws DailyOS_Runtime_Error  on HMAC failure, 401/403/422/423/409/500 per W4-B V9 §6.5
     */
    public function project_composition_for_surface(
        string $composition_id,
        int $composition_version,
        array $actor_ctx,
        ?string $cache_hint_token = null
    ): array;
}
```

Ability-id namespace is `dailyos/account-overview` (slash-form) per W3-A V8 §37 namespace reservation. The runtime resolves which producer ability to invoke from `composition_id` → `composition_versions.generated_by_invocation_id` (W4-B V9 §8). PHP never names the ability — that's the runtime's job.

W4-A's interlock with DOS-565: surface this method as an explicit obligation on the W3-B issue (Linear comment on DOS-565 once Linear MCP is reconnected).

### 6.4. Trust-band rendering (V2 — bound to `TrustBandBadge` primitive)

- Trust scoring lives upstream.
- W4-A renders the band supplied by W4-A0/W4-D.
- W4-A renders the visible three-band vocabulary.
- The visible values are `likely_current`, `use_with_caution`, and `needs_verification`.
- **Full Design System attribute set required (V2 per ui-design P1-3):** trust-band element carries `data-ds-tier="primitive"` + `data-ds-name="TrustBandBadge"` + `data-ds-spec="primitives/TrustBandBadge.md"` to preserve audit-tool targeting parity with the shipped React `TrustBandBadge` component. `data-ds-trust-band="<band>"` is supplementary state.
- Markup example: `<span data-ds-tier="primitive" data-ds-name="TrustBandBadge" data-ds-spec="primitives/TrustBandBadge.md" data-ds-trust-band="likely_current">…</span>`
- CSS maps those attributes to DailyOS trust tokens.
- CSS uses `--color-trust-likely-current`.
- CSS uses `--color-trust-use-with-caution`.
- CSS uses `--color-trust-needs-verification`.
- Stock themes receive safe fallback custom properties.
- Missing or unknown band degrades to `needs_verification`.
- W4-A does not expose `unscored` as a visible band.
- W4-A does not promote a caution band to likely-current.
- W4-A does not hide the band in compact or mobile layout.

### 6.5. AboutThisIntelligencePanel (WordPress port) (V2 — bound to canonical pattern)

V1 proposed a "vanilla DOM island"; ui-design P1-1 flagged that DailyOS already has the `AboutThisIntelligencePanel` pattern (`.docs/design/patterns/AboutThisIntelligencePanel.md`). V2 binds the WP side to that pattern instead of inventing a parallel surface.

- Provenance is rendered as an inline affordance on fields or blocks with attribution.
- Clicking or focusing the affordance opens a side panel.
- The panel is a small vanilla DOM island in v1.
- The panel does not require a shadcn-style React island.
- The panel lazy-loads field attribution detail through a WordPress server route.
- The server route calls the runtime through W3-B.
- The runtime returns scope-filtered field attribution per W4-B §16 and W4-D projection context.
- W4-A renders only the filtered result.
- W4-A does not ask for broader data to fill the panel.
- W4-A does not show hidden fields, hidden sources, revoked attribution, or dropped payload.
- The panel must be keyboard reachable.
- The panel must close on Escape and outside click.
- The panel must not serialize detail into block attributes.
- The panel must not preload all provenance into page HTML.
- Product copy uses "About this" or equivalent DailyOS vocabulary.

### 6.6. Stale-projection banner (V2 — bound to `StaleReportBanner` pattern)

V2 binds to the canonical `StaleReportBanner` pattern (`.docs/design/patterns/StaleReportBanner.md`) rather than authoring inline banner HTML.

- W4-A compares local `composition_version` to the latest substrate-reported version.
- **Version state surface (V2 per eng P1-2):** the latest version reaches WordPress through DOS-589 delivery into **`wp_options.dailyos_composition_versions[composition_id]`** — a `dailyos_*`-prefixed option per W3-A V8 namespace reservation. W4-A reads this option key; does not poll substrate tables.
- If local version is older, W4-A renders the banner.
- **Pattern binding:** banner emits `data-ds-tier="pattern"` + `data-ds-name="StaleReportBanner"` + `data-ds-spec="patterns/StaleReportBanner.md"`. Inherits the pattern's editorial mark (rule-above, lowercase eyebrow, single-sentence body, finite link). No ad-hoc card or alarm chrome.
- Banner appears above the affected account overview.
- Banner does not block rendering of the freshly fetched projection (per §6.2 there is no attribute cache; the projection is the live runtime call).
- Banner offers refresh when the viewer has permission.
- Front end never claims stale content is fresh.
- Editor Reload from runtime button clears stale state only after successful invocation.
- If runtime is unreachable, current rendered content remains visibly marked stale.
- Stale detection uses `composition_id` + `composition_version`.
- Stale detection tolerates missing outbox state by treating as "unknown — refresh cautiously."

User-facing copy (V2 — magazine voice per ui-design P3):

```text
Newer context has arrived. Refresh to bring this in.
```

### 6.7. Tamper and rollback banner + enforcement_mode trust-gate (V2 — codex HIGH + ui-design P1-2)

V2 pins the W4-C `projection_signature_enforcement` mode as a HARD trust-affordance gate, plus binds the banner to canonical pattern naming.

**Verification responsibility:**
- Signature verification lives in W4-C; W4-A reacts to the result.
- W4-A does NOT run Ed25519 verification, reconstruct canonical bytes, or compare signature IDs.

**Enforcement-mode states (V2 NEW per codex HIGH):**

| enforcement_mode | Verification + currentness PASS | Failure or unknown-key |
|---|---|---|
| `enforce` | Trusted affordances visible (provenance click-through, trust-band badges, click-bound feedback) | Suppress trusted affordances; render banner |
| `shadow` | Bytes may render BUT trust affordances visibly DOWNGRADED (banner "Verification in progress" + no provenance click-through) | Same — never trusted, never alarming |
| `disabled` | Same as shadow plus operator-facing notice | Same as shadow |

**Banner pattern binding:** tamper and rollback banner is a variant of `ConsistencyFindingBanner` (`.docs/design/patterns/ConsistencyFindingBanner.md`), or a promoted-new `VerificationBanner` pattern (requires 4-way sync per `feedback_chrome_overlap_audit_before_new_pattern` — markdown spec + source CSS module + reference render + inventory). Emits `data-ds-tier="pattern"` + `data-ds-name="ConsistencyFindingBanner"` (or `VerificationBanner` if promoted) + `data-ds-spec="patterns/ConsistencyFindingBanner.md"`.

**W4-C precedence rules (per W4-B V9 §6.5):**
- `ProjectionTampered` (precedence 0) above stale-watermark (4-6).
- `ProjectionVersionRollback` (precedence 1) above stale-watermark.
- Tamper error path does NOT emit `correction.claim` payload.

**Quarantine + ledger linking:**
- If W4-C reports quarantine, W4-A renders the banner.
- Banner links to safe ledger / quarantine detail only for authorized editors (`current_user_can('manage_options')` or operator role).
- Public visitors see degraded trust state without raw diagnostic IDs.
- W4-A NEVER exposes raw runtime errors, signature bytes, key IDs, or payload hashes in public HTML.

User-facing copy (V2 — magazine voice per ui-design P3):

```text
Something about this account doesn't line up. Verify before acting.
```

### 6.8. Unknown block delegation

- W4-A knows only the block types W4-A0 can emit today.
- W4-A does not author additional block types.
- If `ProjectedComposition` contains an unknown or fallback block, W4-A follows W4-D fallback rules.
- W4-D owns nearest-known-type selection.
- W4-D owns unknown-block caps.
- W4-D owns raw-payload exclusion.
- W4-D owns fallback trust-band capping.
- W4-A renders the fallback DTO as supplied.
- W4-A never renders raw payload values from an unknown block.
- W4-A never logs dropped payload values.
- W4-A never stores dropped payload values in attributes.
- W4-A switch statements must have a fallback case that delegates to W4-D output, not to raw JSON.
- W4-A may emit W4-D `AuditIntent` through the runtime-supported path.
- Unknown-block fallback should log support diagnostics upstream via W4-D audit intent.

### 6.9. Editor preview

- `edit.js` renders the same composition shape as `render.php`.
- `edit.js` uses `useBlockProps()`.
- `edit.js` calls a WordPress API wrapper for preview.
- The wrapper proxies to PHP/server render logic in preview mode.
- Browser JS never calls the Rust runtime directly.
- Browser JS never receives HMAC key material.
- Browser JS never receives pairing secret material.
- The preview route returns sanitized HTML or projected preview props.
- Preview and front-end render share the same projection contract.
- The Reload from runtime button re-invokes the ability through PHP.
- Reload bypasses compatible cache only for the explicit refresh request.
- Repeated clicks must not start parallel refresh storms.
- A 200ms editorial shimmer on preview load is acceptable.
- **Editor chrome isolation (V2 NEW per ui-design P2-5):** editor preview renders the block's editorial content only; magazine shell (`FolioBar`, `FloatingNavIsland`, `AtmosphereLayer`, `MagazinePageLayout`) does NOT appear inside the Gutenberg canvas. Gutenberg supplies its own chrome.
- **Account selection UX (V2 NEW per devex P2):** the editor exposes `account_id` selection via `InspectorControls` + `ComboboxControl`, populated from a paginated REST search endpoint backed by W3-B. No slash-command or URL-param shortcut in v1.
- **Build toolchain (V2 per devex P1):** `wp-scripts start --hot` for dev loop. `wp-scripts build` for production. `package.json` lives at `wp/dailyos/package.json` (or workspace root if W3-A consolidates).
- Every preview render calls `DailyOS_Runtime_Client::project_composition_for_surface(...)` per §6.3; the runtime returns either a cache-served or live `ProjectedComposition` opaquely (PHP cannot distinguish, and need not). If the underlying composition version may have advanced (DOS-589 outbox check), the editor surfaces a refreshing indicator while the next render call completes.
- Successful refresh updates `composition_id`, `composition_version`, `watermarks`, and optionally `cache_hint_token` block attributes. No `cached_projection` attribute exists (per §6.2 V2 — runtime owns the cache; block attributes do not store projection bodies).
- Failed refresh keeps the prior rendered DOM visibly marked stale via the §6.6 banner pattern; PHP does NOT serve a local cached projection (there is no PHP-side cache per §6.2 V3). The "preserve last-known good" semantics live RUNTIME-SIDE: on upstream runtime failure, the substrate may serve its own cached `ProjectedComposition` if the cache entry remains valid for the actor's current scope set; otherwise the renderer surfaces the error envelope per §6.7.

### 6.9.1. Empty state (V2 NEW per ui-design P2-3)

When the produced Composition contains zero visible blocks (all claims sensitivity-filtered, no data yet, or all-quarantined), W4-A renders a silent posture per /cso class-level scope-filter rule:

- Single editorial line: "No account context to show here."
- No trust band on the empty state.
- No banners.
- No empty cards.
- No "some content is restricted" indicator (per privacy-safe default — silence is the absence-signal).
- `FinisMarker` rule below the message.
- Block-level fixture `dos572_fixture_empty_projection_renders_silent.php` asserts no provenance-attribution traces leak.

### 6.10. Magazine theme and stock-theme rendering

- The DailyOS theme provides the full magazine shell in W5-B.
- W4-A must work under that shell without fighting it.
- W4-A must also render safely under stock WordPress themes.
- `style.scss` should expose shell-aware CSS variables.
- `style.scss` should define local fallbacks for page, ink, rule, account, and trust tokens.
- The block should use editorial rows, rules, sections, and reading order.
- The block should avoid dashboard card grids unless rendering a repeated item that genuinely needs a card.
- The block should not assume `FolioBar` or `FloatingNavIsland` exist.
- Under the DailyOS theme, the block may align with `MagazinePageLayout` width and typography.
- Under stock themes, the block should remain self-contained, readable, and accessible.
- The first viewport signal should be the account context, not internal diagnostics.
- Color communicates trust, state, entity identity, or action.
- Decorative color is out of scope.
- Finite endings are preferred where the theme shell can support them.

### 6.11. Feedback hook

- W4-A exposes feedback affordances only when W4-E nonce issuance succeeds.
- W4-A requests a nonce from the runtime before exposing a feedback action.
- W4-A binds nonce request to the field route supplied by W4-D.
- W4-A does not mint nonces locally.
- W4-A does not persist feedback locally.
- W4-A does not call claim mutation services.
- W4-A does not implement W5-A routing.
- W5-A observes save-handler attribute changes and edit-route metadata.
- W4-A records enough block attribute delta to let W5-A identify click-bound feedback candidates.
- Missing nonce means no feedback affordance.
- Expired nonce means affordance refresh before action.
- FeedbackTarget routes with zero claim refs do not expose claim correction affordances.
- `ComputedFrom` and `DisplayOnly` routes do not become claim-feedback receivers.

### 6.12. `wp_user_id` session binding (V2 — renderer is pure forwarder)

Per /cso LOW Cycle 1: V1 had PHP `derive actor context from current_user` AND bridge validates session-bound `wp_user_id`. Two validation paths invite divergence. V2 eliminates the renderer-side derivation:

- W4-A renderer NEVER asserts `wp_user_id`. It forwards only the SurfaceClient session token; the runtime substrate is the sole authority on actor identity.
- The bridge precondition `validate_session_bound_wp_user_id` (per W4-B V8 §17) runs at request entry and binds `wp_user_id` from the paired session, NOT from any request body/header/query field.
- The runtime side resolves the SurfaceClient session → bound `wp_user_id` → `Actor::SurfaceClient { instance, scopes }`.
- If the SurfaceClient session is invalid or paired to a different `wp_user_id` than the current WP user, the bridge returns `403 wrong_user` BEFORE any further dispatch — no runtime invocation, no nonce issue, no projection detail, no feedback path. Per W4-B V9 §17 and §43 fixture.
- `render.php` MAY use `wp_get_current_user()` to gate WHO sees an affordance (`current_user_can('edit_posts')` for editor-only links), but that is WP-side capability gating distinct from substrate actor binding.
- Editor preview uses the same session-bound actor rule as front-end render.
- Provenance side-panel detail (per §6.5) uses the same session-bound actor rule.

## 7. Acceptance criteria

1. The implementation creates exactly one Gutenberg block: `dailyos/account-overview`.
2. The block is registered by PHP via `register_block_type()`.
3. The block lives under `wp/dailyos/blocks/account-overview/`.
4. The directory contains `block.json`.
5. The directory contains `render.php`.
6. The directory contains `edit.js`.
7. The directory contains `save.js`.
8. The directory contains `style.scss`.
9. The directory contains `editor.scss`.
10. `block.json` uses `apiVersion: 3`.
11. `block.json` declares a dynamic PHP render path.
12. `save.js` returns `null`.
13. The PHP render path calls **`DailyOS_Runtime_Client::project_composition_for_surface($composition_id, $composition_version, $actor_ctx, $cache_hint_token = null)`** (per §6.3.1) — a single runtime-side method that internally invokes W4-A0 producer (or serves a cache hit per §6.2 V3 runtime-side cache), calls W4-D projector, drains audit intents, and returns BOTH the scope-filtered `ProjectedComposition` DTO and a refreshed opaque `cache_hint_token` (per the V3 round-trip).
14. Runtime invocation uses the W3-B HMAC signer.
15. No direct browser-to-runtime request exists.
16. PHP NEVER receives raw `AbilityOutput<Composition>`; only the projected DTO.
17. PHP NEVER calls W4-D directly (per V2 §6.3).
18. PHP NEVER calls W4-D block-level helpers (those are `pub(crate)` per W4-D V3).
19. The render path renders the returned `ProjectedComposition` DTO and no other shape.
20. Audit intents are drained by the runtime side; PHP does not call substrate audit tables.
21. Trust bands render visibly as `likely_current`, `use_with_caution`, and `needs_verification`.
22. Trust-band CSS uses DailyOS trust tokens with stock-theme fallbacks.
23. Missing or unknown trust band degrades to `needs_verification`.
24. Provenance refs render inline where policy allows.
25. Claim refs render inline where policy allows.
26. Provenance click-through opens a side panel.
27. Side-panel detail is scope-filtered per W4-B §16.
28. Side-panel detail is lazy-loaded and not serialized into attributes.
29. Block attributes include `composition_id`.
30. Block attributes include `composition_version`.
31. Block attributes include `watermarks` (the structured `{claims, composition}` watermark object per §6.2).
32. Block attributes include `block_id` and optional `block_instance_id`. **Block attributes do NOT contain `cached_projection` or `actor_context_hint`** (per V2 §6.2 — those V1 fields were removed as scope-leak vectors).
33. Watermarks detect stale projection using `composition_id` and `composition_version`.
34. Stale projection displays a visible banner when local version lags delivered version state.
35. Stale projection remains visibly marked if runtime is unreachable.
36. Tamper state from W4-C displays a visible banner.
37. Projection rollback state from W4-C displays a visible banner.
38. W4-A reacts to W4-C verification state and does not re-verify signatures.
39. Unknown block fallback delegates to W4-D rules.
40. Unknown block fallback never renders raw payload.
41. Editor preview renders the same projected composition as front-end render.
42. Editor Reload from runtime re-invokes the ability.
43. Editor reload updates composition attributes only after success.
44. Editor Reload that succeeds updates `composition_id`, `composition_version`, `watermarks`, and optionally `cache_hint_token` block attributes (per §6.2 V2 — NO `cached_projection` attribute). Editor Reload that fails leaves block attributes unchanged AND keeps the prior rendered DOM visibly marked stale via §6.6 banner; PHP serves no local cached projection (no PHP-side cache per §6.2 V3).
45. Feedback affordances appear only after W4-E nonce issue succeeds.
46. Missing nonce suppresses the feedback affordance.
47. W5-A can consume save-handler attribute changes for click-bound feedback.
48. W4-A does not persist feedback.
49. W4-A does not author additional block types.
50. W4-A derives actor context from current WordPress user and paired session.
51. W4-A inherits W4-B §17 wrong-user rejection.
52. The block renders under the DailyOS theme.
53. The block renders under stock WordPress themes.
54. The block does not use customer-specific data in fixtures or snapshots.
55. The block does not leak HMAC key material, nonce material, pairing secrets, raw runtime errors, or raw payload fields.
56. PHPUnit fixtures cover PHP render and registration failures.
57. JS fixtures cover editor preview parity and reload behavior.
58. CI checks protect dynamic block registration, null save, and asset loading.
59. L1 proof includes screenshots or rendered HTML under DailyOS theme and stock theme.
60. L1 proof includes a stale banner case and a tamper banner case.

## 8. Negative fixtures

Negative fixtures are named here for implementation.
Do not create the files during L0 prep.
Use generic data only.

1. **`dos572_fixture_stale_projection_banner.php`**
2. Input: saved attributes contain an older `composition_version` than delivered version state.
3. Expected: render includes stale banner and does not claim the view is current.
4. **`dos572_fixture_signature_mismatch_banner.php`**
5. Input: W4-C reports `ProjectionTampered`.
6. Expected: render includes verification banner and no correction payload.
7. **`dos572_fixture_unknown_block_falls_back.php`**
8. Input: projected composition includes fallback block from W4-D.
9. Expected: render uses fallback DTO and contains no raw unknown payload sentinel.
10. **`dos572_fixture_missing_nonce_no_feedback_affordance.php`**
11. Input: W4-E nonce issue fails or is absent.
12. Expected: feedback action is not rendered.
13. **`dos572_fixture_actor_scope_filters_field_attribution.php`**
14. Input: actor lacks scope for one field attribution.
15. Expected: side panel omits the field and shows no placeholder leak.
16. **`dos572_fixture_stock_theme_renders_safe_shell.php`**
17. Input: block rendered without DailyOS theme variables.
18. Expected: safe fallback typography, color, and spacing apply.
19. **`dos572_fixture_editor_preview_matches_frontend.js`**
20. Input: editor preview and front-end render receive same projected composition.
21. Expected: normalized markup/projection parity holds.
22. **`dos572_fixture_block_registers_under_correct_namespace.php`**
23. Input: plugin bootstrap registers blocks.
24. Expected: `dailyos/account-overview` exists and no alternate namespace exists.
25. **`dos572_fixture_runtime_invocation_uses_w3b_hmac_signer.php`**
26. Input: render invokes the runtime.
27. Expected: invocation goes through `DailyOS_Runtime_Client` signer, not raw HTTP.
28. **`dos572_fixture_projection_version_rollback_banner.php`**
29. Input: W4-C reports `ProjectionVersionRollback`.
30. Expected: rollback banner wins over stale banner.
31. **`dos572_fixture_wrong_user_rejected_before_preview.php`**
32. Input: preview request carries mismatched asserted user.
33. Expected: `403 wrong_user` before runtime invocation.
34. **`dos572_fixture_runtime_cache_hit_drops_raw_error.php`**
35. Input: runtime returns a `503` upstream error WITH a runtime-side cache hit on the prior compatible projection (server-side cache per §6.2 V3).
36. Expected: PHP renders the runtime-served cached `ProjectedComposition`, no raw exception body, no claim/scope material in `post_content`.
37. **`dos572_fixture_save_returns_null.js`**
38. Input: block save invoked.
39. Expected: saved output is null and contains no rendered DailyOS HTML.
40. **`dos572_fixture_trust_band_unknown_degrades.php`**
41. Input: projected block carries absent or unknown band.
42. Expected: visible band is `needs_verification`.
43. **`dos572_fixture_reload_updates_watermarks.js`**
44. Input: editor Reload from runtime succeeds.
45. Expected: attributes update `composition_id`, `composition_version`, `watermarks`, and (optionally) `cache_hint_token`. The `cached_projection` attribute MUST NOT be present (removed per §6.2 V2 — runtime owns the cache, not block attributes).

## 9. CI invariants

1. Block registration test asserts `dailyos/account-overview` exists.
2. Block registration test asserts registration uses metadata from `block.json`.
3. Static grep fails if `save.js` returns serialized DailyOS HTML.
4. Static grep fails if block attributes include HMAC, secret, token, bearer, or raw nonce fields.
5. Static grep fails if browser JS calls `127.0.0.1`, loopback runtime URLs, or runtime ports directly.
6. Static grep fails if `edit.js` imports or reconstructs HMAC signer logic.
7. PHP unit test asserts runtime invocation uses `DailyOS_Runtime_Client::invoke_ability`.
8. PHP unit test asserts wrong-user precondition prevents runtime invocation.
9. PHP unit test asserts `render.php` calls only composition-level W4-D projection.
10. PHP unit test asserts raw unknown payload sentinel never appears in output.
11. PHP unit test asserts raw runtime exception text never appears in output.
12. PHP unit test asserts stale banner appears when delivered version is newer.
13. PHP unit test asserts tamper banner has precedence over stale banner.
14. PHP unit test asserts rollback banner has precedence over stale banner.
15. PHP unit test asserts trust-band data attributes are emitted.
16. PHP unit test asserts unknown trust band degrades to `needs_verification`.
17. PHP unit test asserts provenance side-panel bootstrap contains no full attribution payload.
18. JS test asserts preview calls WordPress API wrapper, not runtime.
19. JS test asserts Reload from runtime debounces parallel refresh.
20. JS test asserts successful reload updates watermark attributes.
21. JS test asserts failed editor reload leaves block attributes unchanged and surfaces stale-marker DOM; static grep fails if `edit.js` or `render.php` introduces any `cached_projection`, `actor_scope_fingerprint`, or `actor_context_hint` attribute — the runtime is the sole projection-cache authority (per §6.2 V3).
22. JS test asserts `save.js` returns null.
23. Style test or snapshot asserts stock theme fallback variables exist.
24. Style test or snapshot asserts DailyOS theme variables override fallbacks.
25. Fixture test asserts no customer-specific strings in block fixtures.
26. Fixture test asserts user-facing copy avoids internal process vocabulary.
27. Accessibility test asserts provenance affordance is keyboard reachable.
28. Accessibility test asserts side panel closes on Escape.
29. Accessibility test asserts trust band remains perceivable without color alone.
30. L1 evidence includes rendered front-end HTML and editor preview parity.

## 10. Interlocks with W4 stage-2 + W5-A + DOS-589

| Owner | W4-A consumes | W4-A obligation |
|---|---|---|
| W4-B | `commit_composition`, watermarks, `ClaimRef::with_field`, `Block.field_bindings`, error precedence, `version_events`, `wp_user_id` session binding, `surface_client.rs` ownership | Render watermarks and refs; do not assign or validate them locally |
| W4-A0 | Committed `AbilityOutput<Composition>`, claim-type to `BlockType` mapping, trust-band fallback table | Invoke producer and render output; do not produce alternate composition shape |
| W4-C | Signed projection envelope, ledger currentness, quarantine state, `ProjectionTampered`, `ProjectionVersionRollback`, shadow-trust flag | Render degraded/quarantine states; do not re-verify signatures |
| W4-D | `project_composition_for_surface(composition, ctx)`, `ProjectedComposition`, fallback blocks, edit routes, audit intents | Consume composition-level API only; never raw unknown payload |
| W4-E | Nonce issue/verify endpoints and tuple binding | Request nonce before feedback affordance; do not mint locally |
| W3-B | `DailyOS_Runtime_Client`, HMAC signer, pairing UI, PHP-to-runtime transport | Use client; no raw HTTP signer in block code |
| DOS-589 | Delivery of `version_events` into surface-observable stale state | Compare delivered composition version against attributes |
| W5-A | Feedback router and save-handler attribute observation | Preserve enough edit-route/watermark attributes for routing |
| W5-B | DailyOS magazine WordPress theme | Render shell-aware CSS while retaining stock-theme fallbacks |

Interlock rules:

1. W4-A starts after W4-A0, W4-C, W4-D, and W3-B contracts are merged or available in the implementation branch.
2. Feedback affordances may ship disabled until W4-E is available.
3. If W4-E is not merged, W4-A must render no feedback actions rather than a local placeholder.
4. If DOS-589 is not merged, stale detection may use the runtime response but must mark outbox-driven stale checks as blocked.
5. If W5-B is not merged, W4-A must still pass stock-theme rendering fixtures.
6. If W5-A is not merged, W4-A must still persist the agreed attribute changes for later consumption.
7. Any mismatch between W4-D `ProjectedComposition` and W4-A render needs a W4-D packet amendment, not a local W4-A DTO fork.

## 11. What W4-A explicitly does NOT own

- W4-A does not author the `ProjectedComposition` shape.
- W4-A does not author W4-D fallback policy.
- W4-A does not author unknown-block nearest-type scoring.
- W4-A does not author raw-payload exclusion rules.
- W4-A does not author `BridgeSurfaceError` variants.
- W4-A does not author error precedence.
- W4-A does not author `ProjectionTampered`.
- W4-A does not author `ProjectionVersionRollback`.
- W4-A does not author Ed25519 signature verification.
- W4-A does not author projection envelope canonicalization.
- W4-A does not author ledger currentness checks.
- W4-A does not author quarantine persistence.
- W4-A does not author W4-A0 ability output.
- W4-A does not author claim-type to `BlockType` mapping.
- W4-A does not author trust scoring.
- W4-A does not author the trust-band fallback table.
- W4-A does not author `commit_composition`.
- W4-A does not author `ClaimRef::with_field`.
- W4-A does not author `Block.field_bindings`.
- W4-A does not author `version_events`.
- W4-A does not author DOS-589 delivery.
- W4-A does not author HMAC canonicalization.
- W4-A does not author pairing UI.
- W4-A does not author nonce issue or verify.
- W4-A does not author feedback persistence.
- W4-A does not author claim mutation services.
- W4-A does not author additional block types.
- W4-A does not author a custom MCP exposure path.
- W4-A does not author the DailyOS WordPress theme.
- W4-A does not own database migrations.
- W4-A does not own Rust schema changes.
- W4-A does not own customer-specific fixtures.

## 12. Open questions

1. **Side-panel UI.**
2. Options: shadcn-style React island or vanilla DOM.
3. Recommendation: vanilla DOM for v1 to stay frontend-light and avoid a second React runtime in WordPress.
4. **Caching home — RESOLVED V3.**
5. Resolution per codex C1 + C2: the projection cache lives RUNTIME-SIDE (substrate-owned), keyed by `(composition_id, composition_version, scope_set_canonical_id)`. Block attributes carry only the opaque `cache_hint_token` (advisory). No PHP-side cache. No WP transient. The substrate is the sole interpreter of cache state and the sole scope-identity authority per §6.12. See §6.2 V3.
7. **Editor preview latency.**
8. Question: accept a 200ms editorial shimmer on `edit.js` load?
9. Recommendation: yes; prefer stable preview over flashing spinners.
10. **Unknown-block support logging.**
11. Question: should fallback log to substrate?
12. Recommendation: yes, through W4-D `AuditIntent`, not W4-A string logs.
13. **Duplicate blocks.**
14. Question: can two blocks share one `composition_id`?
15. Recommendation: allow only if W5-A can disambiguate feedback routes; otherwise force refresh or warn.
16. **Public visitor provenance depth.**
17. Question: how much field attribution should unauthenticated visitors see?
18. Recommendation: W4-D actor context decides; W4-A renders only filtered output.
19. **Block icon.**
20. Question: which DailyOS mark appears in the inserter?
21. Recommendation: reuse existing DailyOS mark from plugin/theme assets once W3/W5 settles asset ownership.
22. **Preview response format.**
23. Question: return sanitized HTML or projection props?
24. Recommendation: sanitized HTML first for parity; projection props only if editor interactions need structured routes.
25. **Theme variable contract.**
26. Question: should W5-B provide every token or only shell aliases?
27. Recommendation: W4-A defines local fallbacks and lets W5-B override.
28. **Nonce prefetch.**
29. Question: prefetch nonces for all visible fields?
30. Recommendation: no; request on visible click/focus intent to keep nonce exposure small.

## 13. Linear dependency edges

- Blocks-on: DOS-589 (W4-A0 producer ability).
- Blocks-on: DOS-? (W4-C projection signing and quarantine).
- Blocks-on: DOS-? (W4-D fallback projector).
- Blocks-on: DOS-? (W4-B concurrency and watermarks).
- Blocks-on: DOS-? (W3-B runtime client and HMAC signer).
- Soft blocks-on: DOS-? (W4-E nonce) for feedback affordances.
- Blocks: DOS-? (W5-A feedback consumption).
- Blocks: DOS-? (W5-B magazine theme).
- Blocks: DOS-589 stale delivery consumers where WordPress projection state is part of verification.

Dependency notes:

- Exact W4-C/W4-D/W4-B/W3-B/W5 issue ids should be tightened by L0 reviewers if Linear already has them.
- W4-A should not start implementation until the W3-B runtime client contract is merged or stable enough to call.
- W4-A can draft block files before W4-E but cannot expose feedback actions without nonce issue.
- W4-A can draft theme-neutral CSS before W5-B but must re-test under W5-B when available.
- W4-A cannot substitute polling for DOS-589 stale delivery without a packet amendment.

## 14. L0 reviewer panel - required runners

- `/plan-eng-review` - substrate fit + WP boundary.
- `/cso` - trust-boundary review for nonce, actor scope, signature verification chain, and secret leakage.
- `/plan-devex-review` - block authoring DX, metadata conventions, preview route, and theme interop.
- `/codex challenge` - adversarial pass against upstream contracts and "thin renderer" boundary.
- `/codex consult` - second opinion on implementation plan and acceptance coverage.
- `/plan-design-review` - magazine vs dashboard discipline, editorial tone, trust/provenance readability.

Reviewer prompts should include:

- This is the first DailyOS Gutenberg block.
- The block is a dynamic WordPress block.
- The block consumes committed `AbilityOutput<Composition>`.
- The block renders `ProjectedComposition`.
- The block must not recompute trust.
- The block must not verify signatures.
- The block must not mint nonces.
- The block must not persist feedback.
- The block must work under DailyOS and stock themes.
- The block must keep user-facing copy in product vocabulary.

## 15. Acceptance for L0 closure

1. Packet exists at the required path.
2. Packet length is between 600 and 800 lines.
3. All 15 required sections are present and non-empty.
4. Changelog records V1.
5. Status snapshot states W4-A is stage-3 renderer work.
6. Pre-work records the absence of `wp/dailyos/` in the prep worktree.
7. Pre-work records WordPress dynamic block conventions.
8. Net-new table covers the six required block files.
9. Directional decisions include block metadata and PHP render path.
10. Directional decisions include the required attribute set.
11. Directional decisions include runtime invocation through W3-B.
12. Directional decisions include trust-band rendering.
13. Directional decisions include provenance side panel.
14. Directional decisions include stale projection banner.
15. Directional decisions include tamper and rollback banner.
16. Directional decisions include unknown-block delegation.
17. Directional decisions include editor preview.
18. Directional decisions include magazine and stock-theme rendering.
19. Directional decisions include feedback hook.
20. Directional decisions include `wp_user_id` session binding.
21. Acceptance criteria reflect every issue-level requirement.
22. Negative fixtures include at least 10 names using the `dos572_fixture_` convention.
23. CI invariants include PHP, JS, style, accessibility, and leakage checks.
24. Interlocks reference W4-B, W4-A0, W4-C, W4-D, W4-E, W3-B, W5-A, W5-B, and DOS-589.
25. Explicit non-ownership section prevents W4-A from taking upstream responsibilities.
26. Open questions include recommendations, not just uncertainties.
27. Linear dependency edges are recorded with placeholders where exact ids are unknown.
28. L0 reviewer panel lists all six required runners.
29. Packet contains no code-style example with issue-number leakage.
30. Packet is ready for L0 reviewers to challenge.
