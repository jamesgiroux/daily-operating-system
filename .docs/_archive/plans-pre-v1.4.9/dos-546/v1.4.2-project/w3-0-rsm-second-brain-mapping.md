# W3-0 RSM Second Brain Mapping

DailyOS W3-A should borrow the RSM Second Brain plugin's WordPress scaffold discipline: one entry file, one composition root, domain-owned services, explicit REST registration, centralized hook names, runtime asset guards, and operator diagnostics. It should not borrow the RSM product substrate. DailyOS is a paired WordPress `SurfaceClient`; the Rust runtime remains the authority for abilities, claims, feedback, enrichment, and composition.

## 1. Plugin Entry And Bootstrap Pattern

**RSM source path:** `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/second-brain.php`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Plugin.php`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Activation.php`

**DailyOS target path:** `wp/dailyos/dailyos.php`, `wp/dailyos/includes/class-dailyos-plugin.php`

RSM's entry file does the right minimum: define constants, verify dependencies, load Composer, boot the composition root, and register lifecycle hooks. DailyOS should map this to `DailyOS_Plugin::instance()->init()` with activation/deactivation/uninstall methods on the main class per artifact 13, but the class must wire a SurfaceClient shell only: pairing state, ability proxy registration, admin pages, block registration, REST routes, and save hooks.

## 2. Domain Folder Structure

**RSM source path:** `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/`

**DailyOS target path:** `wp/dailyos/includes/`, `wp/dailyos/admin/`, `wp/dailyos/blocks/`

Borrow RSM's domain boundaries, not its domain semantics. DailyOS can keep artifact 13's `includes/class-dailyos-*.php` shape, but the ownership boundaries should mirror RSM so features do not collapse into one procedural plugin file.

| RSM domain | DailyOS analog | DailyOS target path | Borrowed pattern |
|---|---|---|---|
| `Admin/` | Pairing, settings, diagnostics pages | `wp/dailyos/admin/pages/`, `wp/dailyos/includes/class-dailyos-admin-assets.php` | Thin admin UI, nonce/capability-gated actions, diagnostics surfaced without owning substrate data. |
| `Ai/` | Runtime proxy and capability/status diagnostics | `wp/dailyos/includes/class-dailyos-runtime-client.php`, `wp/dailyos/includes/class-dailyos-runtime-gate.php` | Central client wrapper and availability gate. Do not call AI from PHP. |
| `Db/` | Minimal plugin-owned stores only | `wp/dailyos/includes/class-dailyos-presence-nonce.php` | Versioned/minimal storage if artifact 10 needs consumed nonces; no claims, entities, or composition substrate in WP. |
| `Editor/` | Gutenberg block and editor assets | `wp/dailyos/blocks/account-overview/`, `wp/dailyos/blocks/shared/` | Metadata-registered blocks and guarded editor assets. |
| `Ingest/` | Save-time feedback normalization | `wp/dailyos/includes/class-dailyos-feedback-router.php` | Convert surface events into typed service inputs. Do not ingest canonical content into WP. |
| `Rest/` | Pairing, preview, diagnostics, MCP REST surface | `wp/dailyos/includes/rest/` or `wp/dailyos/includes/class-dailyos-*-controller.php` | Controllers validate and register routes; services do the work. |
| `Scheduler/` | Client maintenance jobs | `wp/dailyos/includes/class-dailyos-scheduler-hooks.php` | Central hook constants for nonce sweeps, descriptor refresh, runtime health checks, and feedback outbox dispatch. |
| `Rag/` | Runtime/ability gate diagnostics | `wp/dailyos/includes/class-dailyos-runtime-gate.php` | `ok` / `degraded` / `unknown` gate state and re-check flow. No PHP RAG. |
| `Write/` | Composition render helpers, not AI writing | `wp/dailyos/blocks/shared/`, `wp/dailyos/includes/class-dailyos-composition-renderer.php` | Feature-specific helpers that prepare surface rendering around runtime outputs. |

## 3. Route Registrar And REST Controller Pattern

**RSM source path:** `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Rest/RouteRegistrar.php`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Rest/*Controller.php`

**DailyOS target path:** `wp/dailyos/includes/rest/class-dailyos-route-registrar.php`, `wp/dailyos/includes/rest/class-dailyos-pairing-controller.php`, `wp/dailyos/includes/rest/class-dailyos-diagnostics-controller.php`, `wp/dailyos/includes/class-dailyos-mcp-server.php`

RSM's registrar keeps `register_rest_route()` calls in one place while controllers self-describe their route, method, permission callback, args, and callback. DailyOS should use the same pattern with a `dailyos/v1` namespace for pairing status, diagnostics, block preview, and runtime health, while the custom MCP endpoint remains isolated behind `DailyOS_MCP_Server` and the same authorization discipline.

## 4. Service Ownership Of Mutations

**RSM source path:** `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Rest/ClipController.php`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Ingest/ClipIngestService.php`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Ingest/HighlightService.php`

**DailyOS target path:** `wp/dailyos/includes/class-dailyos-pairing.php`, `wp/dailyos/includes/class-dailyos-presence-nonce.php`, `wp/dailyos/includes/class-dailyos-feedback-router.php`, `wp/dailyos/includes/class-dailyos-runtime-client.php`

Borrow the good RSM split where controllers validate request shape, identity, capability, and IDs, then hand off to a domain service. For DailyOS, controllers and save hooks should never mutate claims directly; services may update plugin-owned pairing/options/transients/snapshots, and all authoritative writes must go through `DailyOS_Runtime_Client` to the Rust runtime.

## 5. Persist-First, Enrich-Async Lifecycle For Feedback

**RSM source path:** `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Ingest/ClipIngestService.php`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Scheduler/SummarizePostHandler.php`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Scheduler/HighlightPostHandler.php`

**DailyOS target path:** `wp/dailyos/includes/class-dailyos-feedback-router.php`, `wp/dailyos/includes/class-dailyos-feedback-outbox.php`, `wp/dailyos/includes/class-dailyos-runtime-client.php`

RSM persists the user's submitted clip first, marks AI-derived fields as `pending`, enqueues background work, and returns quickly. DailyOS should apply the same lifecycle to save-time feedback: persist the WP edit/projection state that WordPress owns, record a local pending feedback dispatch with the observed composition and presence nonce, send the typed feedback event to `/v1/surface/feedback`, and update `_dailyos_composition_snapshot` only after runtime acknowledgement.

## 6. Centralized Scheduler Hook Constants

**RSM source path:** `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Scheduler/Hooks.php`

**DailyOS target path:** `wp/dailyos/includes/class-dailyos-scheduler-hooks.php`

RSM avoids hard-coded async hook strings by using one constants class. DailyOS should do the same for `dailyos_nonce_sweep`, descriptor refresh, runtime health polling, projection cache cleanup, and feedback outbox dispatch, regardless of whether W3-A uses WP cron initially or a later Action Scheduler dependency.

## 7. Asset Manifest Verification Guard

**RSM source path:** `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Editor/SidebarAssets.php`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Admin/ClipListAssets.php`

**DailyOS target path:** `wp/dailyos/includes/class-dailyos-assets.php`, `wp/dailyos/blocks/account-overview/block.json`, `wp/dailyos/build/`

RSM reads `build/*.asset.php` before enqueueing scripts and simply skips the surface if the manifest is missing, avoiding PHP fatals in partially built installs. DailyOS should keep the runtime guard, but admin diagnostics should make missing compiled block/admin assets visible because a SurfaceClient with invisible pairing or block controls is effectively broken.

## 8. DataViews Admin Surface For Queues And Status

**RSM source path:** `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/clip-list/index.js`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Admin/ClipListAssets.php`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Admin/SettingsPage.php`

**DailyOS target path:** `wp/dailyos/admin/assets/status-dataviews.js`, `wp/dailyos/admin/assets/status-dataviews.css`, `wp/dailyos/admin/pages/settings.php`

RSM's DataViews surface turns queue/status metadata into an operator-readable table with fields, badges, filters, pagination, and actions. DailyOS should use the same admin UX for SurfaceClient status: pairing state, granted scopes, runtime health, ability descriptor snapshot age, pending feedback outbox items, nonce sweep status, projection cache state, and last render failures.

## 9. Capability Gate With `ok` / `degraded` / `unknown`

**RSM source path:** `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Rag/GateProbe.php`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Admin/RagGateStatus.php`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Rest/HealthController.php`

**DailyOS target path:** `wp/dailyos/includes/class-dailyos-runtime-gate.php`, `wp/dailyos/admin/pages/settings.php`, `wp/dailyos/includes/rest/class-dailyos-health-controller.php`

RSM treats capability readiness as explicit state instead of assuming the provider exists: `ok`, `degraded`, or `unknown`, with an admin re-check flow and health endpoint visibility. DailyOS should map this to runtime and ability readiness: `ok` means paired, loopback reachable, HMAC valid, descriptor snapshot current, and required scopes granted; `degraded` means render fallback/projection-only mode; `unknown` means not checked yet and should behave as degraded until probed.

## 10. Build-Asset Verification Guard

**RSM source path:** `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/scripts/verify-build-assets.sh`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/AGENTS.md`

**DailyOS target path:** `wp/dailyos/scripts/verify-build-assets.sh`, `wp/dailyos/package.json`, `.github/workflows/*`

RSM has a release-time script that verifies required built files exist and rejects unexpected build files; its agent docs also state that source JS/CSS changes must be rebuilt because WordPress serves compiled assets. DailyOS should borrow and strengthen this: CI should run the block/admin build, verify the expected `build/` and block asset manifests, and fail if compiled assets are missing or stale relative to source changes.

## Do Not Borrow

**CPT/meta/options as substrate.** RSM source paths: `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/PostTypes/ClipPostType.php`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Db/Schema.php`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Rag/ChunkRepository.php`. DailyOS may store plugin-owned pairing records, descriptor snapshots, nonces, projection snapshots, and diagnostics under `wp/dailyos/`, but claims, entities, embeddings, compositions, and feedback authority stay in the Rust runtime.

**Application Password auth.** RSM source path: `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/docs/foundations.md` section 7 and `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/AGENTS.md` smoke tests. DailyOS must use the v1.4.2 pairing handshake, bearer plus HMAC, SurfaceClient identity, WordPress capability checks, DailyOS scope grants, and presence nonces; WordPress Application Passwords are the wrong trust model for this SurfaceClient.

**Synchronous AI on persist paths.** RSM source paths to borrow as the counterexample guardrail: `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Ai/WpAiClient.php`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Ingest/ClipIngestService.php`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/docs/conventions.md`. DailyOS should not call AI from PHP at all, and save-time feedback must not block on model enrichment or composition regeneration.

**Full-prompt debug logs without privacy controls.** RSM source path: `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Ai/AiLogger.php`. DailyOS diagnostics must redact secrets, user content, HMAC material, bearer fragments, filesystem paths, prompts, and raw substrate payloads by default, with explicit privacy-reviewed debug controls before anything verbose is written.

**Direct controller writes as a habit.** RSM source paths: `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Rest/ImportController.php`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Rest/TranslateController.php`. DailyOS controllers should not contain `wp_insert_post`, claim writes, direct filesystem writes, or runtime mutation logic; they should validate and delegate to services that enforce the SurfaceClient boundary.

**PHP RAG, Write, and AI prompt semantics.** RSM source paths: `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Rag/`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/src/Write/`, `/Users/jamesgiroux/Downloads/rsm-second-brain-trunk/plugin-second-brain/prompts/`. DailyOS may borrow folder/process separation, but not PHP-side retrieval, prompt execution, embeddings, or draft generation; those remain runtime abilities or future substrate work.
