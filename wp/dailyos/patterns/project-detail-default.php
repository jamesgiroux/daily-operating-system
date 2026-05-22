<?php
/**
 * Title: Project Detail Default
 * Slug: dailyos/project-detail-default
 * Categories: dailyos
 * Description: Canonical 15-chapter composition for a Project entity surface (W2 V1.2.1 §5.2 — DOS-483). Insert-then-detach semantics — user reordering does not affect other instances (NOT a synced pattern).
 * Block Types: core/post-content
 * Inserter: no
 *
 * Filesystem pattern (theme-registered per wp-skill H4): ships the
 * canonical chapter ordering for the dailyos/project-detail composite
 * surface. Per W2 V1.2.1 §5.2 + wave §10 invariant "Outer/inner block
 * contract": `templateLock: false` on the outer block means users can
 * reorder; this pattern is the seed, not a lock.
 *
 * Empty `project_id` keeps the pattern preview safe in the editor — the
 * outer block renders an "is-empty" notice rather than invoking the
 * runtime against an unknown subject.
 */
?>
<!-- wp:dailyos/project-detail -->
<!-- wp:dailyos/project-hero /-->
<!-- wp:dailyos/vitals-strip /-->
<!-- wp:dailyos/portfolio-chapter /-->
<!-- wp:dailyos/trajectory-chapter /-->
<!-- wp:dailyos/horizon-chapter /-->
<!-- wp:dailyos/watch-list /-->
<!-- wp:dailyos/watch-list-milestones /-->
<!-- wp:dailyos/stakeholder-gallery /-->
<!-- wp:dailyos/the-work /-->
<!-- wp:dailyos/project-detail-touchpoints-feed /-->
<!-- wp:dailyos/project-detail-open-loops-feed /-->
<!-- wp:dailyos/linear-issues-chapter /-->
<!-- wp:dailyos/project-detail-unified-timeline /-->
<!-- wp:dailyos/project-detail-recommended-actions /-->
<!-- wp:dailyos/project-appendix /-->
<!-- /wp:dailyos/project-detail -->
