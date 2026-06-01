<?php
/**
 * Person Appendix dynamic block render entrypoint (W2 L1 — DOS-484).
 *
 * Inner block under `dailyos/person-detail`. Projects from envelope
 * section(s): envelope-level EnvelopeProvenance + all section caveats.
 * Trust band source: aggregate.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes from core.
 * @var string               $content    Inner content (unused).
 * @var WP_Block|null        $block      Block instance carrying parent context.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_person_appendix_render' ) ) {
	require_once __DIR__ . '/render-functions.php';
}

$attributes = isset( $attributes ) && is_array( $attributes ) ? $attributes : [];
$ctx        = [];
if ( isset( $block ) && is_object( $block ) && isset( $block->context ) && is_array( $block->context ) ) {
	$ctx = $block->context;
}
// phpcs:ignore WordPress.Security.EscapeOutput.OutputNotEscaped -- render functions escape output internally (W3 contract).
echo dailyos_person_appendix_render( $attributes, $ctx );
