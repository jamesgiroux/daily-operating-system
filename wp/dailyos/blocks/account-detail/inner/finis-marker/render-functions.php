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
		unset( $attributes, $content );

		$handle    = null;
		$entity_id = '';
		if ( null !== $block && isset( $block->context ) && is_array( $block->context ) ) {
			$handle    = isset( $block->context['dailyos/envelopeHandle'] ) ? (string) $block->context['dailyos/envelopeHandle'] : null;
			$entity_id = isset( $block->context['dailyos/entityId'] ) ? (string) $block->context['dailyos/entityId'] : '';
		}
		if ( ( null === $handle || '' === $handle ) && isset( $GLOBALS['dailyos_envelope_handle_for_request'] ) ) {
			$handle = (string) $GLOBALS['dailyos_envelope_handle_for_request'];
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		$mark = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 433 407" width="18" height="18" aria-hidden="true"><path d="M159 407 161 292 57 355 0 259 102 204 0 148 57 52 161 115 159 0H273L271 115L375 52L433 148L331 204L433 259L375 355L271 292L273 407Z" fill="currentColor"/></svg>';
		$out  = '<div class="editorial-reveal" data-ds-tier="pattern" data-ds-name="FinisMarker" data-ds-spec="patterns/FinisMarker.md" data-dailyos-projection="chrome">';
		$out .= '<div class="FinisMarker_root">';
		$out .= '<div class="FinisMarker_marks">';
		$out .= $mark . $mark . $mark;
		$out .= '</div>';
		$out .= '<div class="FinisMarker_timestamp">' . esc_html__( 'End of dossier', 'dailyos' ) . '</div>';
		$out .= '</div>';
		$out .= '</div>';
		return $out;
	}
}
