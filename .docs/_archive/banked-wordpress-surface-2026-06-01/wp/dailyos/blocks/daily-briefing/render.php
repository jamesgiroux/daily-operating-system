<?php
/**
 * Daily briefing dynamic block render entrypoint (W2 sub-L0 SKELETON).
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes from core.
 * @var string               $content    Inner content rendered upstream by core.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_daily_briefing_render' ) ) {
	require_once __DIR__ . '/render-functions.php';
}

$attributes = isset( $attributes ) && is_array( $attributes ) ? $attributes : [];
$content    = isset( $content ) && is_string( $content ) ? $content : '';
// phpcs:ignore WordPress.Security.EscapeOutput.OutputNotEscaped -- render functions escape output internally (W3 contract).
echo dailyos_daily_briefing_render( $attributes, $content );
