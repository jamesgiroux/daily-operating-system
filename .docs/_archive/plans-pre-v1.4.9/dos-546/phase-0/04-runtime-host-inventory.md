---
status: spec:ready
date: 2026-05-10
related_adrs: [0102, 0111, 0128, 0129]
open_questions: see ./INDEX.md (routed to W2-A and W1-B L0 Prep)
---

# DOS-546 Phase 0 — Runtime-Host Inventory + Architecture Recommendation

## Context

This artifact inventories the runtime-host responsibilities currently carried by the
DailyOS Tauri application and uses that inventory to choose a viable host shape for
the WordPress Studio spike.

The requested primary CSO review file,
`.docs/reviews/dos-546-l0-cso-2026-05-10.md`, is not present in this checkout.
The requested ADR-0129 file,
`.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md`,
is also not present in this checkout.
This document therefore treats the prompt's summary of those sources as normative
for DOS-546:

- WordPress Studio is being evaluated as the primary user-facing Studio surface.
- DailyOS continues hosting MCP regardless of the WordPress outcome.
- Under the corrected architecture, Tauri's MCP-server role for non-WP surfaces is
  more load-bearing, not less.
- Any proposal to drop Tauri must answer where the headless MCP surface lives.

Available repository sources used for the inventory:

- ADR-0128: MCP is a co-equal product surface, not an export pipe.
- ADR-0111: Tauri, MCP, workers, and future surfaces invoke abilities through
  surface-specific bridges over one ability registry.
- ADR-0102: abilities are the runtime contract of the product.
- `src-tauri/tauri.conf.json`: current Tauri product, bundle, updater, CSP, and
  external binary configuration.
- `src-tauri/Cargo.toml`: current Tauri, updater, process, tray, SQLCipher,
  RMCP, and sidecar binary dependencies.
- `src-tauri/src/lib.rs`: current application setup, worker startup, tray menu,
  lifecycle behavior, and IPC command registration.
- `src-tauri/src/mcp/main.rs`: current `dailyos-mcp` stdio server binary.
- `src-tauri/capabilities/default.json`: current Tauri capability/permission set.
- Representative frontend settings/recovery files for update, pairing, lock,
  and recovery UI behavior.

The architecture principle from ADR-0128 is the key constraint:
the substrate is the product, and heads are surfaces over it.
WordPress may become the primary user-facing head, but it should not become a
second runtime authority.

The architecture principle from ADR-0111 is the second key constraint:
surfaces construct their own invocation context, then invoke through the shared
ability registry.
The runtime host must therefore support:

- user actor invocations from a visible surface;
- agent actor invocations from MCP;
- system actor invocations from background workers;
- durable local storage and provenance-bearing claim reads/writes;
- confirmation and attestation flows where mutation needs a human boundary.

## Current Tauri Inventory

| # | Responsibility | What Tauri provides today | How DailyOS uses it today | Daemon-only replacement would need | What becomes harder | What becomes impossible without visible controls |
|---|---|---|---|---|---|---|
| 1 | App lifecycle | Tauri owns the packaged desktop process, creates the main webview window, initializes plugins, manages `AppState`, and starts the Rust runtime from `dailyos_lib::run()`. Window close is intercepted and hidden rather than quitting. Tray menu offers Open, Run Briefing Now, and Quit. | DailyOS launches as a native app, keeps background workers alive after the window closes, foregrounds from tray, runs startup sync, initializes DB service, starts scheduler/executor, and exposes recovery screens when DB/key startup checks fail. | A launch agent/service plus a process supervisor, foreground helper discovery, lifecycle IPC, shutdown semantics, config reload, worker initialization ordering, and a way to present recovery state. | Cross-platform launch-on-login, user-driven foregrounding, recovery UX, app relaunch after updates, and orderly worker shutdown all move from Tauri conventions into custom service-management code. | Any first-run, recovery, unlock, settings, or "WP is down, open the local escape hatch" workflow that requires showing a trusted local screen. |
| 2 | Single-instance guarantee | No explicit `tauri-plugin-single-instance` dependency or config was found. Tauri currently gives one packaged app entry point, but the code also builds a separate `dailyos-mcp` binary and has process-safe guards for some DB work. | DailyOS relies on the normal app process for the main runtime. Some startup work is idempotent or guarded in SQLite, for example claim cutover ownership markers. MCP is a separate stdio process opened read-only. | An OS-level mutex/lock file or service manager policy, explicit port/socket ownership, cross-process stale-lock recovery, and DB writer serialization. | Preventing two runtime owners from starting concurrently becomes a first-class platform problem. The daemon and helper UI could accidentally race over migrations, workers, or MCP pairing state. | User-visible conflict resolution when another process owns the runtime. Without a UI, the system can only fail silently, log, or require manual terminal cleanup. |
| 3 | Keychain access | `LocalKeychain` stores the SQLCipher database key under service `com.dailyos.desktop.db` and account `sqlcipher-key`. The current implementation shells through macOS `security`; no Windows Credential Manager or Linux secret-service provider is present in this checkout. Key rotation and recovery state are guarded with process-wide locks and staged keychain entries. | DailyOS encrypts the SQLite database with SQLCipher, creates a key only for fresh/plaintext DBs, refuses to generate a replacement key for an existing encrypted DB, and shows `EncryptionRecovery` when the key is missing. Glean and other connectors also use token stores backed by OS keychain patterns. | A cross-platform secret provider abstraction with macOS Keychain, Windows Credential Manager, and Linux Secret Service backends; migration from the current service/account; helper UI for lost-key recovery; and tests for key rotation across daemon restarts. | Code-signing and entitlements affect keychain identity. A headless daemon may run under a different bundle identity, service label, or user session than the current Tauri app, making keychain continuity non-trivial. | Lost-key recovery, rekey confirmation, and "start fresh" decisions. A daemon can detect a missing key, but it cannot safely ask the user to approve destructive recovery without a trusted visible surface. |
| 4 | MCP server hosting | The Tauri bundle includes `externalBin: ["binaries/dailyos-mcp"]`. `Cargo.toml` defines `dailyos-mcp` behind the `mcp` feature. The binary hosts an RMCP stdio server, opens the SQLCipher DB read-only, initializes embeddings, serves static tools, and bridges ability-backed MCP tools through `McpAbilityBridge`. | DailyOS configures Claude Desktop via Tauri settings commands, exposes DailyOS as a headless MCP surface, and keeps MCP aligned with ADR-0128's "co-equal head" framing. The MCP server is intentionally a product surface for non-WP hosts such as Claude Desktop and future CLI/agent flows. | A stable packaged MCP binary or long-running daemon MCP endpoint; installation/configuration into host clients; versioning; DB read access; ability registry loading; provenance cache; and user-approved pairing/config changes. | If Tauri is removed, MCP becomes the main non-WP runtime contract. Packaging, upgrading, configuring host apps, and communicating MCP health must be rebuilt outside the current app/settings flow. | Interactive MCP pairing approval, host configuration repair, and user-visible diagnostics for "Claude cannot see DailyOS." Under ADR-0128, dropping the visible host without replacing those controls strands the co-equal headless surface. |
| 5 | File system permissions | Tauri v2 capability file grants the main window `core:default`, `shell:allow-open`, `notification:default`, `dialog:default`, `updater:default`, and `process:allow-restart`. `tauri.conf.json` restricts CSP and bundles resources/plugins. There is no broad Tauri FS plugin permission in `default.json`; file access is mostly Rust backend code and user-selected dialogs. | DailyOS reads/writes the workspace, `~/.dailyos`, bundled plugin resources, MCP config files, backups, exports, inbox files, and imported user attachments through Rust commands and services. The frontend reaches these through IPC rather than direct browser file access. | A daemon permission model, helper UI file pickers, a policy for workspace-root access, safe export/import flows, backup file selection, and equivalent protections against arbitrary frontend file access. | Tauri's capability model currently constrains the webview. A daemon plus WordPress shifts trust to HTTP/plugin calls and must rebuild per-operation authorization, CSRF/session boundaries, and path-scoping. | User-mediated file selection via native dialogs, if no helper UI exists. WordPress alone cannot safely grant local file system access to a native local database/workspace without a native broker. |
| 6 | System tray / menubar presence | Tauri is built with `tray-icon`. `lib.rs` creates a tray icon and menu with Open DailyOS, Run Briefing Now, and Quit. Closing the window hides it and leaves the tray process alive. | The tray is the always-available control point for returning to the app, forcing a briefing run, and quitting the runtime. It also signals that background processing is intentionally running. | A menu bar helper, status item, or platform-native companion app. A pure daemon has no comparable control affordance. | User trust and controllability. A silent daemon running pollers, embeddings, Google/Linear/Drive sync, and MCP can feel opaque without a visible status/control point. | Run-now, quit, open diagnostics, and "is DailyOS running?" checks for non-technical users. |
| 7 | Crash recovery | Release profile uses `panic = "unwind"`. `task_supervisor::spawn_supervised` restarts named background tasks after normal exit or panic with capped backoff. Startup drains persisted pending health recomputes and claim invalidation/repair jobs. DB recovery screens handle migration/integrity failures. No full-process crash reporter upload system was found. | Calendar, email, capture, intelligence, meeting prep, embeddings, hygiene, Quill, Granola, enrichment, Linear, and Drive workers are supervised in-process. Persisted job markers survive app restarts so some work can resume after process death. | A daemon supervisor, per-worker restart policy, durable queue recovery, logs/crash capture, user notification on repeated failures, and an independent recovery UI. | Full-process crash recovery becomes an OS service problem. Capturing diagnostics from a background daemon and asking the user for remediation is harder than showing a Tauri recovery screen. | User-approved recovery actions such as restore backup, export DB before reset, start fresh, or acknowledge degraded mode. |
| 8 | Update flow | `tauri.conf.json` enables updater artifacts and configures the Tauri updater endpoint/public key. The Settings SystemStatus UI checks for updates via `@tauri-apps/plugin-updater`, downloads/installs, then relaunches via `@tauri-apps/plugin-process`. Capability permissions include `updater:default` and `process:allow-restart`. | DailyOS exposes update checks and install/restart in Settings. The packaged Tauri app controls app version, bundle artifacts, and relaunch behavior. | A signed daemon updater, helper UI updater, service restart flow, rollback behavior, MCP binary update coordination, and WordPress plugin compatibility/version negotiation. | Updating a daemon safely while MCP hosts and background workers may be active is more complex. The helper UI must know whether it is updating itself, the daemon, the WP plugin, or the MCP binary. | A user-friendly "install and restart" flow. A daemon can self-update, but cannot explain update state or request restart consent without some visible helper. |
| 9 | Code-signing | `tauri.conf.json` bundles all targets, sets macOS signing identity to `Developer ID Application`, and uses Tauri's packaging pipeline. The product identifier is `com.dailyos.desktop`; minimum macOS is 13.0. The Windows/macOS/Linux bundle targets are delegated to Tauri's bundler. | DailyOS distributes as a signed desktop app with a known bundle identifier, icon set, updater artifacts, and bundled external binaries/resources. | Separate signing/notarization for daemon, helper app, MCP binary, installers, update manifests, launch agents, and possibly WordPress/native bridge components. Windows SmartScreen reputation and Linux service packaging would need new pipelines. | Headless daemons still need signing, entitlements, install/uninstall, auto-start registration, and user trust. Tauri makes that one app-shaped artifact; a daemon/helper split creates multiple signed identities and update channels. | User-facing trust prompts that explain why a background service is installed. Without helper UI, OS security prompts and failures are harder to contextualize. |
| 10 | User-visible controls | Current UI includes Settings, connector setup, Claude Desktop MCP configuration, cowork plugin export, update controls, Google/Glean/Drive/Linear/Quill/Granola/Clay settings, diagnostics, privacy/data controls, lock/unlock, encryption recovery, database recovery, iCloud warning, post-meeting capture prompts, and claim reveal/correction/feedback controls. | DailyOS uses local UI for all trust, setup, and recovery interactions: connector OAuth, MCP host configuration, sensitive claim reveal, intelligence feedback, correction submission, destructive data actions, database backup/restore, and update install/relaunch. | A helper UI that can be launched from WP, tray, installer, or OS search; deep links from WordPress to helper; local auth/session handoff; and complete parity for setup, recovery, approval, and diagnostics. | Separating "primary Studio" from "trusted local controls" creates product and engineering complexity. WordPress cannot safely replace every local OS-mediated approval or recovery screen. | Interactive pairing approval, sensitive claim reveal, destructive data deletion confirmation, lost-key recovery, backup restore, update restart consent, and "local runtime unhealthy" troubleshooting. |
| 11 | IPC between frontend and Rust backend | Tauri's IPC command surface is large and explicit. `invoke_handler` registers ability invocation, operation invocation, settings, connectors, data export/privacy, database recovery, background triggers, search, app lock, audit log, and many domain commands. Frontend uses `@tauri-apps/api/core` and listens to Tauri events via `@tauri-apps/api/event`. | React calls Rust commands directly. Rust emits events such as lock state, calendar updates, transcript processed, intelligence updates, and sync status. ADR-0111's `invoke_ability` path is present for registry-backed ability calls. | A local HTTP/WebSocket/gRPC bridge, authentication, CSRF protection, origin allowlist for WP, request signing/pairing, event streaming, schema/version negotiation, and equivalent command authorization. | Tauri IPC implicitly binds a trusted local webview to the Rust backend. WordPress is remote/web-hosted or plugin-hosted, so the IPC boundary becomes a network/API security boundary. | Trusted local commands from an unpaired UI. Without pairing and user-visible approval, WP must not be allowed to mutate or reveal local data. |
| 12 | SQLite/storage access for claim substrate | `rusqlite` uses bundled SQLCipher. `AppState` owns sync and async DB services, read/write separation, migration startup, hardening, backups, claim cutover hooks, claim invalidation jobs, feedback tables, audit log, and recovery status. MCP opens a read-only `ActionDb`. | DailyOS treats local SQLite as the source of truth for the claim substrate, entities, meetings, actions, feedback, embeddings, sync state, audit records, and recovery markers. Abilities and bridges read/write through service context and DB services. | A single authoritative storage owner, migration lock, read-only MCP access, write serialization, backup/restore, SQLCipher key continuity, file permission hardening, schema forward-compat handling, and repair workers. | If WP, daemon, helper UI, and MCP all open the DB independently, migration and write ordering become a serious risk. A daemon can solve this by owning all writes, but then every surface must use its API. | User-visible database recovery and backup decisions. The DB can self-protect, but users need a local UI to choose restore/export/start-fresh paths. |

### Inventory Readout

The current Tauri runtime is not just an editorial shell.
It is the native process host for:

- product UI;
- local OS trust boundaries;
- settings and diagnostics;
- background ingestion and enrichment;
- storage initialization and recovery;
- update/relaunch;
- tray/menu lifecycle controls;
- MCP installation/configuration support;
- IPC and eventing between React and Rust;
- ability invocation as `User`;
- supervised `System` background workers.

The separate `dailyos-mcp` binary already proves that the MCP head can run as a
separate process, but it is not independent of the Tauri distribution model.
It is bundled by Tauri, configured by Tauri UI, reads the same SQLCipher database,
and depends on the same ability registry and local keychain behavior.

That means the real DOS-546 question is not "can WordPress replace Tauri UI?"
It can replace much of the editorial Studio surface if pairing and data contracts
are solved.
The harder question is "which runtime process owns local trust, MCP, storage,
background workers, updates, and recovery?"

## Option A — Tauri Supervisor + WP Primary Surface

### Shape

Tauri continues to run as the native local runtime supervisor.
WordPress Studio becomes the primary user-facing editorial surface.
Tauri's visible UI is reduced to the controls that require local OS trust or
must remain available when WordPress is unavailable.

Tauri owns:

- local app lifecycle and tray/menu presence;
- SQLCipher database and keychain continuity;
- background workers and durable repair jobs;
- MCP server bundling/configuration for non-WP surfaces;
- local pairing approval for WordPress Studio;
- settings, diagnostics, database recovery, encryption recovery, updates, and
  escape-hatch controls;
- ability invocation bridge for local user-confirmed flows.

WordPress owns:

- primary Studio editing and review experience;
- primary visual consumption of DailyOS intelligence;
- WP-native publishing/editorial workflows;
- non-sensitive day-to-day user interaction where calls can be mediated through
  a paired local runtime API.

### Pros

| Dimension | Assessment |
|---|---|
| ADR-0128 alignment | Strong. MCP remains a co-equal non-WP product surface, hosted by the native runtime that already bundles/configures `dailyos-mcp`. |
| ADR-0111 alignment | Strong. WP becomes another surface client; Tauri/MCP/worker bridges remain examples of the same surface-independent invocation model. |
| Runtime continuity | Strong. Existing DB, keychain, updater, tray, worker, crash-restart, and recovery behavior remain intact. |
| Spike value | High. The spike can focus on whether WP Studio is a viable primary editorial surface instead of rebuilding OS runtime plumbing first. |
| User safety | Strong. Pairing, sensitive reveal, destructive actions, recovery, and diagnostics stay in trusted local UI. |
| Delivery risk | Moderate. Requires a local API/pairing contract and a pared-down Tauri shell, but avoids a full daemon rewrite. |

### Cons

| Dimension | Cost |
|---|---|
| Two visible products | Users may see both WordPress Studio and a small DailyOS native helper. The product needs clear language: WP is the Studio; DailyOS native is the local runtime/control center. |
| Pairing complexity | WP must pair with the local Tauri runtime and respect capabilities, origin checks, and consent boundaries. |
| UI scope discipline | The Tauri editorial app must be intentionally reduced or hidden to avoid competing with WP as the primary surface. |
| Testing matrix | Need tests for WP-up/Tauri-up, WP-down/Tauri-up, Tauri-down/WP-up, MCP while WP is down, updates during paired sessions, and DB recovery paths. |

### Risks

| Risk | Mitigation |
|---|---|
| Tauri accidentally remains the real primary UI | Define the retained Tauri UI as setup, trust, diagnostics, and escape hatch only. Move editorial workflows to WP during the spike. |
| WP gets direct local-data authority | Force WP through a paired local runtime API that maps to ability/operation contracts; do not expose raw DB or broad file access. |
| MCP is treated as incidental | Make MCP health and configuration first-class in the Tauri control center because ADR-0128 makes it a co-equal surface. |
| Pairing becomes ad hoc | Model pairing as an explicit trust boundary with user approval, revocation, diagnostics, and scoped capabilities. |

### Inventory Fit

Option A preserves the hardest-to-replace inventory items:

- lifecycle;
- keychain;
- MCP hosting/configuration;
- tray/menu controls;
- crash-supervised workers;
- update/relaunch;
- code-signing/package identity;
- user-visible approval and recovery screens;
- IPC/event bridge, translated into a pairing API for WP;
- SQLCipher/claim-substrate storage ownership.

It also respects the CSO reminder in the prompt:
Tauri's MCP-server role is more load-bearing under the corrected architecture.
If WordPress becomes the main Studio surface, Tauri becomes less important as an
editorial UI, but more important as the native runtime host for non-WP surfaces.

## Option B — Pure Daemon + Helper UI

### Shape

DailyOS is split into:

- a background daemon with no Tauri app shell;
- a small helper UI for setup/pairing/recovery;
- WordPress Studio as the only ongoing user-visible product surface;
- a daemon-hosted MCP surface or separately installed MCP server binary.

The daemon owns runtime, storage, workers, local API, and MCP.
The helper UI is launched only for setup, pairing, settings, diagnostics, update,
and recovery flows.

### Pros

| Dimension | Assessment |
|---|---|
| Product clarity | Strong if executed perfectly: users live in WP Studio day to day. |
| Runtime separation | Clean conceptual split between background substrate host and Studio surface. |
| Future surface model | A daemon API could serve WP, MCP, CLI, and future native/mobile surfaces uniformly. |
| Reduced Tauri editorial baggage | Avoids carrying a full desktop editorial app if WP proves primary. |

### Cons

| Dimension | Cost |
|---|---|
| Rebuild scope | Very high. It replaces the app host, updater, lifecycle, tray, recovery, pairing, signing, installer, and large pieces of user-visible control. |
| Code-signing complexity | High. Daemon, helper, MCP binary, launch agent/service, updater, and installer each need signing/notarization/reputation handling. |
| User trust | Worse by default. A silent daemon running local data processing and MCP needs a clear installed control point. |
| Keychain continuity | Risky. The daemon/helper identity must preserve access to `com.dailyos.desktop.db` keys and connector tokens. |
| Update complexity | High. Updating a daemon and helper safely while WP and MCP may be using it is harder than Tauri's app-shaped update flow. |
| Recovery complexity | High. DB recovery and lost-key recovery still need a UI, so the "no Tauri app" simplification is not total. |

### Risks

| Risk | Mitigation |
|---|---|
| MCP host is orphaned | The daemon must explicitly host MCP or install/configure a packaged MCP binary. This is not optional under ADR-0128. |
| Helper UI grows into a second app anyway | Define helper scope tightly, but accept that settings/recovery/diagnostics/pairing are substantial. |
| Multi-process DB corruption/migration races | Make the daemon the only writer and force helper/WP/MCP through daemon APIs, or implement robust DB locks and read-only rules. |
| Hard-to-debug field failures | Build crash reporting, logs, health checks, and support bundle export before broad testing. |
| Platform scope explosion | Start with macOS-only daemon/service if this option is pursued; cross-platform daemon parity is not Phase 0 or Phase 1 scope. |

### Inventory Fit

Option B can be architecturally clean, but only after it reimplements almost
everything Tauri currently provides.

The most difficult inventory items to replace are:

- app lifecycle and launch/quit/relaunch semantics;
- code signing/notarization and installer trust;
- keychain identity continuity;
- update/restart flow;
- tray/menu or equivalent control presence;
- recovery UI;
- local file dialogs;
- pairing approval;
- MCP configuration diagnostics;
- single-writer claim-substrate ownership.

The pure daemon option is therefore not a shortcut to WordPress primary.
It is a runtime-platform project.
It may be a later architecture if DailyOS needs a service-first runtime, but it
is too much substrate churn for a viability spike whose core question is whether
WordPress Studio can be the primary surface.

## Option C — Tauri Remains Primary

### Shape

Tauri remains the primary user-facing Studio/editorial UI.
WordPress becomes an additional surface, export destination, publishing surface,
or embedded/editorial adjunct.
The DOS-546 spike no longer tests "WP Studio as primary" in a serious way; it
tests multi-surface coexistence.

Tauri continues to own:

- runtime;
- storage;
- background workers;
- MCP;
- settings;
- primary editorial views;
- update/recovery;
- local user controls.

WordPress owns:

- a secondary Studio surface;
- publishing or content-management workflows;
- perhaps a subset of review/edit loops.

### Pros

| Dimension | Assessment |
|---|---|
| Lowest runtime risk | Strong. It preserves the current architecture and avoids daemon or pairing disruption for the primary surface. |
| Existing UX continuity | Strong. No need to demote or redesign Tauri controls immediately. |
| MCP continuity | Strong. The existing MCP packaging/configuration story remains unchanged. |
| Delivery speed for narrow WP integration | Good if WP is only an adjunct. |

### Cons

| Dimension | Cost |
|---|---|
| Spike intent | Weak. It effectively abandons the premise that WP Studio could be primary. |
| Product ambiguity | High. Users get two editorial surfaces instead of a clear primary Studio. |
| Duplicated UI | High. Editorial features must be implemented, tested, and explained twice unless WP is very narrow. |
| Strategic learning | Low. It does not answer whether WP can carry the Studio product. |

### Risks

| Risk | Mitigation |
|---|---|
| WP becomes ornamental | Set hard success criteria for WP-owned workflows if keeping this option alive. |
| Surface divergence | Use ADR-0111 strictly: both Tauri and WP must call the same ability/runtime contracts. |
| Product story gets muddled | Position WordPress as publish/review only, not another DailyOS home, if choosing this path. |
| Runtime complacency | Even if Tauri remains primary, document MCP as co-equal so non-WP headless access does not regress. |

### Inventory Fit

Option C fits the existing inventory best because it changes the least.
It preserves every Tauri responsibility in the table.

Its weakness is not technical feasibility.
Its weakness is that it does not satisfy the corrected DOS-546 product question.
If the goal is WordPress Studio surface viability, then keeping Tauri primary
turns the spike into a coexistence exercise.

## Recommendation

Choose **Option A: Tauri supervisor + WP primary surface**.

The reasoning is straightforward:

1. The current Tauri runtime owns too many local-host responsibilities to drop
   during a WordPress Studio viability spike.
2. ADR-0128 requires MCP to remain a co-equal product surface regardless of the
   WordPress outcome.
3. ADR-0111 already gives the right shape: WordPress should be a surface client,
   not a second runtime authority.
4. The inventory shows that the hardest responsibilities are not editorial UI
   rendering; they are lifecycle, keychain, MCP hosting/configuration, storage
   ownership, background workers, code signing, update/relaunch, recovery, and
   user-approved trust boundaries.
5. Option A preserves those responsibilities while allowing WordPress to carry
   the actual Studio experience being evaluated.

The recommendation is not "keep Tauri because Tauri UI is the product."
It is the opposite:
reduce Tauri's editorial surface area so WordPress can become primary, but keep
Tauri as the native supervisor because the local runtime host is still required.

Tauri's MCP-host role for non-WP surfaces is **more load-bearing** under the
corrected architecture, not less.
When WP becomes the primary Studio surface, MCP becomes the other explicitly
supported non-WP head over the same substrate.
Dropping Tauri without an MCP-host replacement would violate ADR-0128 and would
leave Claude Desktop/CLI/agent surfaces without a clear home.

### Recommended Phase 1 Architecture

| Layer | Owner | Notes |
|---|---|---|
| WordPress Studio UI | WordPress plugin/theme surface | Primary day-to-day Studio surface. Calls local runtime through explicit paired API. |
| Native runtime supervisor | Tauri | Starts workers, owns DB/keychain, owns updater/recovery/diagnostics, packages/configures MCP. |
| Headless MCP surface | `dailyos-mcp`, bundled/configured by Tauri | Remains co-equal product head for Claude Desktop and future agent/CLI surfaces. |
| Ability/runtime contract | Rust ability registry/services | Shared by Tauri, MCP, workers, and WP-mediated calls. |
| Storage authority | Tauri/AppState/DB service | One local source of truth. Avoid direct WordPress DB access. |
| Pairing/trust boundary | Tauri visible controls | WP must be approved, scoped, revocable, and diagnosable. |

### Recommended Tauri UI Retention

Keep only native UI that is hard or unsafe to move to WP:

- first-run local runtime setup;
- WordPress pairing approval and revocation;
- MCP host setup and diagnostics;
- connector secrets and OAuth flows that require local trust;
- encryption-key recovery;
- database recovery/backup/export/start-fresh;
- updates and relaunch;
- app lock/unlock if local data is exposed;
- sensitive claim reveal approval;
- destructive privacy/data actions;
- runtime health, logs, and support bundle export;
- tray/menu Open, Run Now, Quit, Diagnostics.

Move or duplicate into WP only after a capability review:

- editorial review;
- claim browsing;
- Studio writing/editing;
- day/week/account/project/person views;
- publications and WordPress-native content workflows.

### Non-Recommendation

Do not choose Option B for the DOS-546 viability spike.
It may be a valid future architecture, but it is not a Phase 0 recommendation
because it turns a surface viability spike into a native daemon platform rewrite.

Do not choose Option C unless the product decision is to abandon "WP Studio as
primary" and reframe DOS-546 as "WP as additional surface."
That may be defensible, but it is a different spike.

## Open Questions

| Question | Why it matters | Suggested owner / next artifact |
|---|---|---|
| What is the exact WP-to-local-runtime transport? | Determines pairing, auth, CSRF/origin policy, streaming events, and failure modes. | DOS-546 Phase 1 local API design. |
| Is WordPress local, remote, or both? | A local WP install and a hosted WP site have very different trust and network assumptions. | Product/architecture decision before implementation. |
| What capabilities can WP invoke without fresh user approval? | Prevents WP from becoming a broad local data authority. | Pairing capability matrix. |
| What user-visible Tauri controls remain in MVP? | Sets scope discipline so Tauri does not remain the primary Studio accidentally. | Native control-center spec. |
| Where does MCP health appear when WP is primary? | ADR-0128 makes MCP a product surface; users need diagnostics. | Runtime diagnostics spec. |
| Does `dailyos-mcp` remain stdio-only or gain daemon/local-socket mode? | Affects Claude Desktop, CLI, and future agent hosts. | MCP host architecture note. |
| How are updates coordinated across Tauri, `dailyos-mcp`, and the WP plugin? | Version skew could break pairing or tool contracts. | Update/version compatibility plan. |
| Does the current macOS-only keychain provider block cross-platform claims? | The inventory requested Windows/Linux secret storage, but the code currently shows macOS `security` CLI only. | Secret-provider portability plan. |
| What is the single-instance policy for Tauri + MCP + WP pairing? | Current code lacks an explicit Tauri single-instance plugin. Multi-process runtime ownership needs a defined policy. | Runtime ownership/locking plan. |
| Which existing Tauri commands become ability-backed before WP uses them? | WP should consume stable runtime contracts, not legacy command sprawl. | Ability migration checklist. |
| How does WP degrade when Tauri is not running? | The primary Studio needs clear offline/down states and a route to start/open the local runtime. | WP runtime-unavailable UX spec. |
| How does Tauri degrade when WP is down? | Tauri is the escape hatch; it needs enough UI to preserve trust and core recovery. | Native escape-hatch flow. |
| What logs/support bundle can users export? | A paired WP/native/MCP architecture will be harder to debug without support artifacts. | Diagnostics/support bundle plan. |
| Which destructive or sensitive operations require local confirmation every time? | Claim reveal, data deletion, key reset, and external publish flows cannot rely only on WP buttons. | Trust-boundary policy. |
| What success criteria prove WP is primary? | Avoids Option C by drift. | DOS-546 spike acceptance rubric. |
