---
status: spec:ready
date: 2026-05-10
related_adrs: [0102, 0111, 0129]
open_questions: see ./INDEX.md (routed to W2-C L0 Prep)
---

# Pairing Token Recovery Defenses

## Context

DOS-546 explores whether WordPress Studio can operate as a DailyOS surface without weakening the ability invocation contract.

The Phase 0 security question is not "can WordPress call DailyOS?" The question is whether a paired WordPress site can lose, replay, move, or leak its proof-of-pairing without becoming a durable unauthorized SurfaceClient.

The L0 CSO review for DOS-546 §0.1 names pairing token recovery defenses as the primary blocker for the spike. The review specifically rejects a single generic "re-pair on failure" control and requires four threat paths to receive separate named defenses:

- Reinstall: user reinstalls the DailyOS app or WP plugin.
- DB-restore: WordPress DB is restored from a backup containing an old proof-of-pairing.
- Site-switch: proof-of-pairing is migrated to a different WP site.
- Exfiltration: proof-of-pairing token is copied off the host.

That same §0.1 finding implies three design constraints for this artifact:

- Recovery must be path-specific because each path has different attacker economics and different reliable signals.
- The proof stored in WordPress must not be enough to mint new authority after runtime-side revocation.
- The runtime must keep the authoritative revocation and expiry state because the WordPress host is explicitly in scope as partially restorable, movable, or readable by an attacker.

ADR-0111 §1 defines surfaces as the actors responsible for constructing invocation context and lists Tauri, MCP, worker, eval, and test bridges. DOS-546 amends that model by treating WordPress Studio as a new paired SurfaceClient surface whose bridge still invokes abilities through the shared registry.

ADR-0111 §3 requires MCP tools to derive from the ability registry, and ADR-0111 §4 requires actor-filtered discovery. The WordPress MCP Adapter must preserve those properties: pairing can authorize a surface transport, but it cannot expose abilities outside policy.

ADR-0102 §7.1 defines `AbilityPolicy` around allowed actors, allowed modes, confirmation, and publish authority. For DOS-546, SurfaceClient policy extends that schema with `required_scopes` and `mcp_exposure` so a paired WordPress site can receive only the abilities explicitly granted to that surface.

ADR-0102 §7.4 requires actor-filtered introspection and states that unauthorized callers must not see ability names, schemas, or blast radius. A stale, moved, restored, or exfiltrated proof must therefore fail before ability discovery, not merely before mutation.

ADR-0129 §4 is the WP-side context for this spike: WordPress Studio uses a WP Abilities API and MCP Adapter as the primary surface integration. The design below keeps the WP side limited to a proof-of-pairing token in `wp_options` and never stores the runtime signing key in WordPress.

## Threat Model

### Reinstall

Reinstall covers either side being removed and installed again:

- The user reinstalls the DailyOS Tauri app and loses the local keychain entry holding the runtime signing key.
- The user reinstalls the WordPress plugin and loses or recreates plugin options.
- The user reinstalls one side while the other side still holds stale pairing material.

The realistic attacker model is opportunistic replay by a local user, a support workflow, or malware with access to old application support files or WordPress option exports. The attacker is not assumed to have the current runtime keychain signing key unless explicitly covered by exfiltration.

The main security failure would be silent resurrection: an old proof in WordPress becomes active again after a reinstall because the new runtime accepts it as familiar.

### DB-Restore

DB-restore covers a WordPress database rollback from backup.

The attacker model is an operator, compromised admin, compromised backup system, or hosting workflow that restores an older `wp_options` row containing proof-of-pairing material. This is realistic on WordPress because database backup and restore is a normal operational path, not an exceptional incident.

The main security failure would be time travel: a previously revoked or rotated pairing proof becomes accepted because the restored database contains a valid-looking older public identifier and signature.

### Site-Switch

Site-switch covers migration of a WordPress database, plugin directory, or option set to a different site.

The attacker model is a staging clone, multisite copy, migration plugin, or malicious admin moving the `wp_options` proof to another domain, path, site URL, or WP installation. The attacker may control the target WordPress host but does not control the user's DailyOS runtime.

The main security failure would be host confusion: proof issued for `site-a.example` works from `site-b.example`, allowing an attacker to present as the original surface.

### Exfiltration

Exfiltration covers copying the proof-of-pairing token off the WordPress host.

The attacker model includes read-only SQL compromise, backup dump access, plugin vulnerability, server-side file read that reveals exported options, or malicious WordPress admin. The attacker can copy the public identifier and signature stored in `wp_options`, but should not receive the runtime signing key because that key exists only in the Tauri runtime keychain.

The main security failure would be bearer-token use: the copied proof becomes sufficient to call DailyOS abilities from an attacker-controlled client.

## Defenses

### Reinstall: Anchor Rotation Handshake

Named defense: **Anchor Rotation Handshake**.

Threat description:

- Reinstall is a legitimate recovery action and cannot be treated as automatically malicious.
- The risk is that reinstall creates a new local identity while stale material on the other side still looks paired.
- The recovery path must distinguish "same human intentionally re-pairing" from "old proof silently accepted."

Defense:

- The runtime signing key is the anchor. It is generated by the Tauri runtime and stored in the OS keychain.
- On runtime reinstall, absence of the keychain entry creates a new runtime anchor, not a continuation of the old one.
- The new runtime anchor starts with an empty authoritative pairing table and empty revocation table except for imported user-approved state.
- WordPress plugin reinstall that loses its `wp_options` proof must initiate a fresh pairing challenge.
- WordPress plugin reinstall that preserves a stale proof must present it, but the runtime rejects it unless its `runtime_anchor_id` maps to an active record under the current keychain signing key.
- Re-pairing requires an explicit user-visible handshake that rotates the WordPress proof and revokes the previous pairing id.

Storage split:

- Runtime side: OS keychain entry stores the runtime signing key and its `runtime_anchor_id`.
- Runtime side: local runtime state stores pairing records, revocation records, expiry timestamps, scopes, site binding, and audit ids.
- WP side: `wp_options` stores only proof-of-pairing: public pairing identifier, signed proof, issued-at, expiry hint, scope hint, and site binding claims.
- WP side: no runtime signing key, no private key capable of minting new pairings.

Revocation behavior:

- The runtime revokes the previous pairing id whenever Anchor Rotation Handshake completes for the same site.
- If the old runtime anchor is gone, the new runtime cannot cryptographically revoke records it no longer has; it instead refuses all proofs not issued by the current anchor and emits a reinstall recovery audit event.
- Clock-based expiry limits the useful life of orphaned proofs if an old runtime state file reappears later.

Phase 1 fixtures:

- Delete the runtime keychain entry, keep WP proof in `wp_options`, start DailyOS, and verify the old proof is rejected as `unknown_runtime_anchor`.
- Delete WP plugin options, keep runtime pairing active, reinstall plugin, and verify a new pairing flow creates a new pairing id while revoking the old id.
- Simulate both sides reinstalling and verify no ability discovery happens until a fresh user-confirmed pairing is completed.

### DB-Restore: Monotonic Pairing Epoch

Named defense: **Monotonic Pairing Epoch**.

Threat description:

- WordPress backup restore can reintroduce an older proof after the user intentionally revoked, rotated, or narrowed a pairing.
- The restored proof may still have a valid signature because signatures prove historical issuance, not current authority.
- The runtime must treat current authority as a server-side state question, not a signature-only question.

Defense:

- Every pairing record has a monotonic `pairing_epoch` assigned by the runtime.
- The proof stored in `wp_options` includes the pairing id, epoch, issued-at timestamp, expiry timestamp, site binding, and signed scope digest.
- The runtime stores the highest observed epoch per site binding and per pairing subject.
- A proof with an epoch lower than the runtime's stored floor is rejected as `restored_stale_pairing`.
- Revocation records include the epoch and pairing id so a restored proof cannot bypass revocation by replaying an earlier row.
- Clock-based expiry is defense-in-depth only; epoch rejection is authoritative.

Storage split:

- Runtime side: keychain entry stores only the runtime signing key; local runtime state stores epoch floor, active pairing records, revoked pairing records, and expiry metadata.
- WP side: `wp_options` stores proof-of-pairing token with public id and signature, including epoch claims, but never the signing key.
- WP side: restored backups may contain old proofs, but those proofs cannot lower the runtime epoch floor.

Revocation behavior:

- Runtime-side revocation is authoritative and survives WordPress restore because the revocation list is not sourced from WordPress.
- A restored WP proof transitions to `revoked` if its pairing id is in the revocation list.
- A restored WP proof transitions to `expired` if it is past expiry even when no revocation record exists.
- A restored WP proof transitions to rejected stale state before invocation if its epoch is below the runtime floor.

Phase 1 fixtures:

- Pair at epoch 10, rotate to epoch 11, restore WP DB containing epoch 10, and verify ability discovery fails with `restored_stale_pairing`.
- Pair, revoke, restore a backup from before revocation, and verify the runtime revocation list still rejects the proof.
- Restore a proof whose expiry is in the past and verify it is classified as `expired` even if the epoch is otherwise current.

### Site-Switch: Site-Bound Proof Envelope

Named defense: **Site-Bound Proof Envelope**.

Threat description:

- WordPress sites are frequently cloned to staging, migrated across domains, or duplicated by backup tools.
- A proof that is only bound to a WordPress option row can move with the database.
- The attacker may control a cloned host and can replay outbound calls with the copied proof.

Defense:

- The runtime signs a proof envelope that binds the pairing to canonical site identity claims.
- Required site binding claims include normalized `home_url`, normalized `site_url`, WP installation UUID, plugin instance UUID, and optional multisite blog id.
- The WP plugin generates and stores a stable plugin instance UUID on first install.
- The runtime stores the expected site binding digest with the pairing record.
- Every ability discovery and invocation request includes the current site binding claims.
- The runtime recomputes the binding digest and rejects mismatches as `site_binding_mismatch`.
- Site URL change is a recovery flow, not automatic acceptance. It must produce a new proof and audit entry.

Storage split:

- Runtime side: keychain entry holds the signing key; runtime pairing table holds the accepted site binding digest and allowed migration state.
- WP side: `wp_options` holds proof-of-pairing plus public site binding claims and plugin instance UUID.
- WP side: changing `home_url`, `site_url`, or site UUID without re-pairing does not update authority because the signature no longer matches current claims.

Revocation behavior:

- On site binding mismatch, the runtime does not immediately delete the active record; it suspends use of the presented proof and requires re-pairing or explicit site migration approval.
- If the user approves migration, the runtime revokes the old proof and issues a new active proof bound to the new site claims.
- If the user denies or ignores migration, the old proof remains unusable from the switched site and expires on schedule.

Phase 1 fixtures:

- Pair on `https://example.test`, clone DB to `https://staging.example.test`, and verify discovery fails with `site_binding_mismatch`.
- Change only `home_url` and verify the runtime requires site migration approval before issuing a replacement proof.
- Copy `wp_options` proof and plugin instance UUID to a different WP install UUID and verify the install UUID mismatch blocks invocation.

### Exfiltration: Split-Key Non-Bearer Proof

Named defense: **Split-Key Non-Bearer Proof**.

Threat description:

- WordPress secrets are exposed more often than OS keychain secrets because WP data moves through SQL dumps, host backups, admin tools, and plugins.
- The attacker may obtain the full `wp_options` proof-of-pairing token.
- The copied token must not be sufficient to invoke abilities from another process or host.

Defense:

- Treat the WP token as a signed public proof, not as a bearer credential.
- The proof identifies a pairing and carries a runtime signature over claims, but it does not contain the runtime signing key.
- Every request must prove possession of the paired WordPress-side request secret or ephemeral challenge response where available in Phase 1.
- If Phase 1 does not ship a WP private key, the minimum viable defense is strict site binding, short clock expiry, runtime revocation, and per-call nonce replay protection.
- The runtime maintains nonce windows per active pairing id and rejects duplicate nonces as replay.
- A copied proof from `wp_options` fails off-host because the presented site binding, nonce history, and challenge response do not match the runtime's active record.
- Scopes are resolved runtime-side from the active pairing record, not trusted from the copied token.

Storage split:

- Runtime side: keychain stores runtime signing key; runtime state stores active pairing id, nonce window, revocation list, expiry, scopes, and current site binding.
- WP side: `wp_options` stores public identifier and signed proof only.
- WP side: any WP-side request secret, if introduced, must be generated separately from the proof and rotated on re-pair; it still must not be the runtime signing key.

Revocation behavior:

- User-initiated "disconnect site" immediately transitions the pairing to `revoked` in runtime state.
- Runtime revocation blocks all subsequent discovery and invocation attempts regardless of whether the copied proof has not expired.
- Expiry limits the attack window if exfiltration occurs before revocation is noticed.
- Nonce replay detection produces a suspicious-use audit event and may automatically revoke depending on policy severity.

Phase 1 fixtures:

- Copy the `wp_options` proof to a script that calls the runtime without matching site binding claims and verify rejection before ability introspection.
- Replay an identical signed invocation nonce twice and verify the second call fails with `nonce_replay`.
- Revoke a pairing while an attacker continues sending copied proofs and verify all subsequent calls fail with `pairing_revoked`.

## Token Storage Topology

The topology is intentionally asymmetric.

Runtime side, Tauri:

- Stores the runtime signing key in the OS keychain.
- Stores `runtime_anchor_id`, active pairing records, revoked pairing records, expiry timestamps, site binding digests, epoch floors, scope grants, and nonce windows in runtime-managed local state.
- Owns all authority decisions.
- Signs pairing proofs.
- Verifies presented proofs.
- Resolves effective scopes from runtime state.
- Owns ability invocation authorization before dispatching to the registry.

WP side, `wp_options`:

- Stores `dailyos_pairing_id`.
- Stores `dailyos_pairing_proof` as a public identifier plus runtime signature over pairing claims.
- Stores `dailyos_pairing_issued_at` and `dailyos_pairing_expires_at` as hints for UX and early local failure.
- Stores `dailyos_pairing_site_claims` used to construct the site binding digest.
- Stores `dailyos_pairing_scope_hint` only for display; runtime state remains authoritative.
- Does not store the runtime signing key.
- Does not store a key capable of signing new runtime-accepted proofs.
- Does not own revocation state.

The proof-of-pairing token is therefore not a bearer credential. It is a signed statement that must be checked against runtime-side state, revocation, expiry, site binding, epoch, nonce, scopes, and lifecycle status.

## Revocation Flow

Runtime-side revocation is authoritative.

The flow:

1. Runtime receives a disconnect, rotation, suspicious replay, site migration denial, or reinstall recovery event.
2. Runtime looks up the pairing id and marks it `revoked` with reason, clock timestamp, actor, prior lifecycle state, site binding digest, and scope digest.
3. Runtime writes the revoked pairing id to the revocation list before issuing any replacement proof.
4. Runtime invalidates in-memory ability discovery caches for the pairing id.
5. Runtime invalidates nonce windows for the revoked pairing id.
6. Runtime rejects future discovery and invocation requests before ability registry enumeration.
7. Runtime emits an audit event.
8. WordPress is informed best-effort so the plugin can clear `wp_options`, but failure to clear WordPress does not affect authority.

Clock-based expiry is defense-in-depth:

- Every proof carries an expiry timestamp.
- Runtime enforces expiry even if the proof is not on the revocation list.
- Expiry does not replace revocation because DB-restore can replay unexpired historical material.
- Expiry limits damage from orphaned proofs after lost runtime state, abandoned sites, or unnoticed exfiltration.

Replacement proof rule:

- Any recovery flow that issues a new proof must revoke the old proof first.
- If the old proof id is unknown to the current runtime anchor, the runtime must not mark it active by inference.
- Re-pair creates a new pairing id, new epoch, new site binding digest, new expiry, and new audit lineage.

## Lifecycle State Machine

States:

- `issued`: runtime has created a signed proof but the WP side has not yet completed activation.
- `active`: WP side has acknowledged proof storage and the runtime accepts discovery and invocation subject to policy.
- `revoked`: runtime has explicitly removed authority before natural expiry.
- `expired`: runtime clock has passed the proof expiry.

Allowed transitions:

- `issued -> active`: WP plugin stores proof in `wp_options` and completes activation handshake.
- `issued -> expired`: activation window passes before WP acknowledges storage.
- `issued -> revoked`: user cancels pairing or runtime detects a conflicting recovery.
- `active -> revoked`: user disconnect, rotation, reinstall recovery, suspicious replay, denied site switch, or admin policy action.
- `active -> expired`: clock reaches proof expiry without renewal.
- `revoked -> expired`: optional archival transition after expiry for retention compaction; authority remains denied.

Threat-path transitions:

- Reinstall transitions old known pairings from `active -> revoked` when the runtime can see them and from presented unknown proofs to rejected `unknown_runtime_anchor` when it cannot.
- DB-restore presents an old proof that maps to `revoked`, `expired`, or stale epoch rejection; it never transitions back to `active`.
- Site-switch suspends the presented proof and then transitions `active -> revoked` if migration is denied or `active -> revoked` plus new `issued -> active` if migration is approved.
- Exfiltration transitions `active -> revoked` on user disconnect, suspicious replay auto-revocation, or incident response; copied proofs past the timestamp are `expired`.

State checks must run before ADR-0102 §7.4 actor-filtered introspection and before ADR-0111 §3 MCP tool registration output is exposed to the WordPress surface.

## Audit Requirements

Every recovery path must produce audit entries that are useful to both security review and user-facing support.

Common fields:

- `event_id`
- `event_type`
- `occurred_at`
- `runtime_anchor_id`
- `pairing_id`
- `previous_pairing_id`
- `pairing_epoch`
- `lifecycle_from`
- `lifecycle_to`
- `site_binding_digest`
- `scope_digest`
- `actor`
- `source_surface`
- `reason`
- `request_id`
- `ability_invocation_id` when applicable
- `decision`

Reinstall audit events:

- `pairing.reinstall.runtime_anchor_missing`
- `pairing.reinstall.wp_proof_orphaned`
- `pairing.reinstall.repair_started`
- `pairing.reinstall.old_pairing_revoked`
- `pairing.reinstall.new_pairing_issued`

DB-restore audit events:

- `pairing.restore.stale_epoch_detected`
- `pairing.restore.revoked_proof_presented`
- `pairing.restore.expired_proof_presented`
- `pairing.restore.wp_cleanup_requested`

Site-switch audit events:

- `pairing.site_binding.mismatch_detected`
- `pairing.site_binding.migration_requested`
- `pairing.site_binding.migration_approved`
- `pairing.site_binding.migration_denied`
- `pairing.site_binding.old_pairing_revoked`

Exfiltration audit events:

- `pairing.exfiltration.suspected_replay`
- `pairing.exfiltration.nonce_replay`
- `pairing.exfiltration.off_host_binding_failure`
- `pairing.exfiltration.user_revoked`
- `pairing.exfiltration.auto_revoked`

Ability invocation audit events:

- `ability.invocation.pairing_checked`
- `ability.invocation.pairing_revoked_mid_call`
- `ability.invocation.cancelled_for_revocation`
- `ability.invocation.completed_after_revocation_as_readonly`
- `ability.invocation.result_suppressed_for_revocation`

Audit posture:

- Rejected stale proofs are auditable even when no ability is invoked.
- Unauthorized callers must not learn ability names, schemas, or blast radius, consistent with ADR-0102 §7.4.
- Scope decisions should record the policy digest, not expand sensitive policy content in logs by default.

## In-Flight Ability Invocation Semantics

Revocation can happen while a WordPress-initiated ability call is already running.

The runtime must support a revocation check at these points:

- Before ability discovery.
- Before invocation dispatch.
- Before every mutating service boundary.
- Before external publish.
- Before returning result material to the WordPress surface.

Behavior:

- Read and Transform abilities already in progress may finish internally, but the runtime re-checks pairing state before returning output.
- If the pairing is revoked before return, the runtime suppresses the result and returns a revocation error envelope without domain data.
- Publish abilities must check revocation immediately before external write; if revoked, they fail closed.
- Maintenance abilities invoked through WordPress SurfaceClient must check revocation before mutation and fail closed if revoked.
- If a child ability is running inside a composed call, the top-level surface invocation id carries the revoked status and the final result is suppressed.

Rationale:

- ADR-0102 §10 treats Transform outputs as untrusted for mutation authorization; revocation must similarly prevent a stale surface from converting already-produced data into action.
- ADR-0102 §11.2 forbids crossing transactional and external-publish boundaries casually; revocation checks must preserve that discipline by failing before mutations or external writes.
- ADR-0111 §1 makes the bridge responsible for context construction; the WordPress SurfaceClient bridge is therefore responsible for carrying pairing state into invocation cancellation and result suppression.

## Phase 1 Test Fixtures

### Fixture 1: Runtime Reinstall Orphans WP Proof

Setup:

- Pair WordPress site with DailyOS.
- Confirm ability discovery works.
- Delete the runtime keychain entry and runtime local pairing table.
- Keep WordPress `wp_options` proof unchanged.

Expected:

- WordPress discovery request fails before ability enumeration.
- Error class is `unknown_runtime_anchor` or equivalent reinstall recovery failure.
- Audit emits `pairing.reinstall.runtime_anchor_missing` and `pairing.reinstall.wp_proof_orphaned`.
- No ability names or schemas are returned.

### Fixture 2: WP Plugin Reinstall Rotates Pairing

Setup:

- Pair WordPress site with DailyOS.
- Remove plugin options as if the plugin was uninstalled.
- Reinstall plugin and initiate pairing again for the same site binding.

Expected:

- Runtime issues a new pairing id and higher epoch.
- Runtime revokes the previous pairing id before activating the new proof.
- Old proof replay fails with `pairing_revoked`.
- Audit emits `pairing.reinstall.old_pairing_revoked` and `pairing.reinstall.new_pairing_issued`.

### Fixture 3: DB Restore Replays Revoked Proof

Setup:

- Pair at epoch 1.
- Take a WP DB backup.
- Rotate or revoke pairing so runtime epoch floor is 2 or pairing id is revoked.
- Restore the old WP DB backup containing epoch 1 proof.

Expected:

- Runtime rejects discovery before registry introspection.
- Error class is `restored_stale_pairing` or `pairing_revoked`.
- Audit emits `pairing.restore.stale_epoch_detected` or `pairing.restore.revoked_proof_presented`.
- WordPress cleanup is requested but not required for security.

### Fixture 4: Site Clone Attempts Invocation

Setup:

- Pair `https://source.example.test`.
- Clone database and plugin options to `https://clone.example.test`.
- Present the copied proof from the clone.

Expected:

- Runtime recomputes site binding digest and rejects the request.
- Error class is `site_binding_mismatch`.
- Audit emits `pairing.site_binding.mismatch_detected`.
- No ability discovery or invocation occurs.

### Fixture 5: Exfiltrated Proof Replayed Off-Host

Setup:

- Pair a WordPress site.
- Copy the `wp_options` proof into an external client script.
- Attempt discovery and invocation from the script with missing or mismatched site binding claims.

Expected:

- Runtime rejects the proof before ability introspection.
- Audit emits `pairing.exfiltration.off_host_binding_failure`.
- Runtime does not trust scope hints from the copied token.
- If nonce replay is attempted, audit emits `pairing.exfiltration.nonce_replay`.

### Fixture 6: Mid-Call Revocation Suppresses Result

Setup:

- Start a long-running Transform ability from WordPress SurfaceClient.
- Revoke the pairing before the ability returns.

Expected:

- Ability may complete internally if it has no mutation path.
- Runtime suppresses domain output before returning to WordPress.
- Response is a revocation error envelope.
- Audit emits `ability.invocation.pairing_revoked_mid_call` and `ability.invocation.result_suppressed_for_revocation`.

### Fixture 7: Publish Revoked Before External Write

Setup:

- Start a Publish ability from WordPress SurfaceClient with valid confirmation.
- Pause execution immediately before external write.
- Revoke the pairing.
- Resume execution.

Expected:

- Publish ability fails closed before external write.
- No external side effect occurs.
- Audit emits `ability.invocation.cancelled_for_revocation`.
- The pairing remains `revoked`.

## Open Questions

- ADR-0129 §4 was not present in this worktree at authoring time. Phase 1 should reconcile this artifact with the final WP Abilities API and MCP Adapter text before implementation.
- The exact SurfaceClient actor representation needs a formal ADR amendment: new `Actor::SurfaceClient`, `Actor::Agent` with surface-bound subject, or a separate transport principal mapped to existing actors.
- The exact `required_scopes` and `mcp_exposure` fields should be added to the canonical `AbilityPolicy` schema or represented as a SurfacePolicy wrapper layered over ADR-0102 §7.1.
- Phase 1 needs to decide whether WordPress gets a request-signing key pair in addition to the runtime-signed proof. The stronger design is mutual proof-of-possession; the minimum viable spike can start with site binding, nonce replay protection, runtime revocation, and short expiry.
- The retention period for revoked pairing records must balance local-first storage constraints against DB-restore risk. A short revocation list retention weakens restore detection.
- User experience for legitimate site URL changes needs product design: automatic hard failure is secure, but WordPress migrations are common enough that recovery must be understandable.
- The audit log privacy model needs a final decision on whether raw site URLs are logged or only site binding digests plus redacted host labels.
- Phase 1 should decide whether suspicious nonce replay auto-revokes immediately or requires user confirmation after a threshold.
