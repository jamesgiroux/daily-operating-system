<?php
/**
 * Metadata Proposal Cue dynamic block render entrypoint (W2 L1 — DOS-328 / §5.6).
 *
 * Inner block surfaced inside entity-detail outer blocks via
 * `usesContext: ["dailyos/envelopeHandle"]`. Reads the
 * `EntityIntelligenceEnvelope.metadata_proposals` slice and renders a
 * calm peripheral cue when unresolved proposals exist. The expanded
 * accept/dismiss/edit affordance lives in `dailyos/metadata-proposal-drawer`.
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

if ( ! function_exists( 'dailyos_metadata_proposal_cue_render' ) ) {
	require_once __DIR__ . '/render-functions.php';
}

$attributes = isset( $attributes ) && is_array( $attributes ) ? $attributes : [];
$content    = isset( $content ) && is_string( $content ) ? $content : '';
$block      = isset( $block ) && is_object( $block ) ? $block : null;
// phpcs:ignore WordPress.Security.EscapeOutput.OutputNotEscaped -- render functions escape output internally via esc_html / esc_attr.
echo dailyos_metadata_proposal_cue_render( $attributes, $content, $block );
