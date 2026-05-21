<?php
/**
 * Projects index list-shell server-side render.
 *
 * Per L0 Packet W2 V1.2.1 §5.5. Mirrors accounts-index / people-index. Client-
 * side pagination runs against the `list_projects` ability, consuming W1's
 * `Paginated<T>` + `CursorState` shape via the shared `useAbilityCursor` hook.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes from core.
 * @var string               $content    Inner content (unused; list shell has no inner blocks).
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

$page_size = isset( $attributes['page_size'] ) ? (int) $attributes['page_size'] : 25;
if ( $page_size < 1 || $page_size > 200 ) {
	$page_size = 25;
}
$watermark = isset( $attributes['watermark'] ) ? (string) $attributes['watermark'] : '';

$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
	? get_block_wrapper_attributes(
		array(
			'class'                  => 'wp-block-dailyos-projects-index',
			'data-ds-tier'           => 'pattern',
			'data-ds-name'           => 'ProjectsIndex',
			'data-dailyos-surface'   => 'projects_index',
			'data-dailyos-list-of'   => 'project',
			'data-dailyos-ability'   => 'list_projects',
			'data-dailyos-page-size' => (string) $page_size,
			'data-dailyos-watermark' => $watermark,
		)
	)
	: 'class="wp-block-dailyos-projects-index" data-dailyos-surface="projects_index" data-dailyos-list-of="project" data-dailyos-ability="list_projects" data-dailyos-page-size="' . (int) $page_size . '" data-dailyos-watermark="' . esc_attr( $watermark ) . '"';

$out  = '<section ' . $wrapper_attrs . '>';
$out .= '<div class="dailyos-projects-index__mount" data-dailyos-projects-index-mount>';
$out .= '<span class="dailyos-empty-chip" data-empty-reason="hydrating" aria-live="polite">'
	. esc_html__( 'Loading projects…', 'dailyos' )
	. '</span>';
$out .= '</div>';
$out .= '</section>';

// phpcs:ignore WordPress.Security.EscapeOutput.OutputNotEscaped -- wrapper attrs + escaped labels above.
echo $out;
