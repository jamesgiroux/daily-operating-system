<?php
/**
 * Meeting Prep Status inner block render entrypoint (W2 §5.4 / DOS-752 / AC-MD.2).
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes from core.
 * @var WP_Block             $block      Block instance (provides context).
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_prep_status_render' ) ) {
	require_once __DIR__ . '/render-functions.php';
}

$attributes = isset( $attributes ) && is_array( $attributes ) ? $attributes : [];
$block      = isset( $block ) ? $block : null;
// phpcs:ignore WordPress.Security.EscapeOutput.OutputNotEscaped -- render functions escape output internally.
echo dailyos_meeting_prep_status_render( $attributes, $block );
