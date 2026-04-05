# ADR-0039: Feature toggle architecture

**Date:** 2026-02-06
**Status:** Accepted
**Evolves:** [ADR-0026](0026-extension-architecture.md) (extension architecture)

## Context

ADR-0026 established extensions as profile-activated module bundles. This works for distribution boundaries but doesn't give users granular control. A CS user might want account tracking but not post-meeting capture. The monolithic extension model doesn't support this.

Every capability should be individually toggleable. Roles (profiles) become preset feature configurations, not rigid bundles.

## Decision

Three-level hierarchy: Extension > Feature > Profile.

**Extension** = a bundle of features and configurations that creates a role experience. Unit of distribution and SDK boundary. Can be first-party or third-party.

**Feature** = individually toggleable capability within an extension. Each has a boolean toggle in Settings. Examples: "Account tracking", "Post-meeting capture", "Email triage", "Weekly planning".

**Profile** = the active extension preset. Selecting "CS" activates the CS Extension with all its features defaulted on. Users can then toggle individual features off.

**Config schema:**
```json
{
  "profile": "customer-success",
  "features": {
    "accountTracking": true,
    "postMeetingCapture": true,
    "emailTriage": true,
    "weeklyPlanning": true
  }
}
```

**Key rules:**
1. Features within an extension default to on for the active profile.
2. Users can toggle any feature off in Settings without changing profile.
3. ADR-0026's extension boundaries remain the distribution/SDK unit.
4. Third-party extensions can be developed to create new profiles — first-party CS extension proves the pattern, then open the SDK.
5. Feature flags are checked at runtime in Rust and React. Missing keys default to `true` for the active profile's extension.

## Consequences

- Users get granular control without needing to understand extensions
- Settings page gets a "Features" section with toggles (UI implementation is separate work)
- Extension authors define which features their extension provides and their defaults
- Migration path: existing boolean configs (e.g., `postMeetingCapture.enabled`) map directly to feature toggles
- ADR-0026's Phase 4/5 timeline for extension SDK still applies — this ADR adds granularity within extensions, not a new timeline
