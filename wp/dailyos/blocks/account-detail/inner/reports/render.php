<?php
/**
 * Account Reports dynamic block render entrypoint (W2 L1 inner block).
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes
 * @var string               $content
 * @var \WP_Block|null       $block
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_account_detail_reports_render' ) ) {
	require_once __DIR__ . '/render-functions.php';
}

$attributes = isset( $attributes ) && is_array( $attributes ) ? $attributes : [];
$content    = isset( $content ) && is_string( $content ) ? $content : '';
$block      = isset( $block ) && is_object( $block ) ? $block : null;
// phpcs:ignore WordPress.Security.EscapeOutput.OutputNotEscaped
echo dailyos_account_detail_reports_render( $attributes, $content, $block );
