<?php
/**
 * Default Metadata Proposal composition (filesystem pattern).
 *
 * Per L0-packet-W2-entity-surfaces.md V1.2.1 §5.6 + AC-328.1: filesystem
 * pattern (theme-registered via register_block_pattern), insert-then-detach
 * semantics — NOT a DB-stored synced pattern. Bundles the calm peripheral
 * cue plus the expanded accept/dismiss/edit drawer as a single insertion
 * unit so an empty-context insert renders meaningfully on Account /
 * Project / Person / Meeting Detail composites.
 *
 * Per §5.6: cue is field-adjacent (multiple instances permitted, one per
 * field); drawer is composite-singleton. This pattern's default emits one
 * cue + one drawer; users can drag additional cue instances onto specific
 * fields in the Site Editor.
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
	'dailyos/metadata-proposal-default',
	[
		'title'       => __( 'Metadata proposals — default composition', 'dailyos' ),
		'description' => __( 'Default Metadata Proposals composition (DOS-328 / W2 §5.6). Peripheral cue plus accept/dismiss/edit drawer. Inserts into any entity-detail outer block; consumes envelope via usesContext envelopeHandle.', 'dailyos' ),
		'categories'  => [ 'dailyos' ],
		'blockTypes'  => [
			'dailyos/account-detail',
			'dailyos/project-detail',
			'dailyos/person-detail',
			'dailyos/meeting-detail',
		],
		'content'     => implode(
			"\n",
			[
				'<!-- wp:dailyos/metadata-proposal-cue /-->',
				'<!-- wp:dailyos/metadata-proposal-drawer /-->',
			]
		),
	]
);
