# Release Checklist

Pre-push checklist for DailyOS releases. Complete every section before tagging and pushing to `main`.

**Release cadence:** Weekly train, Tuesday late morning ET. See `RELEASE-POLICY.md` for the full schedule, hotfix criteria, and operating rules. This checklist runs on Monday afternoon (acceptance) and Tuesday morning (final validation).

---

## 0. Train Readiness

- [ ] All issues assigned to this train are complete and committed on `dev`
- [ ] Monday noon feature cutoff observed — no new work after cutoff
- [ ] Release notes draft reviewed — changes grouped by user value area (Meetings, Accounts, Actions, Email, Polish)
- [ ] No open hotfix-worthy issues from the previous train

---

## 1. Version Bump

- [ ] Bump version in `src-tauri/tauri.conf.json`
- [ ] Bump version in `src-tauri/Cargo.toml` (triggers `Cargo.lock` update)
- [ ] Bump version in `package.json`
- [ ] All three versions match

## 2. Changelog & Documentation

- [ ] `CHANGELOG.md` entry added for the new version with today's date
- [ ] Entry follows Keep a Changelog format (sections: Added, Changed, Fixed, Removed, Security as needed)
- [ ] Every user-facing change has a line item — no silent changes
- [ ] **`release-notes.md` entry added** — user-facing, product marketing language. This is what appears in the What's New modal. Lead with the story of the release, not a list of issues. No internal jargon, no issue numbers, no "entity intelligence" or "enrichment". Write like you're telling a customer what got better. See existing entries for format.
- [ ] `README.md` updated if the release changes installation steps, requirements, or core features
- [ ] Architecture Decision Records created for any new architectural decisions (`.docs/decisions/`)

## 3. Dependency Updates

Run these BEFORE build verification. CI runs `pnpm audit --audit-level high` and will fail on unpatched vulnerabilities.

- [ ] `pnpm update` — update all packages within semver ranges
- [ ] `pnpm audit --audit-level high` — zero high/critical vulnerabilities. Fix any that appear before proceeding.
- [ ] `pnpm outdated` — review available updates. Apply security-relevant patches. Defer major version bumps unless needed.
- [ ] `cargo update --manifest-path src-tauri/Cargo.toml` — update Rust deps within semver ranges
- [ ] `cargo audit --file src-tauri/Cargo.lock` — zero known vulnerabilities (or documented exceptions). Install with `cargo install cargo-audit` if missing.
- [ ] Commit lockfile updates before build verification

## 4. Build Verification

- [ ] `pnpm install` — clean install succeeds with no warnings
- [ ] `pnpm build` — frontend builds without errors or TypeScript failures
- [ ] `pnpm build:mcp` — MCP sidecar binary builds and lands in `src-tauri/binaries/` (script creates a stub first to satisfy Tauri's build.rs, then overwrites with the real binary — if this step fails with "resource path doesn't exist", the stub creation is broken)
- [ ] `pnpm tauri build --target aarch64-apple-darwin` — full app bundle succeeds
- [ ] DMG opens and installs to `/Applications` cleanly
- [ ] Verify `DailyOS.app/Contents/MacOS/` contains both `dailyos` and `dailyos-mcp`
- [ ] App launches from `/Applications` (not from build directory)

## 5. Rust Backend

- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` — all tests pass
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` — zero warnings
- [ ] `cargo audit --file src-tauri/Cargo.lock` — no known vulnerabilities (or documented exceptions)
- [ ] No new `unwrap()` or `expect()` in IPC command handlers (use `Result` propagation)
- [ ] Database migrations are forward-compatible and idempotent

## 6. Frontend

- [ ] `pnpm test` — all Vitest tests pass
- [ ] `pnpm audit` — no high/critical vulnerabilities (or documented exceptions)
- [ ] No TypeScript `// @ts-ignore` or `any` casts added without justification
- [ ] No `console.log` left in production code (use structured logging)

## 7. Security Review

- [ ] No secrets committed — search for `client_secret`, `api_key`, `password`, `token` in diff
- [ ] No hardcoded credentials or API keys in source
- [ ] `option_env!` used for build-time secrets (e.g., `DAILYOS_GOOGLE_SECRET`)
- [ ] CSP in `tauri.conf.json` unchanged (or changes are intentional and documented)
- [ ] IPC commands validate all input parameters (no path traversal, no injection)
- [ ] OAuth flow uses PKCE with S256 challenge
- [ ] Keychain storage for tokens — no plaintext token files
- [ ] `reveal_in_finder` and `copy_to_inbox` path validation intact

## 8. Performance Audit

- [ ] App cold launch to usable dashboard: under 3 seconds
- [ ] Hot read commands (status, focus): p95 under 100ms
- [ ] Dashboard load: p95 under 300ms
- [ ] No DB lock held across AI calls, network calls, or filesystem scans (split-lock pattern)
- [ ] AI subprocess runs with `nice -n 10` (yields to interactive work)
- [ ] Background tasks open own SQLite connections (not competing for shared Mutex)
- [ ] No regressions to binary size (compare against previous release)

## 9. Logic Tests (Does It Do What It Should)

- [ ] **Onboarding:** Fresh install → onboarding wizard completes → Google OAuth connects → first briefing generates
- [ ] **Daily briefing:** Click refresh → workflow progresses through Preparing/AI Processing/Delivering → briefing renders with meetings, emails, actions
- [ ] **Meeting prep:** Click a meeting → prep page loads with agenda, wins, context → fields are editable → changes persist
- [ ] **Email triage:** Emails load → AI priority classification renders → high/medium/low tiers display correctly
- [ ] **Entity pages:** Account, project, person detail pages load with correct data and editorial layout
- [ ] **Actions:** Create, update, complete actions → changes persist across app restart
- [ ] **Search:** Cmd+K → search returns relevant entities → navigation works
- [ ] **MCP integration:** Settings → "Connect to Claude Desktop" → config written → Claude Desktop can query workspace
- [ ] **Auto-updater:** Settings → "Check for Updates" → updater check completes (verify against current release endpoint)
- [ ] **What's New modal:** Reset state, then verify modal shows current release notes on next launch. Run in the app's DevTools console (open with ⌘⌥I while the app is running — not the terminal, localStorage lives in the WebView):
  ```js
  localStorage.removeItem('dailyos_release_notes');
  localStorage.removeItem('dailyos_last_seen_version');
  ```
  Reload the app — the What's New modal should appear with the current version's notes.
- [ ] **Transcript processing:** Attach transcript → outcomes extracted → actions created

## 10. UI/UX Tests

- [ ] **Magazine layout:** All pages render in editorial shell with navigation island and folio bar
- [ ] **Typography:** Newsreader (body) and Montserrat (headings) load correctly — no system font fallback flash
- [ ] **Color system:** Material palette (Paper, Desk, Spice, Garden) renders correctly in both light and dark themes
- [ ] **Theme toggle:** Light ↔ Dark switches cleanly with no flash or layout shift
- [ ] **Empty states:** Pages with no data show personality-driven empty states (not blank)
- [ ] **Loading states:** Async operations show progress indicators (no silent waits)
- [ ] **Error states:** Network failures, auth failures, and missing data show user-friendly messages (no developer errors)
- [ ] **Navigation:** All sidebar links route correctly, deep links work (`/settings?tab=...`)
- [ ] **Responsive behavior:** Window resize from 1280px down to minimum — no overflow, no broken layouts
- [ ] **Keyboard navigation:** Tab order is logical, Cmd+K opens search, meeting prep keyboard shortcuts work
- [ ] **Tray icon:** System tray icon renders as template icon (adapts to light/dark menu bar)
- [ ] **ADR-0083 vocabulary audit:** grep user-facing strings for "entity", "intelligence", "signal", "enrichment", "prep", "connector" — translate per vocabulary rules in ADR-0083
- [ ] **Release notes vocabulary:** verify `release-notes.md` entry exists for this version and uses product vocabulary (no internal jargon)

## 11. CI Pipeline

- [ ] All checks pass on a clean branch (not just locally)
- [ ] Release workflow dry-run: verify `release.yml` steps match current build requirements
- [ ] **Validate workflow YAML** — `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml')); print('valid')"`. GitHub silently refuses to start jobs if the YAML is unparseable. Common trap: `---` inside a heredoc in a `run: |` block is parsed as a YAML document separator, not shell content. Use `printf` instead of heredocs containing `---`.
- [ ] **Verify tag triggers release workflow** — after pushing a tag, check `gh run list --workflow=release.yml --limit 3` within 30 seconds. The run must show `headBranch: "vX.Y.Z"` (not `main`). If missing, the YAML is likely broken (see above) or GitHub deduplicated the push event (create a new commit before re-tagging).
- [ ] Apple certificate and notarization secrets are current (not expired)
- [ ] `DAILYOS_GOOGLE_SECRET` repo secret is set
- [ ] `TAURI_SIGNING_PRIVATE_KEY` repo secret is set (for updater signatures)
- [ ] **Service layer boundary check** — `bash scripts/check_service_layer_boundary.sh` passes. Script uses `grep` (not `rg`) for CI compatibility. The script skips `#[cfg(test)]` blocks — verify the cutoff works by checking the script uses `grep -n`, not `rg -n`. If adding new hotspot files to the script, use the actual path (e.g., `hygiene/mod.rs` not `hygiene.rs`).
- [ ] **No uncommitted files referenced by committed code** — `git stash && cargo check --manifest-path src-tauri/Cargo.toml --lib && git stash pop` to verify committed code compiles independently. Partial commits (e.g., calling `crate::foo` without committing `foo.rs`) pass locally but fail in CI.
- [ ] **Sidecar build script is intact** — `build-mcp.sh` must create a stub file BEFORE `cargo build` (Tauri's build.rs validates externalBin paths during any cargo build from that Cargo.toml, including the sidecar itself). Verify `touch src-tauri/build.rs` runs after sidecar creation in both `test.yml` and `release.yml` to force re-evaluation during the Tauri build step.

## 12. Git Hygiene

- [ ] All changes are on `main` (merged from `dev`)
- [ ] No unrelated changes in the release commit
- [ ] **No work-in-progress files from other branches leaking into the working tree** — run `git status` and verify every modified/untracked file is either staged for the release or intentionally excluded. Partial commits (staging some files but not their dependencies) cause CI compile failures that don't reproduce locally.
- [ ] Commit messages are descriptive — `Co-Authored-By` tags present where applicable
- [ ] Tag matches version: `git tag v{version}` (e.g., `v0.8.0`)
- [ ] `.gitignore` covers all build artifacts (`src-tauri/target/`, `src-tauri/binaries/`, `dist/`)

## 13. Post-Push Verification

- [ ] GitHub Actions release workflow completes green
- [ ] GitHub Release page has DMG, `.tar.gz`, `.tar.gz.sig`, and `latest.json`
- [ ] Download DMG from GitHub Release → install → app launches and functions
- [ ] Existing install receives update notification (test with previous version installed)
- [ ] `latest.json` signature validates against the public key in `tauri.conf.json`

---

## Quick Reference

```bash
# Full pre-release build + test sequence
pnpm install
pnpm build:mcp
pnpm test
pnpm tauri build --target aarch64-apple-darwin
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo audit --file src-tauri/Cargo.lock
pnpm audit
bash scripts/check_service_layer_boundary.sh
```

```bash
# Verify bundle contents
ls "src-tauri/target/aarch64-apple-darwin/release/bundle/macos/DailyOS.app/Contents/MacOS/"
# Should show: dailyos, dailyos-mcp

# Tag and push
git tag v{version}
git push origin main --tags
```
