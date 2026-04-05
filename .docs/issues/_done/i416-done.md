# I416 — User Entity Navigation — Dedicated Nav Item and Route

**Status:** Open
**Priority:** P1
**Version:** 0.14.0
**Area:** Frontend / Navigation

## Summary

The user entity page (I415) needs a dedicated navigation entry and route. The YouCard identity fields are removed from the Settings page — Settings becomes exclusively technical configuration. A new `/me` route renders the user entity page. The nav item carries a subtle badge indicator when user entity fields are populated, making the nav item contextually informative.

## Acceptance Criteria

### 1. Nav item exists

A "Me" nav item exists in the main sidebar navigation. It is positioned alongside the entity nav items (Accounts, People, Projects) — not grouped with Settings or other utility items. The icon uses a person/profile icon consistent with the existing nav icon style and size.

The label "Me" is preferred for concision. If the design system's nav label treatment makes "Me" ambiguous or too short, "My Profile" is the alternative. The exact label choice should follow the existing nav label conventions (check `.docs/design/COMPONENT-INVENTORY.md` for the nav component and its label patterns).

Verify: the main sidebar renders the nav item. Clicking it navigates to `/me`.

### 2. Route exists in router.tsx

The `/me` route is registered in `router.tsx` and renders the `MePage` component (I415). The route is accessible via direct navigation (e.g., reload at `/me` does not 404).

Verify: navigate to `/me` — the user entity page renders. Reload the page — the route resolves correctly.

### 3. Settings page no longer contains identity fields

The Settings page does not contain name, company, title, or focus fields. These have moved to `/me` (I415 § About Me). Settings retains only:
- Google OAuth / integrations (Gmail, Calendar)
- Workspace configuration (workspace path, Claude Code path)
- Role preset selection (still lives in Settings — it is a system-level configuration choice, not professional context)
- Appearance (personality token, theme, font size preferences)
- Notifications and sync preferences
- System status (last sync time, connection state, storage usage)

Any Settings section or component that previously rendered the YouCard identity fields is removed or replaced with a navigation prompt: "Manage your professional context in [Me ↗]" (a link to `/me`).

Verify: open Settings — no name, company, title, or focus input fields are visible. A navigation reference to `/me` is visible in the location where the YouCard previously appeared.

### 4. No data loss during migration

The `user_name`, `user_focus`, `user_company`, and `user_title` fields in `workspace_config` continue to be read and written correctly. Only the UI location changes — from Settings to `/me`. No migration of underlying data is needed. No schema change.

Verify: with an existing app instance where these fields are populated (user_name, user_company, user_title, user_focus have values), navigate to `/me` — the About Me section shows the existing values in the name, company, title, and focus fields. Editing and saving these fields on `/me` writes the updated values back to `workspace_config`.

### 5. Nav item context indicator

The "Me" nav item carries a subtle visual indicator (a filled dot, accent mark, or equivalent consistent with the design system) when at least one user entity field is populated. The indicator signals that the user has active professional context in the system — it makes the nav item informative, not merely navigational.

The indicator is shown when: `SELECT COUNT(*) FROM user_entity WHERE value_proposition IS NOT NULL OR strategic_priorities IS NOT NULL OR company_bio IS NOT NULL` returns > 0.

The indicator is not a notification badge (no count, no urgency). It is a presence indicator — a subtle signal that this area of the app has content.

Verify: with an empty `user_entity` table, the indicator is absent from the nav item. After setting value_proposition to a non-null value, the indicator appears without page reload (or after next nav render).

## Dependencies

- Part of the same PR as I415 — the nav item and the page it routes to should ship together.
- I415 must exist (the MePage component must be implemented) before this issue can be fully verified.
- No backend dependencies — this is purely frontend routing and navigation.

## Notes / Rationale

Navigation position matters. Placing the user entity page alongside entity nav items (not inside Settings) is the signal that professional context is as important as the accounts and people the user manages. A CSM should think of `/me` the same way they think of a key account page — somewhere they visit to set context and check that the system understands their professional world correctly.

The Settings simplification is a secondary benefit. Settings currently contains a mixture of configuration (Google auth, workspace path) and identity/context (YouCard fields). Separating them makes both surfaces cleaner: Settings becomes unambiguously about system setup; `/me` becomes unambiguously about the user's professional self.
