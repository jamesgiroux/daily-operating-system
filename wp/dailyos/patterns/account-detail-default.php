<?php
/**
 * Default Account Detail composition (filesystem pattern).
 *
 * Per L0-packet-W2-entity-surfaces.md V1.2.1 §5.1 + AC-462.8: filesystem
 * pattern (theme-registered via register_block_pattern), insert-then-detach
 * semantics — NOT a DB-stored synced pattern. User reordering of one
 * instance does not affect other inserted instances. The pattern mirrors
 * the canonical chapter ordering declared in account-detail/block.json
 * "template" so an empty-context insert renders meaningfully.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return;
}

if ( ! function_exists( 'register_block_pattern' ) ) {
	return;
}

register_block_pattern(
	'dailyos/account-detail-default',
	[
		'title'       => __( 'Account Detail — default composition', 'dailyos' ),
		'description' => __( 'Canonical 24-chapter Account Detail composition. Translation of src/pages/AccountDetailPage.tsx into Gutenberg inner blocks. User can reorder or remove chapters in the Site Editor.', 'dailyos' ),
		'categories'  => [ 'dailyos' ],
		'blockTypes'  => [ 'dailyos/account-detail' ],
		'content'     => implode(
			"\n",
			[
				'<!-- wp:dailyos/account-detail -->',
				'<!-- wp:dailyos/account-hero /-->',
				'<!-- wp:dailyos/sentiment-hero /-->',
				'<!-- wp:dailyos/triage-section /-->',
				'<!-- wp:dailyos/divergence-section /-->',
				'<!-- wp:dailyos/outlook-panel /-->',
				'<!-- wp:dailyos/on-track-chapter /-->',
				'<!-- wp:dailyos/supporting-tension /-->',
				'<!-- wp:dailyos/about-intelligence /-->',
				'<!-- wp:dailyos/account-pull-quote /-->',
				'<!-- wp:dailyos/stakeholder-grid /-->',
				'<!-- wp:dailyos/strategic-landscape /-->',
				'<!-- wp:dailyos/value-commitments /-->',
				'<!-- wp:dailyos/quote-wall /-->',
				'<!-- wp:dailyos/commercial-shape /-->',
				'<!-- wp:dailyos/account-technical-footprint /-->',
				'<!-- wp:dailyos/relationship-fabric /-->',
				'<!-- wp:dailyos/about-this-dossier /-->',
				'<!-- wp:dailyos/recommended-actions /-->',
				'<!-- wp:dailyos/touchpoints-feed /-->',
				'<!-- wp:dailyos/open-loops-feed /-->',
				'<!-- wp:dailyos/file-list /-->',
				'<!-- wp:dailyos/linear-issues-chapter /-->',
				'<!-- wp:dailyos/unified-timeline /-->',
				'<!-- wp:dailyos/finis-marker /-->',
				'<!-- /wp:dailyos/account-detail -->',
			]
		),
	]
);
