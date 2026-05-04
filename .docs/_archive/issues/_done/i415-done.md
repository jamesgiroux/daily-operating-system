# I415 — User Entity Page — Dedicated Professional Context Surface

**Status:** Open
**Priority:** P1
**Version:** 0.14.0
**Area:** Frontend / Entity

## Summary

The user entity moves out of Settings into its own dedicated page — the same editorial entity detail treatment as accounts and people. This is the user's professional workspace: where they tell DailyOS about their company, their product, their priorities, their methodology, and their professional knowledge. The page is primarily authored (user writes, AI consumes) rather than primarily read (AI generates, user corrects). It has six sections, a context entries input, and a document attachment dropbox. Role presets shape section prominence and vocabulary. v0.14.0 implements the CS preset fully with the generic structure available to all other presets; v0.14.1 expands preset-specific vocabulary to all nine presets.

The page component is `src/pages/MePage.tsx`. Route: `/me`.

## Acceptance Criteria

### 1. Navigation and routing

A "Me" nav item exists in the main sidebar alongside Accounts, People, and Projects. It routes to `/me`. Settings no longer contains the YouCard identity section — it retains only technical configuration (integrations, workspace path, notifications, appearance, role preset selection, system status).

Verify: navigate to `/me` — the user entity page loads. Open Settings — name, company, title, and focus fields are absent from the Settings page. The route exists in `router.tsx`.

### 2. § About Me section

The About Me section renders with all fields: Name, Title, Company (migrated from Settings but reading/writing to `workspace_config` via the existing fields), Company Bio (new — one paragraph, reading/writing to `user_entity.company_bio`), Role Description (new — `user_entity.role_description`), How I'm Measured (new — `user_entity.how_im_measured`).

Each field uses `EditableText` and saves on blur via the appropriate command (`update_user_entity_field` for new fields; the existing `workspace_config` update path for name/title/company/focus). No separate Save button.

Placeholder text is specific and role-appropriate. For CS preset examples:
- Company Bio: "One paragraph — what does your company do and who does it serve? The 30-second explanation, not the marketing tagline."
- Role Description: "What do you actually do day-to-day? What are you responsible for?"
- How I'm Measured: "What KPIs or outcomes define success for you? NRR, GRR, CSAT, renewal rate?"

This section is Featured (expanded by default) for all presets.

Verify: edit Company Bio, click outside — `SELECT company_bio FROM user_entity` returns the updated value. Name and Company fields continue writing to `workspace_config` (no regression).

### 3. § What I Deliver section

Fields rendered: Value Proposition (EditableText), Success Definition (EditableText), Product Context (EditableText), Pricing Model (EditableText — placeholder clarifies "category-level only, not exact figures"), Differentiators (EditableList — each item is a single differentiator), Common Objections (EditableList — each item is a Q&A pair with a question field and response field), Competitive Context (EditableText).

All fields save on blur via `update_user_entity_field`. Differentiators save as a JSON array of strings. Common Objections save as a JSON array of `{"question": "...", "response": "..."}` objects.

For CS preset: Value Proposition and Success Definition are featured (expanded, first field immediately visible). Pricing Model is shown (not hidden). Common Objections is featured.

For all other presets in v0.14.0: all fields are available but the section defaults to collapsed (header visible, expand affordance present).

Verify: add a differentiator item — `SELECT differentiators FROM user_entity` returns a JSON array containing the item. Add a common objection — `SELECT objections FROM user_entity` returns a JSON array of Q&A objects.

### 4. § My Priorities section — two-layer

The section renders two clearly visually distinct sub-sections.

**Strategic Priorities — "This Quarter"**

A list of named priority items. Each item displays:
- The priority text
- An optional entity chip (account, project, or person name) if `linked_entity_id` is populated — chip is clickable and navigates to that entity
- A delete affordance (removes the item from the array)

An "Add annual priority" input row at the bottom. Entity linking uses the existing entity picker component. Adding or removing an item saves the full updated array via `update_user_entity_field("annual_priorities", ...)`.

**Quarterly Priorities — "This Quarter"**

Same visual treatment as annual but framed as "This Quarter." Items show an optional linked action, meeting, or person chip if `linked_to_id` is populated. No automatic expiration — items persist until the user removes them. No "reset" action. When the user wants to start a new quarter, they simply edit, add, and remove items as they see fit. Zero-guilt: the system uses whatever is here; it never nags the user to update.

Verify: add an annual priority linked to a known account — the entity chip appears. `SELECT annual_priorities FROM user_entity` shows the item with `linked_entity_id`. Add a quarterly priority — item appears in the list and `SELECT quarterly_priorities FROM user_entity` shows it. Remove a quarterly priority — it is gone from both the UI and the stored JSON.

### 5. § My Playbooks section

For CS preset: three named sections are rendered as separate EditableText fields:
- "At-Risk Accounts" — placeholder: "How do you approach accounts showing health decline or disengagement? What's your standard play?"
- "Renewal Approach" — placeholder: "How do you manage the renewal cycle? What's your process from 90 days out?"
- "EBR/QBR Preparation" — placeholder: "How do you prepare for and structure executive business reviews?"

For all other presets in v0.14.0: one generic section, "My Methodology," with a generic placeholder: "How do you approach your work? Describe your recurring patterns, standard plays, and professional judgment calls."

Each named section saves to its named key in `user_entity.playbooks` (stored as a JSON object: `{"at_risk": "...", "renewal": "...", "ebr": "..."}`). The generic section saves as a flat string for non-CS presets.

Verify: with CS preset active, three named sections are visible. Switch to a non-CS preset in Settings — one generic section is shown.

### 6. § Context Entries section

A list of existing entries. Each entry in the list shows: title (bold), truncated content preview (two lines), creation date. An entry can be expanded inline to show full content and an edit affordance. A delete icon is present on each entry.

A "+ Add context entry" button opens an inline form directly below the list (not a modal). The form has a title field and a content textarea. On save (button or Cmd+Enter), `create_user_context_entry(title, content)` is called. The form dismisses and the new entry appears at the top of the list.

Editing an existing entry opens the same inline form with existing values populated. On save, `update_user_context_entry(id, title, content)` is called.

The embedding state is NOT shown to the user. No "Processing" badge, no spinner, no "embedding in progress" state. The entry appears in the list immediately after save. Embedding is background infrastructure.

Verify: create a context entry — it appears in the list without page reload. After 2–3 minutes, `SELECT embedding_id FROM user_context_entries WHERE title = '<title>'` returns a non-null value.

### 7. § Attachments section — document dropbox

A drag-and-drop zone labelled "Drop files here — product decks, playbooks, case studies, battlecards." The zone accepts PDF and Markdown files. Dropped files are written to `<workspace_path>/_user/docs/` via a Tauri command.

Below the drop zone: a list of already-attached files showing filename, date added, and a delete affordance (removes the file from `_user/docs/` and from the displayed list).

Verify: drop a PDF onto the zone — it appears in the file list and exists at `<workspace_path>/_user/docs/<filename>`. After the file watcher/processor runs (up to 2 minutes), the file appears in `content_files` with `source = 'user_context'`.

### 8. Activity indicator

A single line of contextual text near the top of the page, below the page heading: "Your context is actively shaping intelligence for N accounts." The count N is the number of entity records that received a `user_relevance_weight > 1.0` in the last signal scoring cycle (I414).

If all user entity fields are null/empty AND no strategic priorities are set, this line does not appear. It only appears when there is actual user context influencing the system.

Verify: populate `current_priorities` with the name of a known account. After one signal scoring cycle (I414), reload the page — N > 0. The account mentioned is among those with `user_relevance_weight > 1.0` in `entity_intel`.

### 9. Role preset shaping

The currently active role preset (from `workspace_config`) shapes which sections are featured vs. collapsed:
- "Featured" = section header visible and first field immediately visible (expanded)
- "Collapsed" = section header visible only; content is hidden behind an expand affordance

For CS preset: About Me, What I Deliver (Value Proposition + Success Definition subsection), My Priorities, and My Playbooks are Featured. Context Entries and Attachments are Shown (collapsed, expand available).

For all non-CS presets in v0.14.0: About Me and My Priorities are Featured. All other sections are Shown (collapsed). This ensures the page is usable and not overwhelming for users on presets not yet fully implemented.

Verify: with CS preset active — What I Deliver section is expanded on page load. Switch to a non-CS preset in Settings, navigate back to `/me` — What I Deliver section is collapsed.

### 10. Design system compliance

All components on this page use existing tokens, typography, and layout patterns. Specifically:
- Section headers use the editorial section rule treatment (`section-rule` class or equivalent per `.docs/design/DESIGN-SYSTEM.md`)
- Body text uses existing design token variables (`--font-body`, `--color-ink`, etc.) — no hardcoded hex values
- No new typography variants introduced
- EditableText and EditableList components are the existing shared components, not new implementations
- Spacing uses existing token steps, not arbitrary pixel values

Run: `grep -c "style={{" src/pages/MePage.tsx` — the count should be minimal and limited to genuinely dynamic values (computed widths, animation transforms, or truly one-off calculated values). Static design values must use CSS classes or tokens, not inline styles.

## Dependencies

- Blocked by I411 (user entity table and commands must exist before the page can read/write data).
- Blocked by I416 (navigation item and route).
- Requires I412 for the activity indicator (criterion 8) — the indicator reads entity intel user-relevance weights produced by I412/I414. If I412 is not yet merged, criterion 8 can render "—" or be omitted in the initial PR.
- See ADR-0090 for the full design rationale and section prominence table.

## Notes / Rationale

The page is the first DailyOS surface that is primarily authored rather than primarily read. This shapes every design decision: emphasis on good placeholder text (users need to know what good input looks like), progressive disclosure (empty sections don't intimidate), inline editing with immediate feedback (no form/save-button friction). The editorial magazine aesthetic still applies — clean typography, generous whitespace, section rules — but the interaction density is higher than a read-only intelligence page.

The YouCard component (previous I415 scope) is superseded by this page. The identity fields that were in YouCard (name, company, title, focus) are now in § About Me on this page, reading/writing the same underlying fields in `workspace_config`. YouCard as a widget is retired; the user entity page is its replacement and expansion.
