<?php
/**
 * Trajectory Chapter (trajectory-chapter) — W2 L1 inner block render-functions (DOS-483).
 *
 * Projection rule: health (envelope sections projected by this chapter).
 * Trust band source: aggregate.
 *
 * Per L0-packet-W2-entity-surfaces.md V1.2.1 §5.2 + wave-plan §10
 * invariant "empty-state pattern": every inner block renders a quiet
 * `dailyos-empty-chip` with `data-empty-reason` on absent projection —
 * NEVER silent-hidden. Claim-bearing rows route through
 * `build_receipt_for_audience` (DOS-341 AgentMcp audience filter) via the
 * outer block's `dailyos_project_detail_claim_inner_read` helper.
 *
 * Inner blocks declare `usesContext` for `dailyos/envelopeHandle` and
 * consume the cached envelope via `dailyos_resolve_envelope()`, which
 * short-circuits to the outer block's single producer invocation.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_resolve_envelope' ) ) {
	require_once dirname( __DIR__, 3 ) . '/_shared/envelope/envelope-resolver.php';
}

if ( ! function_exists( 'dailyos_trajectory_chapter_render' ) ) {
	/**
	 * Render the trajectory-chapter inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content    Inner content (empty).
	 * @param \WP_Block|null       $block      Parsed block carrying usesContext.
	 * @return string
	 */
	function dailyos_trajectory_chapter_render( array $attributes, string $content = '', $block = null ): string {
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

		$envelope = dailyos_resolve_envelope( $handle, 'project', $entity_id, $scope_set );

		$projected_sections = [ 'health' ];
		$any_present        = false;
		$first_empty_reason = '';
		foreach ( $projected_sections as $section_key ) {
			$state = dailyos_envelope_section( $envelope, $section_key );
			if ( $state['present'] && $state['item_count'] > 0 ) {
				$any_present = true;
				break;
			}
			if ( '' === $first_empty_reason && '' !== $state['reason'] ) {
				$first_empty_reason = $state['reason'];
			}
		}

		if ( ! $any_present ) {
			$reason = '' !== $first_empty_reason ? $first_empty_reason : 'no_trajectory_read';
			if ( function_exists( 'dailyos_empty_chip' ) ) {
				return dailyos_empty_chip(
					$reason,
					__( 'No trajectory read yet', 'dailyos' ),
					'wp-block-dailyos-trajectory-chapter'
				);
			}
			return '<section class="wp-block-dailyos-trajectory-chapter"><span class="dailyos-empty-chip" data-empty-reason="' . esc_attr( $reason ) . '">' . esc_html__( 'No trajectory read yet', 'dailyos' ) . '</span></section>';
		}

		$wrapper = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'                       => 'wp-block-dailyos-trajectory-chapter',
					'data-ds-tier'                => 'pattern',
					'data-ds-name'                => 'trajectory-chapter',
					'data-dailyos-trust-band-src' => 'aggregate',
				]
			)
			: 'class="wp-block-dailyos-trajectory-chapter" data-dailyos-trust-band-src="aggregate"';

		// Per W2 V1.2.1 §5.2: structural-wiring L1 deliverable. Full typed
		// rendering (per-claim affordances, trust-band chrome, ability-cursor
		// pagination) composes via primitive blocks inside this body in
		// subsequent L1 passes per AC-483.6 visible-QA matrix. The L1 wiring
		// invariants — envelope-handle context, empty-state chip,
		// AgentMcp audience filter (where applicable), claim-consumer
		// delegation — are the deliverable for this commit.
		$out  = '<section ' . $wrapper . '>';
		$out .= '<div class="dailyos-trajectory-chapter-body"></div>';
		$out .= '</section>';
		return $out;
	}
}
