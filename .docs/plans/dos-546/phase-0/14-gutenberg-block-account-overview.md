---
status: spec:ready
date: 2026-05-10
spike: DOS-546
phase: 0
wave: 2
artifact: 14
related_adrs: [0102, 0105, 0111, 0129, 0130]
open_questions: see ./INDEX.md (routed to W4-B L0 Prep)
---

# 14 — Gutenberg block: `dailyos/account-overview`

## Summary

First DailyOS Gutenberg block. Demonstrates the producer/renderer split
(ADR-0130) by sourcing its content from the `dailyos/account-overview` ability.
Saves only attributes; re-renders on read by invoking the ability, with cached
projection support for performance and fallback per artifact 11.

The ability is the producer. The Gutenberg block is the renderer. WordPress
stores block configuration and references, not canonical DailyOS content.

The block proves that WordPress can invoke DailyOS as a paired `SurfaceClient`,
render DailyOS content without direct substrate reads, preserve provenance and
trust bands, and avoid frozen substrate-authored HTML in `post_content`.

ADR-0129 and ADR-0130 may not exist in this branch. This spec treats them
conceptually: ADR-0129 establishes WordPress Studio as a DailyOS surface, and
ADR-0130 establishes a surface-independent `Composition` contract whose producer
is an ability and whose consumers are renderers.

## block.json schema

The block descriptor is:

```json
{
  "$schema": "https://schemas.wp.org/trunk/block.json",
  "apiVersion": 3,
  "name": "dailyos/account-overview",
  "version": "0.1.0",
  "title": "Account Overview",
  "category": "dailyos",
  "icon": "<TBD — lucide-style svg slug>",
  "description": "DailyOS-produced account overview composition.",
  "keywords": ["dailyos", "account", "overview"],
  "supports": {
    "html": false,
    "reusable": false,
    "inserter": true
  },
  "attributes": {
    "account_id": { "type": "string", "default": "" },
    "composition_id": { "type": "string", "default": "" },
    "composition_version": { "type": "string", "default": "" },
    "claim_refs": { "type": "array", "default": [] },
    "trust_band_render_mode": {
      "type": "string",
      "enum": ["full", "compact", "icon"],
      "default": "full"
    },
    "dailyos_origin": { "type": "string", "default": "substrate" }
  },
  "render": "file:./render.php",
  "editorScript": "file:./edit.js",
  "style": "file:./style.css",
  "editorStyle": "file:./editor.css"
}
```

Descriptor field requirements: `$schema` is the WordPress metadata schema;
`apiVersion` is `3`; `name` is the stable block name; `version` is the block
contract version; `title`, `category`, `icon`, `description`, and `keywords`
drive editor discovery; `supports.html: false` prevents generated HTML edits;
`supports.reusable: false` avoids Phase 0 reuse ambiguity; `supports.inserter:
true` permits manual insertion; `render`, `editorScript`, `style`, and
`editorStyle` point to the renderer and editor assets.

### Attribute: `account_id`

Purpose: opaque substrate account identifier selected by the editor user.

Lifecycle:

- Starts blank when the block is inserted.
- Set by the account picker.
- Sent as primary input to `dailyos/account-overview`.
- Persisted as a block attribute.

Governed by artifact 13 for plugin routing and ADR-0111 for SurfaceClient
identity. The renderer must not display this id as user-facing copy or use it
to query WordPress-local account mirrors.

### Attribute: `composition_id`

Purpose: identifies the last successful Composition returned for this block.

Lifecycle:

- Starts blank.
- Updated after successful ability invocation.
- Used as a cache lookup key for artifact 11 projection snapshots.
- May become stale relative to substrate state.

Governed by artifact 11 and ADR-0130. It is a reference only; it cannot
authorize display without policy, freshness, and compatibility checks.

### Attribute: `composition_version`

Purpose: records the Composition contract version used by the last successful
ability output.

Lifecycle:

- Starts blank.
- Updated from successful ability responses.
- Used to decide whether a cached projection can render safely.
- Sent with render requests when compatibility negotiation needs it.

Governed by ADR-0130 conceptually and artifact 11 operationally. It is not the
block version and not the ability version.

### Attribute: `claim_refs`

Purpose: stores references to substrate claims represented by the latest
successful composition.

Lifecycle:

- Starts empty.
- Replaced after successful ability invocation.
- Used for provenance affordance resolution and feedback candidate attribution.
- Preserved during safe fallback paths when possible.

Governed by artifact 11, artifact 12, and ADR-0105. It must not contain raw
claim text, raw source snippets, or full provenance envelopes.

### Attribute: `trust_band_render_mode`

Purpose: controls trust band presentation.

Lifecycle:

- Defaults to `full`.
- Editable through InspectorControls.
- Read by both `render.php` and `edit.js`.
- Changes presentation only; it never changes ability output or trust
  assessment.

Governed by ADR-0105 and DailyOS magazine visual rules. Accepted values are
`full`, `compact`, and `icon`.

### Attribute: `dailyos_origin`

Purpose: marks the block as substrate-originated DailyOS output.

Lifecycle:

- Defaults to `substrate`.
- Used as a diagnostic and migration hint.
- Not exposed as a user-editable Phase 0 control.

Governed by ADR-0130 and artifact 13. It is not a security boundary and must
not be trusted as proof of provenance.

## Server-side render (`render.php`)

Behavior only; no PHP body is specified here.

Render sequence:

1. Read `account_id`, `composition_id`, `composition_version`, `claim_refs`,
   `trust_band_render_mode`, and `dailyos_origin`.
2. If `account_id` is blank, render an editor-safe placeholder in editor
   context and a non-disruptive placeholder on the front end.
3. Invoke `dailyos/account-overview` through WP Abilities API, then the plugin
   runtime client from artifact 13.
4. Sign runtime requests with the HMAC contract from artifact 08.
5. Apply rate-limit and observability behavior from artifact 09.
6. Construct ability input with account id, requested composition version when
   present, SurfaceClient identity, WordPress/Gutenberg surface target, and
   cache-bypass flag only for explicit refresh.
7. Receive `AbilityOutput<Composition>`. The `Composition` is the domain
   output; canonical provenance remains on the wrapper per ADR-0105.
8. Walk Composition blocks and map each block to Gutenberg-render output per
   ADR-0130 Block-to-Gutenberg mapping.
9. For every substrate block, render content, provenance affordance, trust band
   indicator, freshness cue, and any bounded fallback banner.
10. Cache the successful Composition projection as a post-meta snapshot for the
    save handler and fallback paths per artifact 11.
11. Return projected HTML for the current request.

The post-meta snapshot may include `composition_id`, `composition_version`,
`claim_refs`, sanitized projection metadata, produced-at timestamp, source
freshness summary, and request id. It must not include signing secrets, raw
runtime errors, raw unknown block payloads, per-block full provenance envelopes,
or unmasked revoked attribution.

Server render never silently rewrites `post_content` during front-end render.
Attribute updates happen through normal block save/editor flows.

Runtime error or unreachable:

- Render a recent compatible cached snapshot when available.
- Otherwise render `DailyOS surface temporarily unavailable`.
- Never render raw exception messages, raw HTTP bodies, or runtime error
  payloads.
- Surface a safe `request_id` only where appropriate for the viewer.

Stale projection handling, artifact 12 F-01:

- If cache is stale and runtime is reachable, re-invoke and replace snapshot.
- If cache is stale and runtime is unreachable, render stale snapshot with a
  `last-updated <relative time>` affordance.
- `full` and `compact` modes must show stale state visibly; `icon` mode must
  keep an accessible label.

Partial projection failure, artifact 12 F-05:

- Render resolved blocks.
- Show a bounded failure surface for unresolved blocks.
- Include request id when available.
- Do not collapse the whole account overview if independent blocks resolve.

Unknown sub-block fallback, artifact 12 F-09:

- Use the schema-bounded fallback projection from artifact 07.
- Preserve claim refs and provenance refs.
- Never render raw unknown payload fields.

Revoked provenance, artifact 12 F-02:

- Remove revoked attribution from visible output, hidden DOM, tooltips, preload
  data, and editor state.
- Keep content only if current policy allows display without the attribution;
  otherwise render a redacted or degraded claim surface.

Discovery leakage, artifact 12 F-08:

- Unpaired or unauthorized contexts must not learn ability names, schemas,
  required scopes, rate-limit classes, or detailed diagnostics.
- Placeholders remain generic until pairing and scope checks pass.

## Editor experience (`edit.js`)

Behavior only; no JSX body is specified here.

### Empty

Condition: `account_id` is blank.

Show an account picker with typeahead. Populate it from `dailyos/list-accounts`
when that ability is `client_side_executable: true`; otherwise use an
admin-ajax proxy through the PHP runtime client per artifact 13.

The picker must not expose ability discovery to unpaired clients and must not
store account-list results in post content. Selecting an account sets
`account_id`, clears incompatible `composition_id`, `composition_version`, and
`claim_refs`, then starts populated preview loading.

### Populated

Condition: `account_id` is present.

Invoke `dailyos/account-overview` and show a live preview of the returned
Composition with provenance and trust bands. The editor projection follows the
same rules as `render.php`, with additional editor chrome for selection,
retry, re-pick, feedback candidates, and authorized diagnostics.

The preview should remain visually close to front-end output so users can trust
what will publish.

### Loading

Show a skeleton state that matches the account overview layout. Avoid a spinner
that flashes; per DailyOS magazine rules, prefer subtle text affordance.

When a compatible cached preview exists, show it immediately, mark it as
refreshing, and replace it only after a successful ability response. Do not show
stale content as fresh.

### Error

Show `DailyOS unavailable — try again` with a retry button. Surface the
`request_id` from artifact 09 for diagnosis when available. Do not surface raw
exception text or raw response bodies.

If artifact 09 returns retry metadata, the retry control respects it. Repeated
clicks must not create parallel refresh storms.

### InspectorControls

Controls:

- Account picker for re-pick.
- Trust band render mode selector.
- `Refresh from substrate` button.

Re-picking account clears composition identity and claim refs, but creates no
feedback event by itself.

The trust band selector writes only `trust_band_render_mode` and should re-render
locally when possible; it does not require ability invocation.

`Refresh from substrate` forces ability re-invocation ignoring cache, remains
rate-limit aware, and updates `composition_id`, `composition_version`, and
`claim_refs` after success.

### Editing behavior

Editing text within a substrate-authored child block flags the relevant claim as
a candidate `correct` feedback event per artifact 11. Raw editor diffs never
mutate substrate state directly.

Deleting a substrate-authored child block flags the relevant claim or block as a
candidate `dismiss` feedback event per artifact 11.

Adding free-form blocks adjacent to the DailyOS block is standard Gutenberg
behavior. It creates no DailyOS feedback event and does not become part of the
DailyOS composition.

Moving the block changes WordPress layout only. Duplicating the block is allowed
only if Phase 1 resolves feedback attribution for duplicate `composition_id`
values; otherwise the editor should warn or force re-pick.

## Save (`save.js`)

`save.js` returns `null`.

This is a server-side rendered block. Save stores only attributes via
`block.json` schema. It produces no static DailyOS HTML.

The saved representation may include:

- `account_id`;
- `composition_id`;
- `composition_version`;
- `claim_refs`;
- `trust_band_render_mode`;
- `dailyos_origin`.

The saved representation must not include:

- rendered substrate HTML;
- full ability response JSON;
- raw provenance envelopes;
- raw unknown block payloads;
- raw runtime errors;
- signing material;
- account picker result sets.

This is the canonical ADR-0130 producer/renderer split: the ability produces a
`Composition`; the block stores renderer configuration and references; the
renderer projects current or cached Composition into WordPress output; the
renderer never freezes a rendered HTML payload into `post_content`.

## AbilityPolicy declarations

Picker population ability: `dailyos/list-accounts`.

Policy requirements:

- `client_side_executable: true`;
- allowed actor includes `SurfaceClient`;
- required scope is an account-list read scope;
- response is schema-bounded for picker use;
- response excludes hidden account intelligence.

Main render ability: `dailyos/account-overview`.

Policy requirements:

- `client_side_executable: true`;
- `mcp_exposure: true`;
- allowed actor includes `SurfaceClient`;
- required scope is an account-overview read scope;
- output is `AbilityOutput<Composition>`;
- no substrate mutation;
- provenance emitted per ADR-0105.

Per `/cso §0.7`, the WP plugin's custom MCP server is the only authorized MCP
exposer for this surface. The default WordPress MCP server must deny listing per
artifact 12 F-07. Unauthorized callers must not learn the ability name, input
schema, output schema, required scopes, or rate-limit class.

## Block category

Register a new block category in the plugin main class per artifact 13:

- slug: `dailyos`;
- label: `DailyOS`;
- icon: DailyOS mark.

The category is an editor affordance only. It does not imply pairing, scope
grant, or ability authorization. If the plugin is unpaired, inserting the block
shows the pairing/unavailable state while ability discovery remains denied.

## Provenance + trust band rendering

Reference ADR-0105. Each rendered claim shows a provenance affordance with source
attribution, freshness, and a detail path when policy allows. Source masking
must apply before visible output, hidden DOM, tooltips, serialized editor state,
or preload data are produced.

Composition blocks carry claim/provenance references into the canonical ability
provenance envelope. They do not duplicate full provenance envelopes per block.

Trust band values:

- `likely_current`;
- `use_with_caution`;
- `needs_verification`.

Modes:

- `full`: inline badge plus text and freshness cue.
- `compact`: small icon/badge with hover or focus detail and accessible label.
- `icon`: icon-only visual treatment with accessible label and detail path.

The block attribute `trust_band_render_mode` controls presentation only. It does
not change trust assessment, source masking, claim eligibility, or ability
output.

## Negative paths

Cross-reference artifact 12 fixtures:

- F-01 stale projection: render stale snapshot only with `last-updated <relative
  time>` when runtime is unreachable.
- F-02 revoked provenance mask: remove rendered attribution everywhere,
  including hidden and serialized surfaces.
- F-05 partial projection failure: render resolved blocks plus bounded failure
  surface for unresolved blocks.
- F-07 default WordPress MCP denial: default WP MCP server denies listing;
  plugin custom MCP server is the only authorized exposer.
- F-08 discovery leakage paths: unpaired and unauthorized clients learn no names,
  schemas, scopes, or rate-limit classes.
- F-09 custom block fallback projection: unknown sub-block falls back
  generically with no raw payload.

Additional negative paths: malformed Composition, unsupported child block
version, account no longer visible to the paired SurfaceClient, refresh blocked
by rate limit, corrupt/incompatible post-meta snapshot, and saved `claim_refs`
that no longer resolve.

## Test fixtures

Concrete fixtures:

- Happy path render: paired site, valid account, mapped Gutenberg HTML,
  provenance and trust bands present.
- Editor populate: picker loads, account selection sets attributes, preview
  invokes ability.
- Editor pick-then-refresh: force refresh ignores cache and updates composition
  attributes on success.
- Save-without-render: `post_content` has block comment and attributes but no
  substrate HTML.
- Runtime unreachable: recent snapshot renders; no raw error payload appears.
- Runtime unreachable without snapshot: unavailable placeholder renders.
- Stale snapshot fallback: stale snapshot renders with last-updated affordance.
- Revoked provenance: attribution removed after mask changes.
- Partial projection failure: resolved blocks render beside bounded failure UI.
- Unknown sub-block fallback: no raw unknown payload appears.
- Discovery denial: default WP MCP cannot list DailyOS abilities.
- Editor correction candidate: edited substrate text flags `correct`.
- Editor dismiss candidate: deleted substrate child flags `dismiss`.

## Open questions

- Should duplicated blocks sharing a `composition_id` be allowed, or should
  duplicate force a fresh invocation?
- What exact DailyOS mark should be used for the block icon and category icon?
- Should `composition_id` remain a saved attribute, or move to post-meta keyed
  by a block instance id?
- What is the block instance identity strategy for copy, paste, duplicate, and
  template reuse?
- How much provenance detail should public visitors see versus logged-in
  editors?
- Should `dailyos/list-accounts` remain an ability or become a mechanical read
  outside ADR-0102's ability boundary?
- What TTL defines a recent cached snapshot in artifact 11?
- How should a renderer behave when `composition_version` is newer but all child
  block types are known?
- Should trust band mode stay block-wide, or allow per-section overrides?
- What is the touch-device accessibility contract for provenance detail in
  `compact` and `icon` modes?
- Should feedback candidates persist as draft post meta for editor reload
  recovery, or live only in editor state until confirmation?
- Which artifact 09 diagnostics are safe for administrator, editor, and author
  roles?
