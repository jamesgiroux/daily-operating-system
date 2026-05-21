<?php
/**
 * Recommended Actions (recommended-actions) dynamic block render entrypoint (W2 L1 inner block).
 *
 * Delegates to render-functions.php so the same function services both
 * the block-registration render path and the editor preview REST route.
 *
 * @package DailyOS
 *
 * @var array<string, mixed>  $attributes Block attributes from core.
 * @var string                $content    Inner content (empty for dynamic blocks).
 * @var \WP_Block|null        $block      Parsed block (carries usesContext).
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_recommended_actions_render' ) ) {
	require_once __DIR__ . '/render-functions.php';
}

$attributes = isset( $attributes ) && is_array( $attributes ) ? $attributes : [];
$content    = isset( $content ) && is_string( $content ) ? $content : '';
$block      = isset( $block ) && is_object( $block ) ? $block : null;
// phpcs:ignore WordPress.Security.EscapeOutput.OutputNotEscaped -- render functions escape output internally.
echo dailyos_recommended_actions_render( $attributes, $content, $block );
