<?php
/**
 * Finis Marker (finis-marker) — W2 L1 inner block render-functions.
 *
 * Projection rule: Chrome only — no envelope binding.
 *
 * Per L0-packet-W2-entity-surfaces.md V1.2.1 §5.1 + wave-plan §10 invariant
 * "empty-state pattern": every inner block renders a quiet
 * dailyos-empty-chip with data-empty-reason on absent projection — NEVER
 * silent-hidden. Claim-bearing rows route through build_receipt_for_audience
 * (DOS-341 AgentMcp audience filter) via dailyos_envelope_consume_claim().
 *
 * Inner blocks declare usesContext for dailyos/envelopeHandle and consume
 * the cached envelope via dailyos_resolve_envelope(), which short-circuits
 * to the outer block's single producer invocation.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes (provided by core).
 * @var string               $content    Inner content (empty for dynamic blocks).
 * @var \WP_Block|null       $block      Parsed block (carries usesContext).
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_resolve_envelope' ) ) {
	require_once dirname( __DIR__, 3 ) . '/_shared/envelope/envelope-resolver.php';
}

if ( ! function_exists( 'dailyos_finis_marker_render' ) ) {
	/**
	 * Render the finis-marker inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content    Inner content (empty).
	 * @param \WP_Block|null       $block      Parsed block carrying usesContext.
	 * @return string
	 */
	function dailyos_finis_marker_render( array $attributes, string $content = '', $block = null ): string {
		// FinisMarker now ships only in the site footer (wp/dailyos/theme/parts/footer.html).
		// This inner block is preserved as a no-op so saved post_content that
		// references `<!-- wp:dailyos/finis-marker /-->` stays valid without
		// rendering a duplicate. Block.json template + default-template
		// fallback have already dropped the entry; this guard catches the
		// existing-post case.
		unset( $attributes, $content, $block );
		return '';
	}
}
