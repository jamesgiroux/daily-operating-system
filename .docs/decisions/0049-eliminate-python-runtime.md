# ADR-0049: Eliminate Python runtime dependency

**Date:** 2026-02-08
**Status:** Accepted

## Context

DailyOS currently requires Python 3.10+ at runtime to execute Phase 1 data-fetching scripts (`prepare_today.py`, `calendar_poll.py`, `refresh_emails.py`, etc.) and the Google OAuth flow (`google_auth.py`). These scripts call the Google Calendar and Gmail APIs via `google-api-python-client`, and are invoked from Rust as subprocesses via `run_python_script()` in `pty.rs`.

This creates significant distribution friction:

1. **macOS hasn't shipped Python since Catalina (10.15).** Users need Homebrew or another source.
2. **Version constraint:** Google's client library requires Python 3.10+, but many systems have 3.9.
3. **pip dependency chain:** `google-auth`, `google-auth-oauthlib`, `google-api-python-client` must be installed separately. No `requirements.txt` exists.
4. **Two-language stack:** Every Python subprocess spawn crosses a process boundary, loses type safety, and communicates via stdout JSON parsing.
5. **Bundle bloat:** `scripts/` directory is bundled into the DMG via Tauri resources.

Meanwhile, the Rust side has steadily absorbed Python's responsibilities:
- ADR-0042 (I38) replaced `deliver_today.py` entirely with `workflow/deliver.rs`
- AI enrichment (emails, briefing, preps, inbox, transcripts, accounts) runs 100% through Rust → Claude Code CLI
- ID generation, gap analysis logic already ported
- `google.rs` already has calendar polling orchestration (but delegates to Python for the actual API call)

The remaining Python surface is ~2,400 lines across 8 scripts and 7 ops modules. Every external API call is to Google (Calendar v3, Gmail v1) — standard REST APIs.

## Decision

Eliminate the Python runtime dependency by porting all Google API interactions to native Rust using `reqwest` for HTTP and manual OAuth2 token management.

**Approach:**

1. **New `google_api.rs` module** — OAuth2 token storage/refresh, localhost redirect server for initial auth, Calendar v3 event listing, Gmail v1 message listing/fetching. Uses `reqwest` (async HTTP) + `serde` (JSON parsing) + `tokio::net::TcpListener` (OAuth callback server).

2. **Port operations modules** — Meeting classification, email priority classification, action parsing from markdown, meeting prep context gathering. These are pure logic + SQLite + file I/O — all things Rust already does elsewhere in the codebase.

3. **Port orchestrators** — `prepare_today`, `prepare_week`, `deliver_week`, `refresh_emails`, `calendar_poll`, `prepare_meeting_prep` become Rust functions composed from the operations above.

4. **Delete Python** — Remove `scripts/` directory, `run_python_script()` from `pty.rs`, script resources from `tauri.conf.json`, Python check from onboarding.

**Why not use `google-calendar3` / `google-gmail1` crates?** They're auto-generated, pull in heavy dependency trees, and abstract away the REST API in ways that make debugging harder. The Calendar and Gmail APIs we use are 2-3 endpoints each — `reqwest` calls are simpler and more maintainable.

**Why not bundle Python in the DMG?** Adds 50-100MB, requires managing a venv, and perpetuates the two-language split for no architectural benefit.

## Consequences

**Easier:**
- Distribution: DMG is self-contained, no Python/pip prerequisite
- Testing: all data-fetching logic is testable in Rust's type system
- Error handling: no stdout JSON parsing, no subprocess failure modes
- Performance: no process spawn overhead for API calls
- Onboarding: one fewer dependency to validate (Claude Code remains the only external CLI)

**Harder:**
- OAuth2 flow implementation in Rust (one-time cost, ~200 lines)
- Porting ~2,400 lines of Python logic (mostly straightforward — the logic isn't complex, just voluminous)
- Google API response parsing needs Rust struct definitions (but gains type safety)

**Trade-offs:**
- Adds `reqwest` as a dependency (but `tokio` is already present)
- Loses the ability to iterate on Python scripts without recompiling (but we haven't changed them in months — they're stable)
- The `scripts/` directory becomes historical reference only (move to `_archive/` or delete)

**Supersedes:** ADR-0006 (determinism boundary) described Python as the Phase 1/3 executor. Phase 3 was already eliminated by ADR-0042. This eliminates Phase 1, making the three-phase pattern fully Rust-native. The conceptual boundary (fetch → enrich → deliver) remains; only the implementation language changes.
