<?php
/**
 * Source Management dynamic block render entrypoint.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes from core.
 * @var string               $content    Inner block content from core.
 * @var \WP_Block|null       $block      Parsed block instance from core.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_source_management_render' ) ) {
	require_once __DIR__ . '/render-functions.php';
}

$attributes = isset( $attributes ) && is_array( $attributes ) ? $attributes : [];
$content    = isset( $content ) && is_string( $content ) ? $content : '';
$block      = isset( $block ) ? $block : null;

// phpcs:ignore WordPress.Security.EscapeOutput.OutputNotEscaped -- render functions sanitize and escape output internally.
echo dailyos_source_management_render( $attributes, $content, $block );
