# Release Notes

Release notes for DailyOS alpha/beta versions. For alpha testers receiving builds directly.

---

## 0.7.2 (Hotfix)

**Release type:** Alpha hotfix

### What's New

**OAuth Now Shows Real Errors (I208)**
- Browser no longer says "Authorization successful" before the token exchange actually completes
- If the exchange fails, the browser shows the actual error instead of false success
- Detailed logging at every step of the OAuth flow for diagnostics
- This fixes the issue where clicking "Connect Google" appeared to succeed in the browser but the app never updated

### Full Issue List

- I208: OAuth callback shows success before token exchange completes

---

## 0.7.1 (In Progress)

**Release type:** Alpha fast-follow (security + performance hardening)

### What's New

**Focus & Priorities Now Work**
- Daily focus statement: Static user-set focus from config now flows through to dashboard and focus page
- AI-generated daily focus: Briefing enrichment now generates a contextual "one thing that matters most today" alongside the narrative
- Weekly top priority: Already working — generated during week enrichment

**Performance Improvements**
- **Enrichment no longer beach-balls the app (I173):** Database lock released before AI operations run. Manual entity enrichment now queued instead of blocking the UI. Claude subprocess runs with lower CPU priority (`nice -n 10`) so it yields to interactive work.
- **AI operations are now 15-60x cheaper (I174):** Three model tiers automatically assigned based on task complexity:
  - **Synthesis tier** (Opus/Sonnet): Daily/weekly briefings, entity intelligence
  - **Extraction tier** (Sonnet): Meeting prep, email triage, transcript processing
  - **Mechanical tier** (Haiku): Action extraction, file summaries, name resolution
  - Configurable in Settings if you want Opus everywhere

**Security**
- **OAuth credential hardening (I158):** Migration to PKCE flow (eliminates client_secret from source), macOS Keychain storage for refresh tokens (no more plaintext `token.json`)
- **Input validation (I151):** All Tauri IPC commands now validate parameters (path traversal protection, type safety)

**Reliability**
- **Calendar-to-people sync fixed (I160):** Meeting attendance now recorded correctly, last-seen dates update, meeting counts work
- **Error handling hardened (I152):** Production panics eliminated, proper Result propagation

### Breaking Changes

⚠️ **Database schema changes — migration required**

This release adds an `archived` column to the `accounts`, `projects`, and `people` tables. Your existing database will need to be migrated.

**For alpha testers (you 4):**

Since we don't have the auto-migration system yet (coming in 0.8/0.9), you have two options:

**Option 1: Nuke and rebuild (recommended if you've only been using the app for a few hours/days)**
1. Quit DailyOS
2. Delete `~/.dailyos/dailyos.db`
3. Relaunch — the app will rebuild from scratch
4. Your workspace files (markdown, captures, etc.) are untouched

**Option 2: Wait for proper migrations (if you've built up significant data)**
- The migration runner ships in 0.8.x or 0.9 beta
- Stick with 0.7.0 until then
- All your data will migrate cleanly when the system is ready

I'm in direct contact with all of you — ping me if you hit issues or want to discuss timing.

### Why This Matters

This release fixes the two biggest alpha pain points:
1. **App freezing during enrichment** — the beach ball is gone
2. **API costs** — most operations now use Haiku/Sonnet, not Opus

It also closes critical security gaps (OAuth hardening, input validation) and makes the app stable enough for beta (proper error handling, test coverage).

The breaking schema change is the trade-off for shipping archive/unarchive functionality (I160) — something multiple testers requested. After this, we're building the proper migration system so schema changes don't require nuking your DB.

### Full Issue List

**Shipped in 0.7.1:**
- I173: Enrichment responsiveness (DB lock + nice subprocess)
- I174: Model tiering (Synthesis/Extraction/Mechanical)
- I158: OAuth PKCE + Keychain storage
- I160: Calendar-to-people sync + entity archive/unarchive
- I149: Cargo clippy sweep (zero warnings)
- I150: Dependency security audit
- I151: Input validation audit
- I152: Error handling audit (eliminate panics)
- I153: Binary size + startup performance
- I154: Frontend bundle audit
- I155: Rate limiting + retry hardening
- I156: Theme toggle (DropdownMenu) broken — radix-ui migration
- I157: Frontend component audit

**Test Coverage:** 519 tests passing (up from 504 in 0.7.0)

---

## 0.7.0 (Released 2026-02-09)

**Release type:** Alpha (public GitHub release)

Initial public alpha release. DMG distributed via GitHub Releases (unsigned). OAuth-gated (4 alpha testers on allowlist).

### What Shipped

**Core Intelligence**
- Entity intelligence architecture (ADR-0057): Persistent account/project/people intelligence with incremental enrichment
- Proactive intelligence maintenance (ADR-0058): Overnight batch refresh, pre-meeting updates, email name resolution
- Entity directory templates (ADR-0059): 3-folder scaffold (Call-Transcripts, Meeting-Notes, Documents)

**Data Model**
- Three-tier architecture (ADR-0048): Filesystem (durable) + SQLite (working store) + app memory (ephemeral)
- Meeting-entity relationships with cascade (I52)
- People as useful relationship nodes (I129)

**Workflows**
- Per-operation pipelines (ADR-0042): Mechanical delivery instant, AI enrichment progressive
- Python runtime eliminated (ADR-0049): All Google API calls now Rust via `reqwest`

**Frontend**
- Schedule-first dashboard layout (ADR-0055)
- Meeting entity picker with interactive EntityPicker
- Editable people entities with account linking

### Known Issues (Fixed in 0.7.1)
- App beach-balls during enrichment (I173)
- All AI operations use expensive Opus model (I174)
- OAuth uses plaintext token storage (I158)
- Calendar-to-people sync broken (I160)

**Test Coverage:** 504 tests passing
