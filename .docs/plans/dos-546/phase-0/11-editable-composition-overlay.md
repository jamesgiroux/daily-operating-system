---
status: spec:ready
date: 2026-05-10
spike: DOS-546
phase: 0
wave: 2
artifact: 11
related_adrs: [0102, 0105, 0111, 0129, 0130]
open_questions: see ./INDEX.md (routed to W4-F and W5-A L0 Prep)
---

# 11 — Editable Composition vs user-authored overlay

## Summary

A WordPress page rendered via the DailyOS WP plugin contains TWO categories of
blocks:

1. Substrate-authored: produced by a DailyOS ability returning a `Composition`.
   These map to specific claims and have provenance.
2. User-authored overlay: free-form blocks the user added themselves (Heading,
   Paragraph, Image, Group, etc.) around or between substrate blocks.

The plugin must make this distinction visible to the editor and the save
handler, route substrate edits as feedback events to DailyOS, keep user overlays
local to WP, and handle insertion, deletion, nesting, paste, and reordering
across the boundary.

ADR-0130's producer/renderer split is the governing rule: DailyOS abilities
produce a `Composition`; Gutenberg renders it. User layout work in Gutenberg
does not become substrate authorship unless it is converted into typed feedback.

## Design principle

`post_content` is a mixed projection, not the system of record for substrate
content.

The save handler may see substrate projections and local WP blocks in the same
tree. It must not submit raw Gutenberg diffs to DailyOS. Only a validated edit
to a stored Composition block, mapped to a claim ref and field path, may become a
feedback event.

Block attributes are routing hints, not a security boundary. The authoritative
decision uses:

- The per-post stored Composition snapshot.
- The current runtime `composition_version` watermark.
- The claim refs and field paths recorded in the Composition.
- A fresh user-presence nonce for each feedback event.

If any check fails, the plugin keeps the saved WP content local and emits no
DailyOS feedback.

## Block attribute schema

Substrate-authored blocks carry attributes:

```json
{
  "dailyos_origin": "substrate",
  "dailyos_composition_id": "comp_01JZ8X3P7PCQZFF2HPX6JHQZ4R",
  "dailyos_composition_version": "composition:v17:sha256:0f2c...",
  "dailyos_claim_refs": [
    { "claim_id": "claim_01JZ8YF8T2SQ8N2YY4YWD3H8EB", "field_path": "summary.text" }
  ],
  "dailyos_block_id": "block_account_health_summary"
}
```

| Attribute | Required | Purpose |
| --- | --- | --- |
| `dailyos_origin` | Yes | Marks the block as a DailyOS substrate projection. |
| `dailyos_composition_id` | Yes | Links to the stored Composition snapshot. |
| `dailyos_composition_version` | Yes | Carries the freshness watermark observed by the editor. |
| `dailyos_claim_refs` | Yes | Names the claim fields represented by the block. |
| `dailyos_block_id` | Yes | Stable id inside the Composition for diff lookup. |

User-authored blocks have none of these attributes. The plugin may explicitly
stamp `dailyos_origin: "user"` on plugin-created local scaffolding, but missing
origin and `"user"` are equivalent: local WP content, no DailyOS feedback.

Attributes must not contain raw source excerpts, full provenance envelopes,
secrets, HMAC material, bearer tokens, user-presence nonces, runtime error
bodies, or hidden DailyOS policy data.

## Origin classification

The save handler classifies every block:

| Class | Condition | Routing |
| --- | --- | --- |
| `substrate_matched` | `dailyos_origin == "substrate"` and snapshot contains the same `composition_id` + `block_id`. | Eligible for diff and feedback. |
| `substrate_unmatched` | Origin says substrate, but no matching snapshot exists. | Persist locally; warn; no feedback. |
| `user_overlay` | Origin missing or `"user"`. | Persist locally; no feedback. |
| `tampered_origin` | DailyOS attrs are malformed, partial, or inconsistent. | Persist locally; audit warning; no feedback. |

`substrate_unmatched` is deliberately non-mutating. A copied block, stale block,
or edited block comment must not be enough to submit feedback against a claim.

## Editor distinction (Gutenberg UX)

The editor renders a subtle marker on substrate-authored blocks:

- A small DailyOS/provenance badge in block toolbar or InspectorControls.
- A quiet edge stripe or equivalent boundary marker on selection/focus.
- Claim/provenance details through the normal provenance affordance.
- No large frames, warning-color chrome, dashboard panels, or permanent labels
  that compete with the magazine reading hierarchy.

This follows DailyOS magazine visual rules: typography, spacing, and reading
rhythm do the work; decoration stays minimal. This section is a behavior spec,
not a CSS prescription.

Required behaviors:

- Editing the content of a substrate-authored block flags a candidate
  correction.
- Editing a substrate claim value attribute flags a candidate correction.
- Deleting a substrate-authored block flags a candidate dismissal.
- Adding a free-form block next to substrate content emits no feedback and saves
  as a normal WP block.
- Editing a free-form block emits no feedback.
- Reordering substrate blocks emits no feedback in v1.
- Moving a substrate block into or out of user-created groups preserves its
  substrate identity but emits no layout feedback in v1.

The editor may add a save-time review UI for candidate corrections and
dismissals, but Phase 0 does not require it. The minimum contract is
deterministic save-handler routing.

## Composition version watermark

Every ability invocation stamps the returned `Composition` with
`composition_version`. The watermark is copied into every substrate block's
attributes and stored in the per-post snapshot.

On save, the handler compares the saved watermark to the current substrate
version for the same `composition_id`:

- Match: feedback is fresh and may be submitted.
- Mismatch: reject feedback with `version_skew` and return the current
  Composition for editor re-merge.

Suggested editor notice:

```text
This DailyOS content was updated elsewhere. Review the latest version before
applying your edits.
```

User-authored overlay blocks remain in the post during re-merge. The editor
re-renders the latest substrate blocks and attempts to preserve local overlay
positions around them. It must not silently apply stale corrections to newer
substrate claims.

## Save-handler routing logic

The routing logic runs on explicit Gutenberg post save or the plugin's
server-side save hook. Autosaves may record local drafts, but must not submit
DailyOS feedback unless the implementation treats the autosave as an explicit
user-visible update and obtains user-presence nonces.

Algorithm:

1. Load `_dailyos_composition_snapshot` for the post.
2. Parse the saved Gutenberg block tree with WordPress block parsing APIs.
3. Walk the tree depth-first.
4. For each block with `dailyos_origin == "substrate"`:
   - Validate `dailyos_composition_id`, `dailyos_block_id`,
     `dailyos_composition_version`, and `dailyos_claim_refs` against the
     snapshot.
   - If validation fails, classify as `substrate_unmatched` or
     `tampered_origin`; no feedback.
   - If validation succeeds, diff saved editable content and claim value
     attributes against the original snapshot block.
   - If unchanged, emit no feedback.
   - If changed, create a `correct` candidate per changed field path.
5. Detect deleted substrate blocks by comparing snapshot block ids to valid
   substrate block ids present in the saved post.
6. For each deleted substrate block, create a `dismiss` candidate with reason
   `removed_from_layout`.
7. For unmarked or `dailyos_origin == "user"` blocks, persist normal WP content
   and do not call DailyOS.
8. Detect substrate reorder for diagnostics only; emit no feedback in v1.
9. Obtain or validate a fresh user-presence nonce for each feedback candidate,
   bound to user, session, action, claim id, field path, and
   `composition_version` per artifact 10.
10. Submit all feedback events as a single batched `POST /v1/feedback` to the
    runtime endpoint from artifact 15.
11. On success, store the updated Composition returned by the runtime as the new
    substrate snapshot for this post.
12. On rejection, keep the WP save local, surface the rejection, and do not
    mutate the snapshot except for an explicit `version_skew` re-merge response.

`correct` event:

```json
{
  "action": "correct",
  "composition_id": "comp_a",
  "composition_version": "v1",
  "block_id": "b_summary",
  "claim_id": "c_1",
  "field_path": "summary.text",
  "before": "Acme risk is rising.",
  "after": "Acme risk is stabilizing.",
  "presence_nonce_attr": "<opaque>",
  "presence_nonce_body": "<opaque>"
}
```

`dismiss` event:

```json
{
  "action": "dismiss",
  "reason": "removed_from_layout",
  "composition_id": "comp_a",
  "composition_version": "v1",
  "block_id": "b_summary",
  "claim_id": "c_1",
  "field_path": "summary.text",
  "presence_nonce_attr": "<opaque>",
  "presence_nonce_body": "<opaque>"
}
```

## What becomes feedback vs what stays local

| Action | Feedback to DailyOS? | Reason |
| --- | --- | --- |
| Edit substrate text | Yes — `correct` | Substrate field changed. |
| Edit substrate value (e.g., claim `value` attr) | Yes — `correct` | Substrate field changed. |
| Delete substrate block | Yes — `dismiss` | Author of substrate removed it from rendered layout. |
| Reorder substrate blocks within their region | No v1 | Layout is renderer-side per ADR-0130. |
| Add user block adjacent | No | User overlay, local to WP. |
| Edit user block | No | User overlay, local to WP. |
| Move substrate block out of its region | No v1; flag for v2 review | Risky semantic layout edit. |

The runtime receives typed feedback only. Raw editor-side content diffs never
become substrate writes.

## Edge cases

- Substrate block inside a user-added Group block: still substrate; routing
  still applies to the inner block; the Group remains local WP overlay.
- User pastes content into a substrate Paragraph: treated as `correct` on the
  mapped substrate field; the normalized pasted value becomes `after`.
- Multiple claim refs in one editable region: the block must expose a
  deterministic field mapping or reject feedback as ambiguous.
- Two users editing the same page: WP locking handles normal contention;
  `version_skew` catches stale substrate feedback.
- Substrate Composition removed entirely from post content: emit one batched set
  of `dismiss` events with reason `composition_removed_from_layout`, deduped by
  claim field where possible.
- Page deleted entirely: no feedback. Deletion of renderer state is not
  substrate dismissal. Delete the DailyOS post meta.
- Copied substrate block pasted into another post: classify as
  `substrate_unmatched` unless the destination post has a compatible snapshot;
  no feedback.
- Reusable blocks and synced patterns: disable for substrate content in Phase 0;
  if encountered, route only when the concrete post snapshot matches.
- Block converted to HTML: if attributes are lost, it becomes user overlay; if
  partial attributes survive, classify as `tampered_origin`; no feedback.
- Autosave: persist local draft content, but do not submit feedback unless the
  user-presence nonce requirements are explicitly satisfied.

## Storage

Per post, store `_dailyos_composition_snapshot` containing the last-rendered
Composition projection keyed by `dailyos_composition_id`.

Schema:

```json
{
  "comp_a": {
    "composition_id": "comp_a",
    "composition_version": "v1",
    "schema_version": 1,
    "ability": { "name": "dailyos/account-overview", "version": "0.1.0" },
    "rendered_at": "2026-05-10T22:30:00Z",
    "request_id": "018f4e8f-7f57-7c7c-9cb4-08fc4b753b92",
    "blocks": {
      "b_summary": {
        "block_type": "ClaimSummary",
        "normalized_content": "Acme risk is rising.",
        "claim_refs": [{ "claim_id": "c_1", "field_path": "summary.text" }],
        "content_hash": "sha256:4a9b..."
      }
    }
  }
}
```

Storage rules:

- Store enough normalized original content to compute diffs.
- Store claim refs, field paths, block ids, Composition watermark, request id,
  and rendered timestamp.
- Do not store HMAC keys, bearer tokens, nonces, raw runtime errors, full hidden
  provenance envelopes, revoked attribution, or unmasked source snippets.
- On post deletion, delete `_dailyos_composition_snapshot`.
- On successful feedback, replace the affected Composition snapshot with the
  runtime-returned Composition.

## Test fixtures

Fixtures use simplified Gutenberg-like JSON. Implementers should convert them
into WordPress parser fixtures and runtime endpoint assertions.

### F-01 edit substrate text emits `correct`

Snapshot block `b_summary`: content `Acme risk is rising.`, claim ref
`c_1:summary.text`, version `v1`.

Saved block:

```json
{ "blockName": "core/paragraph", "attrs": { "dailyos_origin": "substrate", "dailyos_composition_id": "comp_a", "dailyos_composition_version": "v1", "dailyos_block_id": "b_summary", "dailyos_claim_refs": [{ "claim_id": "c_1", "field_path": "summary.text" }] }, "innerHTML": "Acme risk is stabilizing." }
```

Expected: one `correct` for `c_1 summary.text`, before `Acme risk is rising.`,
after `Acme risk is stabilizing.`, with a valid nonce.

### F-02 edit substrate value emits `correct`

Snapshot block `b_score`: normalized content `72`, claim ref
`c_2:health.score`, version `v1`.

Saved `dailyos/account-overview` block has the same DailyOS attrs and
`"value": 81`.

Expected: one `correct` for `c_2 health.score`, before `72`, after `81`.

### F-03 delete substrate block emits `dismiss`

Snapshot contains `b_summary` mapped to `c_1:summary.text`. Saved post contains
no valid block with `dailyos_block_id: "b_summary"`.

Expected: one `dismiss`, reason `removed_from_layout`, block `b_summary`, claim
`c_1`, field `summary.text`.

### F-04 reorder substrate blocks emits nothing

Snapshot order is `b_1`, `b_2`. Saved post order is `b_2`, `b_1`, with unchanged
content and matching attrs.

Expected feedback: `[]`. Optional local diagnostic: `layout_reordered`.

### F-05 add user block adjacent emits nothing

Saved post contains an unmarked `core/paragraph` with `My local note.` adjacent
to unchanged substrate block `b_summary`.

Expected feedback: `[]`. Expected persistence: both blocks remain in
`post_content`.

### F-06 edit user block emits nothing

User block changes from `My local note.` to `My revised local note.` and has no
DailyOS attrs.

Expected feedback: `[]`. Expected DailyOS calls: none.

### F-07 move substrate block out of its region emits nothing v1

Same substrate block `b_summary` moves below a user Heading, with unchanged
content and matching snapshot.

Expected feedback: `[]`. Optional diagnostic: `substrate_region_moved`.

### F-08 version skew rejects feedback

Snapshot and saved block carry version `v1`. Runtime current Composition for
`comp_a` is `v2`. User edits `b_summary`.

Expected runtime response:

```json
{ "error": "version_skew", "current_composition": { "composition_id": "comp_a", "composition_version": "v2", "blocks": [] } }
```

Expected plugin behavior: keep local WP edit, do not update substrate, prompt
for re-merge against `v2`.

### F-09 paste into substrate paragraph emits `correct`

Snapshot content: `Renewal risk is moderate.`

Saved substrate paragraph content: `Renewal risk is high because procurement
paused the order.`

Expected: one `correct`; `after` is the full normalized pasted text; nonce is
bound to action `correct`, the claim id, field path, and version.

### F-10 substrate inside user Group still routes

Saved `core/group` has no DailyOS attrs. Its child paragraph has valid substrate
attrs for `b_summary` and edited content.

Expected: one `correct` for the child claim field. The outer Group remains local
WP overlay.

### F-11 page deleted emits no feedback

Action: post moves to trash or is permanently deleted.

Expected feedback: `[]`. Expected cleanup:
`_dailyos_composition_snapshot` removed.

### F-12 copied substrate block without snapshot emits no feedback

Destination post has no snapshot for `comp_a`, but saved content contains a
block copied with `dailyos_origin: "substrate"` and `dailyos_block_id:
"b_summary"`.

Expected classification: `substrate_unmatched`. Expected feedback: `[]`.
Expected local behavior: persist as local content or downgrade to user overlay
with editor warning.

## Interaction with other Wave 2 artifacts

- 10 user-presence nonce — every emitted feedback event carries a nonce bound
  to the editing user, action, claim field, and Composition watermark.
- 12 negative fixtures — stale, revoked, unknown, unauthorized, and failure-mode
  cases should include this origin classification.
- 13 WP plugin skeleton — owns save hook wiring, post meta storage, and WP
  capability checks before runtime calls.
- 14 Gutenberg block design — first concrete block; declares the
  `dailyos_origin` attributes.
- 15 runtime endpoint — receives the batched feedback POST and returns updated
  Composition snapshots or `version_skew`.

## Open questions

1. Should v1 require an explicit review UI before substrate edits become
   feedback, or is save-time routing enough for the spike?
2. Should reordering ever become feedback, or should layout remain permanently
   renderer-local under ADR-0130?
3. How much normalized original content can be stored in post meta before the
   snapshot needs encryption or runtime-side storage?
4. Should copied substrate blocks be automatically downgraded to user-authored
   blocks, or preserve the marker with a warning?
5. How should synced patterns and reusable blocks participate after Phase 0?
6. Should dismissing a block with multiple claim refs emit one dismissal per
   field path or a single aggregate dismissal?
7. What is the final UX for `version_skew` re-merge when user overlays sit
   between changed substrate blocks?
8. Should autosaves ever mint user-presence nonces, or should feedback be
   restricted to explicit publish/update actions?
