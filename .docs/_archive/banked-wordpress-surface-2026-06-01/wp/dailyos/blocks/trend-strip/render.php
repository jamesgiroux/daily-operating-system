<?php
/**
 * TrendStrip primitive dynamic block render entrypoint (DOS-688).
 *
 * Per L0 packet W2 V1.2.1 §5.7 DOS-688:
 *  - Consumes envelope via dailyos/envelopeHandle context (DOS-477 cache).
 *  - Renders a quiet trust-band-aware sparkline. No raw factor values in the
 *    headline (DOS-325 voice rule).
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes from core.
 * @var array<string, mixed> $block      Block instance (carries context).
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_trend_strip_render' ) ) {
	require_once __DIR__ . '/render-functions.php';
}

$attributes = isset( $attributes ) && is_array( $attributes ) ? $attributes : [];
$ctx        = [];
if ( isset( $block ) && is_object( $block ) && isset( $block->context ) && is_array( $block->context ) ) {
	$ctx = $block->context;
}
// phpcs:ignore WordPress.Security.EscapeOutput.OutputNotEscaped -- render function escapes internally.
echo dailyos_trend_strip_render( $attributes, $ctx );
