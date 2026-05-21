<?php
/**
 * Meeting Agenda Draft inner block render entrypoint (W2 §5.4 / DOS-752).
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_agenda_draft_render' ) ) {
	require_once __DIR__ . '/render-functions.php';
}

$attributes = isset( $attributes ) && is_array( $attributes ) ? $attributes : [];
$block      = isset( $block ) ? $block : null;
// phpcs:ignore WordPress.Security.EscapeOutput.OutputNotEscaped -- render functions escape output internally.
echo dailyos_meeting_agenda_draft_render( $attributes, $block );
