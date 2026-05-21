<?php
/**
 * Metadata Proposal Drawer dynamic block render entrypoint (W2 L1 — DOS-328 / §5.6).
 *
 * Inner block surfaced inside entity-detail outer blocks via
 * `usesContext: ["dailyos/envelopeHandle"]`. Renders the expanded drawer
 * with accept / dismiss / edit affordances; each affordance emits a typed
 * FeedbackAction via record_claim_feedback (ADR-0123).
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes from core.
 * @var string               $content    Inner content (empty for dynamic blocks).
 * @var \WP_Block|null       $block      Parsed block (carries usesContext).
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_metadata_proposal_drawer_render' ) ) {
	require_once __DIR__ . '/render-functions.php';
}

$attributes = isset( $attributes ) && is_array( $attributes ) ? $attributes : [];
$content    = isset( $content ) && is_string( $content ) ? $content : '';
$block      = isset( $block ) && is_object( $block ) ? $block : null;
// phpcs:ignore WordPress.Security.EscapeOutput.OutputNotEscaped -- render functions escape output internally via esc_html / esc_attr.
echo dailyos_metadata_proposal_drawer_render( $attributes, $content, $block );
