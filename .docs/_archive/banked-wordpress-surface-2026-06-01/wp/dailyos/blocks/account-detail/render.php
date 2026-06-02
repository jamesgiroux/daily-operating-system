<?php
/**
 * Account detail dynamic block render entrypoint (W2 sub-L0 SKELETON).
 *
 * Delegates to render-functions.php so the same function services both
 * the block-registration render path and the editor preview REST route.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes from core.
 * @var string               $content    Inner content rendered upstream by core.
 * @var \WP_Block|null       $block      Parsed block instance from core.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_account_detail_render' ) ) {
	require_once __DIR__ . '/render-functions.php';
}

$attributes = isset( $attributes ) && is_array( $attributes ) ? $attributes : [];
$content    = isset( $content ) && is_string( $content ) ? $content : '';
$block      = isset( $block ) ? $block : null;
// phpcs:ignore WordPress.Security.EscapeOutput.OutputNotEscaped -- render functions escape output internally via esc_html / esc_attr (W3 contract).
echo dailyos_account_detail_render( $attributes, $content, $block );
