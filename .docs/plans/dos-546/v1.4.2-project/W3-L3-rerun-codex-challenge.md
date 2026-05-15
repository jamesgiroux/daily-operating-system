VERDICT: BLOCKED

1. Severity: HIGH
   File: `wp/dailyos/includes/class-dailyos-activation.php:120`
   AC-bound? yes (`.docs/plans/dos-546/v1.4.2-project/W3-L0-packet.md:88`)
   Bug: `marker_matches_prior_pair()` accepts a dirty namespace when marker fields are only self-consistent (`instance_id` equals `runtime_instance_id`), so a forged DB marker can bypass the runtime-reported-state requirement.
   Fix: Compare the marker against runtime-authoritative pairing state (`runtime_instance_id`, `site_nonce_hash`, `projection_version`) and quarantine when runtime state is unavailable or mismatched.

2. Severity: HIGH
   File: `wp/dailyos/includes/services/class-dailyos-namespace-store.php:25`
   AC-bound? yes (`.docs/plans/dos-546/v1.4.2-project/W3-L0-packet.md:86`)
   Bug: The namespace-vacancy report only scans options, `_dailyos_` post meta, transients, and tables, so first activation misses reserved post types, `dailyos_` user-meta, and pre-existing `dailyos_substrate` role/user collisions.
   Fix: Extend vacancy detection to reserved post types, user-meta keys, role slug, and substrate login/user state, then refuse activation without a runtime-confirmed prior pairing instead of silently adopting.

Bottom-line summary: The folded timestamp and path-alpha findings stay closed, and I did not re-raise them. W3 is still blocked on the W3-A namespace trust boundary: activation can still treat forgeable or incomplete WordPress-local state as proof of prior ownership, while the acceptance packet makes runtime-reported state and complete namespace vacancy the load-bearing checks.
