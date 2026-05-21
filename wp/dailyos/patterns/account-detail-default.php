<?php
/**
 * Title: Account Detail Default
 * Slug: dailyos/account-detail-default
 * Categories: dailyos
 * Description: Canonical 24-chapter composition for an Account entity surface (W2 V1.2.1 §5.1 — DOS-462). Insert-then-detach semantics — user reordering does not affect other instances (NOT a synced pattern).
 * Block Types: core/post-content
 * Inserter: no
 *
 * Filesystem pattern (theme-registered per wp-skill H4): ships the
 * canonical chapter ordering for the dailyos/account-detail composite
 * surface. Per W2 V1.2.1 §5.1 + wave §10 invariant "Outer/inner block
 * contract": `templateLock: false` on the outer block means users can
 * reorder; this pattern is the seed, not a lock.
 *
 * Empty `account_id` keeps the pattern preview safe in the editor — the
 * outer block renders an "is-empty" notice rather than invoking the
 * runtime against an unknown subject.
 */
?>
<!-- wp:dailyos/account-detail -->
<!-- wp:dailyos/account-hero /-->
<!-- wp:dailyos/sentiment-hero /-->
<!-- wp:dailyos/triage-section /-->
<!-- wp:dailyos/divergence-section /-->
<!-- wp:dailyos/outlook-panel /-->
<!-- wp:dailyos/on-track-chapter /-->
<!-- wp:dailyos/supporting-tension /-->
<!-- wp:dailyos/about-intelligence /-->
<!-- wp:dailyos/account-pull-quote /-->
<!-- wp:dailyos/stakeholder-grid /-->
<!-- wp:dailyos/strategic-landscape /-->
<!-- wp:dailyos/value-commitments /-->
<!-- wp:dailyos/quote-wall /-->
<!-- wp:dailyos/commercial-shape /-->
<!-- wp:dailyos/account-technical-footprint /-->
<!-- wp:dailyos/relationship-fabric /-->
<!-- wp:dailyos/about-this-dossier /-->
<!-- wp:dailyos/recommended-actions /-->
<!-- wp:dailyos/touchpoints-feed /-->
<!-- wp:dailyos/open-loops-feed /-->
<!-- wp:dailyos/file-list /-->
<!-- wp:dailyos/linear-issues-chapter /-->
<!-- wp:dailyos/unified-timeline /-->
<!-- wp:dailyos/finis-marker /-->
<!-- /wp:dailyos/account-detail -->
