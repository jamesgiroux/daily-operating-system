# I465 — App Lock on Idle (Touch ID / macOS Auth)

**Status:** Pending
**Priority:** P1
**Version:** 0.15.2
**Area:** Backend + Frontend / Security
**ADR:** 0092

## Summary

DailyOS has no local access control. Anyone who can reach a running, unlocked instance (shared device, stepped-away laptop) sees all corporate intelligence immediately. This issue adds an inactivity lock that covers the running app with a full-window overlay and requires Touch ID or macOS password to resume.

## Acceptance Criteria

### Configuration

1. Settings → Security → "Lock after idle" setting. Options: 5 minutes, 15 minutes, 30 minutes, Never. Default: 15 minutes. Value persisted in `config.json` as `app_lock_timeout_minutes: Option<u32>` (null = Never).
2. Changing the timeout takes effect immediately for the next idle cycle -- no restart required.

### Lock trigger

3. The inactivity timer starts when the Tauri app window loses focus (`applicationDidResignActive` equivalent). If the window regains focus within the timeout window, the timer resets without locking.
4. When the timeout elapses while the window is out of focus, the lock engages before the window is next shown. The user should never see data briefly before the lock overlay appears.
5. If the window is in focus continuously (user is actively using the app), the lock does not trigger regardless of timeout. Activity is measured by window focus state, not by user input frequency.
6. "Never" disables the lock entirely. No overlay, no auth prompt, no timer.

### Lock screen

7. The lock overlay covers the entire app window -- no data from any page is visible behind it.
8. The overlay shows: the DailyOS app name (or brand mark), and a "Unlock" button or prompt. No entity names, account names, or data of any kind.
9. The lock overlay is a React component mounted above the router, rendered when `AppState.is_locked = true`. It is not a separate window.

### Unlock

10. Pressing the Unlock button (or the overlay itself) invokes macOS LocalAuthentication. The auth prompt title: "Unlock DailyOS". Uses biometrics (Touch ID) when available, falls back to macOS password automatically.
11. Successful authentication: overlay is removed, the user returns to the exact screen they were on. No navigation reset.
12. Failed/cancelled authentication: overlay remains. "Authentication cancelled" message shown below the unlock button. User can try again immediately.
13. After 3 failed attempts in one session, a 30-second cooldown is applied before the next attempt. (Deters automated attempts on an unattended machine.)

### Tauri implementation

14. A Rust Tauri command `lock_app()` sets `AppState.is_locked = true` and emits a `"app-locked"` event to the frontend.
15. A Rust Tauri command `unlock_app()` invokes macOS LocalAuthentication via the Security framework. On success, sets `AppState.is_locked = false` and emits `"app-unlocked"`. On failure, emits `"app-unlock-failed"` with reason.
16. The idle timer is managed in Rust (Tokio task), not the frontend. The frontend does not poll -- it reacts to emitted events.
17. macOS LocalAuthentication is invoked via `tauri-plugin-biometric` if available, otherwise via a direct FFI call to the `Security.framework`. The implementation must work on macOS 13+ (Ventura) and newer.

### Audit

18. `app_unlock_attempted`, `app_unlock_succeeded`, and `app_unlock_failed` are logged to the audit log (I471, same version) with `detail: {"trigger": "idle_timeout"}` or `{"trigger": "manual"}`.
19. If the audit logger (I471) is not yet available, stub the calls with `log::info!()` -- do not block this issue on I471.

## Files

### New
- `src/components/LockOverlay.tsx` — full-window overlay component
- `src/hooks/useAppLock.ts` — subscribes to `app-locked` / `app-unlocked` / `app-unlock-failed` events; manages local attempt counter and cooldown

### Modified
- `src-tauri/src/state.rs` — `is_locked: AtomicBool` on `AppState`
- `src-tauri/src/commands.rs` — `lock_app`, `unlock_app` commands
- `src-tauri/src/lib.rs` — register commands; startup idle timer Tokio task
- `src-tauri/src/types.rs` or `src-tauri/src/config.rs` — `app_lock_timeout_minutes` config field
- `src/App.tsx` or root router — mount `LockOverlay` above routes
- `src/pages/SettingsPage.tsx` or Settings component — lock timeout selector

## Notes

- The lock is defense against the "walked away from unlocked laptop" threat, not against a determined attacker with root access. It does not need to be cryptographically unbreakable.
- Do not attempt to freeze or unload the React app state during lock -- just cover it. Unloading creates re-hydration complexity and the overlay is sufficient.
- Touch ID availability varies: Intel Macs without Touch Bar have no Touch ID; those fall back to password only. LocalAuthentication handles this automatically -- no extra code needed.
- `tauri-plugin-biometric` was added to the Tauri plugin ecosystem but may need verification against the current Tauri v2 API. Evaluate at implementation time; direct FFI to `LocalAuthentication.framework` is the fallback.
