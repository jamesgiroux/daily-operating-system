# W5 Symptom Reproduction — Cold-Start Drift

**Date:** 2026-05-19
**Studio sandbox:** `~/Studio/dailyos-dev` at `http://localhost:8884`
**Plugin code in play:** `/tmp/dailyos-w3/wp/dailyos` (symlinked into `wp-content/plugins/dailyos`)
  - Identical to dev tree for the transport layer (`class-dailyos-runtime-client.php`, `dailyos.php` sentinel discovery)
  - Post-W4 reproduction would yield same transport behavior; only the affordance UI differs
**Tauri runtime:** not running

## Observed state

```
~/.dailyos/runtime-endpoint.json     →  ABSENT  (Packet A explicit_sentinel_cleanup worked)
dailyos_pairing_marker.runtime_url   →  http://127.0.0.1:50633   (stale, port dead)
dailyos_pairing_status               →  paired
nc -z 127.0.0.1 50633                →  PORT 50633 DEAD (ECONNREFUSED)
```

## User-visible failure

`GET http://localhost:8884/accounts/acme-corporation/` (post 14, dailyos_account CPT):

- HTTP 200, 112 KB rendered
- DailyOS block containers render with `is-empty` class:
  - `wp-block-dailyos-account-overview is-empty`
  - `wp-block-dailyos-entity-chip dailyos-primitive-inline is-empty`
  - `wp-block-dailyos-type-badge dailyos-primitive-inline is-empty`
- No error banner, no recovery affordance, no diagnostic surfaced to user
- **Block container exists but data fetch failed silently.** Render fallback to `.is-empty` makes the failure invisible.

## Why the existing retry doesn't catch this

Per `wp/dailyos/includes/transport/class-dailyos-runtime-client.php`:

- L530-560: discovery prefers `DailyOS_Plugin::discover_runtime_base_url()` (sentinel) over marker URL. With sentinel absent, falls back to marker URL.
- L295-299: on ECONNREFUSED, "invalidate the sentinel cache, re-discover, and retry once". Re-discovery still finds no sentinel → request fails again, same port.
- Net effect: stale marker URL is the only candidate; retry produces same ECONNREFUSED; block ends in `is-empty` state.

## Failure shape characterized

| Path | Sentinel | Marker | Outcome |
|---|---|---|---|
| 1. Cold start (REPRODUCED HERE) | absent | stale port | block renders is-empty, no diagnostic |
| 2. Hot Tauri restart (NOT YET REPRODUCED) | fresh new port | stale | discover_runtime_base_url returns sentinel → should succeed; race risk on sentinel-write timing |
| 3. Hot Studio sandbox restart (NOT YET REPRODUCED) | stable | stale | sentinel takes precedence → should succeed; risk in marker-cache TTL or boot order |

Cases 2 and 3 require Tauri runtime up + pairing completed + manual restart sequences. Path 1 is the cold-start case any user hits when launching Studio without first launching Tauri.

## Evidence artifacts in this directory

- `wp-options-paired-marker.txt` — full dailyos_* WP options snapshot showing stale marker
- `sentinel-absent.txt` — confirmation that `~/.dailyos/runtime-endpoint.json` does not exist
- `port-50633-state.txt` — `nc -z` probe confirming port is dead
- `rendered-acme-account-page.html` — full HTML body of the failed render
- `block-render-failure.txt` — count + breakdown of `is-empty` block instances

## Implication for L0 packet G

The single biggest fix-target is **case 1** (cold-start): users open Studio before launching Tauri, see a silently-empty block, do not know to launch Tauri or how the discovery contract works. Two design options surface from this:

1. **Active probe + user-facing diagnostic.** When marker resolves but ECONNREFUSED happens repeatedly and sentinel re-discovery returns nothing, render a typed "DailyOS runtime not running — start the Tauri app and refresh" notice in place of the empty block. This is a presentation-layer fix; no transport-layer change required.
2. **Self-launch handshake.** macOS URL scheme `dailyos://` registers Tauri app; when WP plugin detects ECONNREFUSED + missing sentinel, it could prompt the user to launch (or auto-launch via `Launch Services` from the renderer). More invasive; introduces platform-coupling.

L0 packet G should evaluate both. Case 1 alone justifies the W5 wave; cases 2/3 are secondary failure modes worth covering once we have Tauri up.
