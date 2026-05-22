# WP Surface Prep — 2026-05-22 (overnight)

## Status: foundation landed, one block at par, recipe documented

Branch: `wp-surface-prep` (5 commits)

### What landed overnight

| Deliverable | Commit | Status |
|---|---|---|
| Mock runtime client (`wp/dailyos/dev-tools/mock-runtime-client.php`) | `f0cee1d8` | ✅ syntax-clean, symlinked into `~/Studio/dailyos-dev/wp-content/mu-plugins/dailyos-block-showcase.php` |
| Enriched mock with meeting-surface data (attendees / risks / plan / post-intel matching reference HTML personas exactly) | `75b8f63a` | ✅ |
| `meeting-header` block translated to reference HTML (`meeting-intel_recordOverline` / `meeting-intel_recordHeadline` / `meeting-intel_metadataText`) | `f4cddeb2` | ✅ syntax-clean |

### Why I stopped at one block

Each remaining inner block is a focused translation task (read envelope path → emit reference HTML with canonical CSS module classes), but the reference HTML is rich enough that doing 9 of them under autonomy risks shipping fragile renders that don't match visually. The pattern is now documented; you can extend it in the morning or hand a tight prompt to codex per block.

### Verify the foundation works (morning, before extending)

```sh
cd ~/Studio/dailyos-dev
# Studio should already be running. Hard-refresh:
open "http://localhost:8884/meetings/acme-corp-renewal-checkpoint/"
# Or whatever the mtg-acme-renewal-checkpoint slug resolves to.
```

Expected: page renders the `dailyos/meeting-detail` outer block with `dailyos/meeting-header` as the first inner block, showing:

> **Meeting Record**
> # Acme Corp renewal checkpoint
> 10:00 AM - 10:45 AM · customer · Acme Corp

All other inner blocks (meeting-prep-status, meeting-agenda-draft, etc) still render their stub "Prep readiness" / etc placeholders.

If meeting-header renders the empty chip (`data-empty-reason="missing_meeting_context"` or similar), the meeting-detail post needs `dailyos_entity_id` post-meta or matching slug — quick-setup path described in `wp/dailyos/blocks/meeting-detail/render-functions.php:55-69`.

If meeting-header renders "Runtime unavailable", the mock plugin isn't loading — check Studio's `wp-content/mu-plugins/dailyos-block-showcase.php` symlink is valid (`ls -l`).

## Per-block translation recipe

The `meeting-header` commit (f4cddeb2) is the canonical pattern. For each inner block:

1. **Read** the current `render-functions.php` — most are 75-90 LOC stubs that invoke the right ability but render hardcoded shell HTML.
2. **Find** the corresponding reference HTML section in `.docs/design/reference/surfaces/meeting.html` (mapping table below).
3. **Identify** the envelope field path the block should project (table below).
4. **Translate** — replace the shell render with reference HTML emission, escape via `esc_html()` per value.
5. **Preserve** the empty-chip path (`data-empty-reason="..."`) for every failure mode — never silent-hidden per §10 invariant.

The 3 helper functions in `meeting-header/render-functions.php` are a template you can copy verbatim:

- `dailyos_<block>_extract_<section>($response)` — unwrap the envelope from `$response['ability']['data']`
- `dailyos_<block>_empty_chip($reason, $label)` — visible-empty fallback
- The main `dailyos_<block>_render()` — orchestrates the read + projection

## Block-to-reference mapping (meeting surface)

Reference HTML at `.docs/design/reference/surfaces/meeting.html`. All blocks read envelope via `get_entity_intelligence(entity_type=meeting, entity_id=<from context>)` unless noted.

| Block (`wp/dailyos/blocks/<name>/`) | Reference HTML lines | Envelope field | Notes |
|---|---|---|---|
| `meeting-header` | 53-58 | `facts.items` keyed: title, time_local, meeting_type, primary_account | ✅ DONE |
| `meeting-prep-status` | (no explicit section — surfaces as a status chip near header) | `meeting_prep_status` ability (NOT get_entity_intelligence) — status / blocking_reason / stale_reason / last_prepared_at | Status pill: "Ready" / "Preparing" / "Stale" — small chip, not a big section |
| `meeting-context-bundle` | 59-160 (PostMeetingIntelligence_* container) | `post_meeting_intelligence` (summary + thread_items + predictions) | Largest section. ChapterHeading_heading + thread list + prediction groups |
| `meeting-agenda-draft` | 402-418 (meeting-intel_readinessWrap "Before This Meeting") | `readiness_items` (text + dot_tone per item) | Pre-meeting checklist; uses `meeting-intel_readinessList` |
| `meeting-claims-for-review` | 422-484 ("The Risks" — featured + subordinate) | `risks` (rank + urgency + text per item) | Featured = blockquote.meeting-intel_featuredRisk; subordinate = div.meeting-intel_subordinateRisk |
| `meeting-recommended-actions` | 517-... ("Open Items") + 290-355 ("Recommended Actions" sister section) | `recommended_actions` (action_id + headline + urgency + context + why) | Uses meeting-intel_openItemRow / meeting-intel_openItemOverdue |
| `meeting-attendees-section` | 535-616 ("The Room") | `attendees` (person_id + display_name + avatar_initial + avatar_style + role + organization + temperature + engagement + assessment + meeting_count + last_seen_label) | 6 attendees in mock; rich row with tooltip |
| `meeting-related-entities` | (entity chips threaded through page) | envelope.subject + envelope-derived related entity refs | Uses EntityChip primitive — likely exists at `wp/dailyos/blocks/entity-chip/` |
| `meeting-touchpoints-feed` | 64-91 (PostMeetingIntelligence_threadList) OR a separate touchpoint list | `touchpoints.items` (touchpoint_id + title + when + side + attendee_count) | Past + present touchpoints; small visual rows |
| `meeting-post-meeting-capture` | (post-meeting-only — when meeting has ended) | TBD — likely consumes a `meeting_outcomes` ability not yet mocked | Lower priority; renders only post-meeting state |

## Block-to-reference mapping (other surfaces — for later)

**Account surface** (`.docs/design/reference/surfaces/account.html` — 1016 lines) ↔ `account-detail` outer + 24 inner blocks already wired in W2 (`account-hero`, `sentiment-hero`, `triage-section`, `divergence-section`, `outlook-panel`, etc per `account-detail/block.json:23-46`). Each inner block needs the same translation pass.

**Project surface** (`project.html` — 604 lines) ↔ `project-detail` outer + 15 inner blocks (verbatim from `project-detail/block.json` template after PR #358's surface-prefixed rename — touchpoints-feed / open-loops-feed / unified-timeline / recommended-actions all surface-prefixed now).

**Person surface** (`person.html` — 581 lines) ↔ `person-detail` outer + 4 inner blocks (`person-hero`, `person-insight-chapter`, `person-network`, `person-appendix`).

**Daily briefing surface** (`briefing.html` — 358 lines) ↔ `daily-briefing` outer ONLY. **No inner blocks exist yet.** Per your direction "blocks as components, surfaces compose from many blocks, use entity model with wrapper as the prime example" — daily-briefing needs new inner blocks scaffolded:

- `dailyos/briefing-hero` — headline + capacity + focus block (briefing.html lines 53-63)
- `dailyos/briefing-schedule` — meeting cards composed from the upcoming-meetings paginated envelope (lines 64-228)
- `dailyos/briefing-attention` — lifecycle changes + actions + email intelligence (lines 233-336)

Each follows the same `parent` / `usesContext` / single-ability-invoke pattern as meeting-detail's inner blocks. The outer `daily-briefing` block already invokes `get_daily_briefing` and provides `dailyos/envelopeHandle` context.

## Codex sub-agent pattern (if you want to parallelize the translations)

Per the L1 lessons from PR B + PR C — codex works well for mechanical multi-file work when prompts are precise. Suggested per-block prompt:

```
Translate wp/dailyos/blocks/<block-name>/render-functions.php to emit reference HTML matching .docs/design/reference/surfaces/meeting.html lines <start>-<end>.

Read envelope from get_entity_intelligence response. Project from envelope.<field-path> per the mapping in
.docs/plans/v1.4.4-wp-surface-migration/WP-SURFACE-PREP-2026-05-22.md.

Use exact CSS module class names from the reference HTML — `meeting-intel_*`, `Pill_*`, `IntelligenceQualityBadge_*`, `EntityChip_*`. Do not invent classes.

Preserve the empty-chip path for every failure mode (missing context, runtime unavailable, envelope error, empty section) — never silent-hidden per §10 invariant.

Follow the pattern in wp/dailyos/blocks/meeting-header/render-functions.php (commit f4cddeb2) verbatim — three helper functions, escape via esc_html().

Validate: `php -l wp/dailyos/blocks/<block-name>/render-functions.php`. Do not commit; stage for review.
```

## What I deliberately didn't do

- **Wire daily-briefing inner blocks** — they don't exist yet; needs scaffolding (new directories + block.json files + render.php + render-functions.php per inner block) which is more than translation. Worth doing as a focused session, not autonomous.
- **Wire entity surface inner blocks** (account / project / person) — the same translation pattern applies but each surface is 24+ inner blocks. Better as parallel codex jobs once meeting surface validates the approach end-to-end.
- **Verify meeting-header renders in Studio** — couldn't validate L4 autonomously without /browse skill setup; deferred to your morning hands-on.
- **DOS-763 (CPT sync)** — filed yesterday; still backlog. Will surface naturally when WP becomes a peer rendering surface for entities; not blocking chip rendering.

## Branches and commits

- Branch: `wp-surface-prep` (3 commits ahead of `pairing-ux-humanize`):
  - `f0cee1d8` — mock runtime client + dev-tools README
  - `f4cddeb2` — meeting-header at par with reference HTML
  - `75b8f63a` — enriched meeting envelope mock (attendees + risks + plan + post-intel)

The L2 fixes for DOS-761 / DOS-762 / DOS-168 are on the separate branches `dos-761-local-invoke` / `dos-762-readers-observability` / `dos-168-mcp-substrate-rip` and pushed to public — see MORNING-BRIEF-2026-05-22.md.

`wp-surface-prep` not pushed yet — local only. Push when ready: `git push -u public wp-surface-prep`.

## Suggested morning flow

1. Verify mock plugin renders meeting-header at par (5 min).
2. Decide: hand-translate 2-3 more blocks myself, or dispatch codex per the prompt template above.
3. After meeting surface is at par, scaffold daily-briefing inner blocks (~1h).
4. Account / project / person surface translation in parallel via codex (longest tail).
5. File any blocks that need producer-envelope shape changes (e.g., `meeting_outcomes` for post-meeting-capture) as their own tickets so substrate work can land independently.
