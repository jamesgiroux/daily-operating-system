# I350 — In-App Notifications — Release Announcements, What's New, System Status

**Status:** Open (0.14.0)
**Priority:** P1
**Version:** 0.14.0
**Area:** UX / Infrastructure

## Summary

DailyOS currently has no in-app notification system for release announcements, new feature callouts, or system status updates. When a new version ships, users learn about it only via the auto-updater (which installs silently) and have no in-app way to see what changed. This issue covers a lightweight notification/changelog surface: "What's New" on update, system status toasts for pipeline failures, and release announcements for significant features.

## Acceptance Criteria

Not yet specified for v0.14.0. Will be detailed in the v0.14.0 version brief. At minimum: a "What's New" surface accessible from the navigation, populated from a local changelog file updated with each release, and system status toast notifications for pipeline failures or auth errors (currently silent).

## Dependencies

- The auto-updater (I175, Sprint 23) handles the update delivery; this issue covers the post-update communication.
- Related to I348 (email digest) — system status might also be surfaced via email.

## Notes / Rationale

P1 because silent failures are a support burden. When the email enrichment pipeline fails, the user currently sees stale data with no explanation. A system status toast ("Email enrichment paused — tap to retry") converts a mystery into an actionable notification. The "What's New" component serves retention and feature discovery.
