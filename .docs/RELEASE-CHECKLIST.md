# Release Checklist

Pre-release checklist for DailyOS. Complete every section before merging the release PR to `main`.

**Release cadence:** Weekly train, Tuesday late morning ET. See `RELEASE-POLICY.md` for the full schedule, hotfix criteria, and operating rules.

**Release flow:** `dev` → PR to `main` → checklist passes → merge PR → tag → CI builds → publish.

---

## 0. Train Readiness

- [ ] All issues assigned to this train are Done in Linear
- [ ] Monday noon feature cutoff observed — no new work after cutoff
- [ ] Release notes draft reviewed — changes grouped by user value area
- [ ] No open hotfix-worthy issues from the previous train

---

## 1. Create Release PR

Create a PR from `dev` → `main` using this format:

```bash
gh pr create --base main --head dev \
  --title "v1.2.0 — Actions & Success Plans: Closing the Loop" \
  --body "$(cat <<'EOF'
## v1.2.0 — Actions & Success Plans: Closing the Loop

### Issues included
Closes DOS-55, DOS-49, DOS-12, DOS-54, DOS-14, DOS-13, DOS-17, DOS-18, DOS-15, DOS-16, DOS-50, DOS-51, DOS-56, DOS-52, DOS-53, DOS-65

### Summary
- [bullet summary of changes by theme]

### Test plan
- [ ] Visual QA completed
- [ ] Codex review: 6/6 findings fixed
- [ ] `cargo clippy && cargo test && pnpm tsc --noEmit` clean
- [ ] Mock data covers all new features

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

**Why PR-based:**
- Single audit trail for what shipped in each version
- `Closes DOS-XX` auto-moves Linear issues to Done on merge
- Diff is reviewable, reversible, and linked from the GitHub Release page
- Linear shows the PR as linked activity on every referenced issue

---

## 2. Version Bump

- [ ] Bump version in `src-tauri/tauri.conf.json`
- [ ] Bump version in `src-tauri/Cargo.toml` (triggers `Cargo.lock` update)
- [ ] Bump version in `package.json`
- [ ] All three versions match

## 3. Changelog & Documentation

- [ ] `CHANGELOG.md` entry added for the new version with today's date
- [ ] Entry follows Keep a Changelog format (sections: Added, Changed, Fixed, Removed, Security as needed)
- [ ] Every user-facing change has a line item — no silent changes
- [ ] **`release-notes.md` entry added** — user-facing, product marketing language. This is what appears in the What's New modal. Lead with the story of the release, not a list of issues. No internal jargon, no issue numbers, no "entity intelligence" or "enrichment". Write like you're telling a customer what got better. See existing entries for format.
- [ ] `README.md` updated if the release changes installation steps, requirements, or core features
- [ ] Architecture Decision Records created for any new architectural decisions (`.docs/decisions/`)

## 4. Dependency Updates

Run these BEFORE build verification. CI runs `pnpm audit --audit-level high` and will fail on unpatched vulnerabilities.

- [ ] `pnpm update` — update all packages within semver ranges
- [ ] `pnpm audit --audit-level high` — zero high/critical vulnerabilities
- [ ] `cargo update --manifest-path src-tauri/Cargo.toml` — update Rust deps within semver ranges
- [ ] `cargo audit --file src-tauri/Cargo.lock` — zero known vulnerabilities (or documented exceptions)
- [ ] Commit lockfile updates before build verification

## 5. Build Verification

- [ ] `pnpm install` — clean install succeeds with no warnings
- [ ] `pnpm build` — frontend builds without errors or TypeScript failures
- [ ] `pnpm build:mcp` — MCP sidecar binary builds and lands in `src-tauri/binaries/`
- [ ] `pnpm tauri build --target aarch64-apple-darwin` — full app bundle succeeds
- [ ] DMG opens and installs to `/Applications` cleanly
- [ ] Verify `DailyOS.app/Contents/MacOS/` contains both `dailyos` and `dailyos-mcp`
- [ ] App launches from `/Applications` (not from build directory)

## 6. Rust Backend

- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` — all tests pass
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` — zero warnings
- [ ] `cargo audit --file src-tauri/Cargo.lock` — no known vulnerabilities
- [ ] No new `unwrap()` or `expect()` in IPC command handlers (use `Result` propagation)
- [ ] Database migrations are forward-compatible and idempotent

## 7. Frontend

- [ ] `pnpm test` — all Vitest tests pass
- [ ] `pnpm tsc --noEmit` — zero TypeScript errors
- [ ] `pnpm audit` — no high/critical vulnerabilities
- [ ] No TypeScript `// @ts-ignore` or `any` casts added without justification
- [ ] No `console.log` left in production code

## 8. Security Review

- [ ] No secrets committed — search for `client_secret`, `api_key`, `password`, `token` in diff
- [ ] No hardcoded credentials or API keys in source
- [ ] `option_env!` used for build-time secrets (e.g., `DAILYOS_GOOGLE_SECRET`)
- [ ] CSP in `tauri.conf.json` unchanged (or changes are intentional and documented)
- [ ] IPC commands validate all input parameters
- [ ] OAuth flow uses PKCE with S256 challenge
- [ ] Keychain storage for tokens — no plaintext token files

### Hard gate: content sweep against the local blocklist (`.claude/pii-blocklist.txt`)

This is non-negotiable. Release does not ship if any term from the blocklist appears in any of the surfaces below. The pre-commit hook (`.claude/hooks/pre-commit-gate.sh`) catches them at commit time, but the gate is only reliable if the blocklist is current and the hook was not bypassed.

- [ ] Blocklist is current — all customer names, account domains, stakeholder names, and other identifiers from the production workspace have been added since the last release
- [ ] Sweep all commits ahead of the release base (content + messages):
      ```bash
      TERMS=$(grep -v '^#' .claude/pii-blocklist.txt | grep -v '^$' | paste -sd '|' -)
      git log -p origin/<base>..HEAD | grep -iE "$TERMS" | grep -v "^+++" | head -50
      git log --format='%H %s%n%b' origin/<base>..HEAD | grep -iE "$TERMS" | head -20
      ```
- [ ] Sweep working tree:
      ```bash
      git grep -iE "$TERMS"
      ```
- [ ] Sweep `CHANGELOG.md`, `release-notes.md`, all `.docs/decisions/*.md`, all `.docs/plans/*.md`, all test fixtures, all mock data, all example payloads
- [ ] Sweep filenames (some leaks happen in paths, not content): `git diff --name-only origin/<base>..HEAD | grep -iE "$TERMS"`
- [ ] Sweep branch names: `git branch | grep -iE "$TERMS"`
- [ ] Any hits → halt release. Scrub via `git filter-repo --replace-text` and `--replace-message`. Force-push the rewritten branch. Re-run this gate.
- [ ] Commit messages produced by the scrub (or any release commit) do not reference the scrub itself, the blocklist, the act of removing data, or the categories of removed content — that information is itself a leak that tells observers what to look for in the older history

## 9. Performance Audit

- [ ] App cold launch to usable dashboard: under 3 seconds
- [ ] Hot read commands (status, focus): p95 under 100ms
- [ ] Dashboard load: p95 under 300ms
- [ ] No DB lock held across AI calls, network calls, or filesystem scans
- [ ] No regressions to binary size (compare against previous release)

## 10. Logic Tests

- [ ] **Onboarding:** Fresh install → wizard completes → Google OAuth connects → first briefing generates
- [ ] **Daily briefing:** Refresh → workflow progresses → briefing renders with meetings, emails, actions
- [ ] **Meeting prep:** Click meeting → prep loads → fields editable → changes persist
- [ ] **Email triage:** Emails load → priority classification renders → tiers display correctly
- [ ] **Entity pages:** Account, project, person detail pages load with correct data
- [ ] **Actions:** Create, update, complete actions → changes persist across app restart
- [ ] **Search:** Cmd+K → search returns relevant entities → navigation works
- [ ] **MCP integration:** Settings → Connect to Claude Desktop → config written → Claude Desktop can query
- [ ] **Auto-updater:** Settings → Check for Updates → updater check completes
- [ ] **What's New modal:** Reset localStorage, reload, verify modal shows current release notes
- [ ] **Transcript processing:** Attach transcript → outcomes extracted → actions created

## 11. v1.2.0 Feature-Specific Tests

- [ ] **Status vocabulary (DOS-55):** Tabs show "Suggested"/"Active"/"Completed", priorities show "Urgent"/"High"/"Medium"/"Low"
- [ ] **Manual capture (DOS-54):** Cmd+K → "Add action" → creates action with source_type=user_manual
- [ ] **Recommended actions (DOS-13):** Account detail shows Track/Dismiss cards
- [ ] **Decision badges (DOS-17):** Flagged actions show "Decision needed" badge
- [ ] **Linear push (DOS-52):** Hover-reveal push button, persistent badge on pushed actions
- [ ] **Objective evidence (DOS-14):** Objectives show "N mentions in calls"
- [ ] **Value delivered (DOS-12):** User-confirmed items survive re-enrichment
- [ ] **Aging awareness (DOS-53):** Briefing shows "N items aging" when applicable
- [ ] **Node installer (DOS-65):** Onboarding shows single Install button for both Node-missing and Node-present

## 12. UI/UX Tests

- [ ] **Magazine layout:** All pages render in editorial shell with navigation island and folio bar
- [ ] **Typography:** Newsreader and DM Sans load correctly — no system font fallback flash
- [ ] **Color system:** Material palette renders correctly in both light and dark themes
- [ ] **Theme toggle:** Light ↔ Dark switches cleanly with no flash or layout shift
- [ ] **Empty states:** Pages with no data show personality-driven empty states
- [ ] **Loading states:** Async operations show progress indicators
- [ ] **Error states:** Failures show user-friendly messages
- [ ] **Navigation:** All links route correctly, deep links work
- [ ] **Responsive behavior:** Window resize — no overflow, no broken layouts
- [ ] **Keyboard navigation:** Tab order logical, Cmd+K opens search
- [ ] **ADR-0083 vocabulary audit:** No "entity", "intelligence", "signal", "enrichment" in user-facing strings

## 13. CI Pipeline

- [ ] All checks pass on a clean branch
- [ ] Release workflow dry-run: verify `release.yml` steps match current build requirements
- [ ] Apple certificate and notarization secrets are current
- [ ] `DAILYOS_GOOGLE_SECRET` repo secret is set
- [ ] `TAURI_SIGNING_PRIVATE_KEY` repo secret is set
- [ ] Service layer boundary check passes: `bash scripts/check_service_layer_boundary.sh`
- [ ] No uncommitted files referenced by committed code

## 14. Git Hygiene & Release PR

- [ ] Release PR from `dev` → `main` is open and reviewed
- [ ] PR body lists all `DOS-XX` issues with `Closes` prefix
- [ ] No unrelated changes in the release
- [ ] Commit messages are descriptive with `Co-Authored-By` tags where applicable
- [ ] `.gitignore` covers all build artifacts

## 15. Merge & Tag

- [ ] Merge the release PR (creates merge commit on `main`)
- [ ] Tag: `git tag v{version}` on the merge commit
- [ ] Push: `git push origin main --tags`
- [ ] Reconcile: `git checkout dev && git merge main` (keeps dev up to date)

## 16. Post-Merge Verification

- [ ] GitHub Actions release workflow completes green
- [ ] GitHub Release page has DMG, `.tar.gz`, `.tar.gz.sig`, and `latest.json`
- [ ] Download DMG from GitHub Release → install → app launches and functions
- [ ] Existing install receives update notification
- [ ] `latest.json` signature validates against the public key in `tauri.conf.json`
- [ ] Linear issues all show as Done with PR linked

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
# Create release PR
gh pr create --base main --head dev \
  --title "vX.Y.Z — Release Title" \
  --body "Closes DOS-XX, DOS-YY, ..."

# After PR merge
git checkout main && git pull
git tag vX.Y.Z
git push origin main --tags
git checkout dev && git merge main && git push origin dev
```
