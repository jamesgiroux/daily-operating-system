# Release Checklist

Pre-push checklist for DailyOS releases. Complete every section before tagging and pushing to `main`.

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
- [ ] `README.md` updated if the release changes installation steps, requirements, or core features
- [ ] `.docs/RELEASE-NOTES.md` updated with alpha/beta tester-facing summary (if applicable)
- [ ] Architecture Decision Records created for any new architectural decisions (`.docs/decisions/`)

## 3. Build Verification

- [ ] `pnpm install` — clean install succeeds with no warnings
- [ ] `pnpm build` — frontend builds without errors or TypeScript failures
- [ ] `pnpm build:mcp` — MCP sidecar binary builds and lands in `src-tauri/binaries/` (script creates a stub first to satisfy Tauri's build.rs, then overwrites with the real binary — if this step fails with "resource path doesn't exist", the stub creation is broken)
- [ ] `pnpm tauri build --target aarch64-apple-darwin` — full app bundle succeeds
- [ ] DMG opens and installs to `/Applications` cleanly
- [ ] Verify `DailyOS.app/Contents/MacOS/` contains both `dailyos` and `dailyos-mcp`
- [ ] App launches from `/Applications` (not from build directory)

## 4. Rust Backend

- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` — all tests pass
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` — zero warnings
- [ ] `cargo audit --file src-tauri/Cargo.lock` — no known vulnerabilities (or documented exceptions)
- [ ] No new `unwrap()` or `expect()` in IPC command handlers (use `Result` propagation)
- [ ] Database migrations are forward-compatible and idempotent

## 5. Frontend

- [ ] `pnpm test` — all Vitest tests pass
- [ ] `pnpm audit` — no high/critical vulnerabilities (or documented exceptions)
- [ ] No TypeScript `// @ts-ignore` or `any` casts added without justification
- [ ] No `console.log` left in production code (use structured logging)

## 6. Security Review

- [ ] No secrets committed — search for `client_secret`, `api_key`, `password`, `token` in diff
- [ ] No hardcoded credentials or API keys in source
- [ ] `option_env!` used for build-time secrets (e.g., `DAILYOS_GOOGLE_SECRET`)
- [ ] CSP in `tauri.conf.json` unchanged (or changes are intentional and documented)
- [ ] IPC commands validate all input parameters (no path traversal, no injection)
- [ ] OAuth flow uses PKCE with S256 challenge
- [ ] Keychain storage for tokens — no plaintext token files
- [ ] `reveal_in_finder` and `copy_to_inbox` path validation intact

## 7. Performance Audit

- [ ] App cold launch to usable dashboard: under 3 seconds
- [ ] Hot read commands (status, focus): p95 under 100ms
- [ ] Dashboard load: p95 under 300ms
- [ ] No DB lock held across AI calls, network calls, or filesystem scans (split-lock pattern)
- [ ] AI subprocess runs with `nice -n 10` (yields to interactive work)
- [ ] Background tasks open own SQLite connections (not competing for shared Mutex)
- [ ] No regressions to binary size (compare against previous release)

## 8. Logic Tests (Does It Do What It Should)

- [ ] **Onboarding:** Fresh install → onboarding wizard completes → Google OAuth connects → first briefing generates
- [ ] **Daily briefing:** Click refresh → workflow progresses through Preparing/AI Processing/Delivering → briefing renders with meetings, emails, actions
- [ ] **Meeting prep:** Click a meeting → prep page loads with agenda, wins, context → fields are editable → changes persist
- [ ] **Email triage:** Emails load → AI priority classification renders → high/medium/low tiers display correctly
- [ ] **Entity pages:** Account, project, person detail pages load with correct data and editorial layout
- [ ] **Actions:** Create, update, complete actions → changes persist across app restart
- [ ] **Search:** Cmd+K → search returns relevant entities → navigation works
- [ ] **MCP integration:** Settings → "Connect to Claude Desktop" → config written → Claude Desktop can query workspace
- [ ] **Auto-updater:** Settings → "Check for Updates" → updater check completes (verify against current release endpoint)
- [ ] **Transcript processing:** Attach transcript → outcomes extracted → actions created

## 9. UI/UX Tests

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

## 10. CI Pipeline

- [ ] All checks pass on a clean branch (not just locally)
- [ ] Release workflow dry-run: verify `release.yml` steps match current build requirements
- [ ] Apple certificate and notarization secrets are current (not expired)
- [ ] `DAILYOS_GOOGLE_SECRET` repo secret is set
- [ ] `TAURI_SIGNING_PRIVATE_KEY` repo secret is set (for updater signatures)
- [ ] **No uncommitted files referenced by committed code** — `git stash && cargo check --manifest-path src-tauri/Cargo.toml --lib && git stash pop` to verify committed code compiles independently. Partial commits (e.g., calling `crate::foo` without committing `foo.rs`) pass locally but fail in CI.
- [ ] **Sidecar build script is intact** — `build-mcp.sh` must create a stub file BEFORE `cargo build` (Tauri's build.rs validates externalBin paths during any cargo build from that Cargo.toml, including the sidecar itself). Verify `touch src-tauri/build.rs` runs after sidecar creation in both `test.yml` and `release.yml` to force re-evaluation during the Tauri build step.

## 11. Git Hygiene

- [ ] All changes are on `main` (merged from `dev`)
- [ ] No unrelated changes in the release commit
- [ ] **No work-in-progress files from other branches leaking into the working tree** — run `git status` and verify every modified/untracked file is either staged for the release or intentionally excluded. Partial commits (staging some files but not their dependencies) cause CI compile failures that don't reproduce locally.
- [ ] Commit messages are descriptive — `Co-Authored-By` tags present where applicable
- [ ] Tag matches version: `git tag v{version}` (e.g., `v0.8.0`)
- [ ] `.gitignore` covers all build artifacts (`src-tauri/target/`, `src-tauri/binaries/`, `dist/`)

## 12. Post-Push Verification

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
```

```bash
# Verify bundle contents
ls "src-tauri/target/aarch64-apple-darwin/release/bundle/macos/DailyOS.app/Contents/MacOS/"
# Should show: dailyos, dailyos-mcp

# Tag and push
git tag v{version}
git push origin main --tags
```
