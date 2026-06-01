<?php
/**
 * Meeting Header inner block render entrypoint (W2 §5.4 / DOS-752).
 *
 * Delegates to render-functions.php. Projects Facts section (title, time,
 * organizer) from the meeting envelope produced by outer block.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes from core.
 * @var string               $content    Inner content (none — leaf inner block).
 * @var WP_Block             $block      Block instance (provides context).
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_header_render' ) ) {
	require_once __DIR__ . '/render-functions.php';
}

$attributes = isset( $attributes ) && is_array( $attributes ) ? $attributes : [];
$block      = isset( $block ) ? $block : null;
// phpcs:ignore WordPress.Security.EscapeOutput.OutputNotEscaped -- render functions escape output internally.
echo dailyos_meeting_header_render( $attributes, $block );
