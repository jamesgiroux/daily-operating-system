<?php
/**
 * Divergence Section (divergence-section) — W2 L1 inner block render-functions.
 *
 * Projection rule: Health.factors (hasDivergenceContent derived).
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

if ( ! function_exists( 'dailyos_divergence_section_render' ) ) {
	/**
	 * Render the divergence-section inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content    Inner content (empty).
	 * @param \WP_Block|null       $block      Parsed block carrying usesContext.
	 * @return string
	 */
	function dailyos_divergence_section_render( array $attributes, string $content = '', $block = null ): string {
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

		$envelope = dailyos_resolve_envelope( $handle, 'account', $entity_id, $scope_set );
		$projected_sections = ['health'];
		$any_present = false;
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
			$reason = '' !== $first_empty_reason ? $first_empty_reason : 'no_divergence_content';
			return dailyos_empty_chip(
				$reason,
				__( 'No divergence signals on file', 'dailyos' ),
				'wp-block-dailyos-divergence-section'
			);
		}

		$wrapper_attrs = dailyos_inner_block_wrapper_attrs( 'wp-block-dailyos-divergence-section' );
		$out  = '<div ' . $wrapper_attrs . ' data-dailyos-projection="divergence-section">';
		$out .= '<header class="wp-block-dailyos-divergence-section__header"><span class="wp-block-dailyos-divergence-section__title">' . esc_html__( 'Divergence Section', 'dailyos' ) . '</span></header>';
		$out .= '<div class="wp-block-dailyos-divergence-section__body" data-dailyos-envelope-sections="' . esc_attr( implode( ',', ['health'] ) ) . '">';
		// Per AC-462.3 + DOS-341: every claim-bearing inner block routes
		// receipts through build_receipt_for_audience server-side. The
		// dailyos_envelope_consume_claim helper invokes claim_receipt via the
		// runtime client with the resolved scope set; AgentMcp audience filter
		// is applied inside the producer (DOS-341 boundary).
		$projected_claim_refs = dailyos_divergence_section_select_claim_refs( $envelope );
		foreach ( $projected_claim_refs as $claim_ref ) {
			$receipt = dailyos_envelope_consume_claim( $claim_ref, $scope_set );
			if ( null === $receipt ) {
				continue;
			}
			$out .= dailyos_divergence_section_render_row( $claim_ref, $receipt );
		}
		$out .= '</div>';
		$out .= '</div>';
		return $out;
	}
}


if ( ! function_exists( 'dailyos_divergence_section_select_claim_refs' ) ) {
	/**
	 * Select claim references from the envelope for the divergence-section projection.
	 * Pure projection — does not invoke any abilities; receipts fan out in
	 * the renderer via dailyos_envelope_consume_claim().
	 *
	 * Projection rule: Health.factors (hasDivergenceContent derived).
	 *
	 * @param array<string,mixed>|null $envelope Envelope payload.
	 * @return array<int,array<string,mixed>>
	 */
	function dailyos_divergence_section_select_claim_refs( ?array $envelope ): array {
		if ( null === $envelope ) {
			return [];
		}
		$refs = [];
		// Walk the projected envelope sections and collect claim_ids. The
		// exact projection rule lives in the L0-packet projection table; this
		// helper is a single source for the slug's selection so test fixtures
		// can target one function rather than the renderer.
		foreach ( ['health'] as $section_key ) {
			$slice = $envelope[ $section_key ] ?? [];
			if ( ! is_array( $slice ) ) {
				continue;
			}
			$items = $slice['items'] ?? ( is_array( reset( $slice ) ) ? $slice : [] );
			if ( ! is_array( $items ) ) {
				continue;
			}
			foreach ( $items as $item ) {
				if ( ! is_array( $item ) ) {
					continue;
				}
				$claim_id = $item['claimId'] ?? $item['claim_id'] ?? '';
				if ( '' === $claim_id ) {
					continue;
				}
				$refs[] = [
					'claim_id'     => (string) $claim_id,
					'audience_key' => $item['audienceKey'] ?? 'user',
					'subject_ref'  => $item['subjectRef'] ?? null,
					'field_path'   => $item['fieldPath'] ?? null,
				];
			}
		}
		return $refs;
	}
}

if ( ! function_exists( 'dailyos_divergence_section_render_row' ) ) {
	/**
	 * Render a single claim row inside the divergence-section projection.
	 * Receipt was already resolved server-side through claim_receipt; this
	 * function shapes the typed display (trust-band, sensitivity, freshness).
	 *
	 * @param array<string,mixed> $claim_ref The claim_ref passed to the consumer.
	 * @param array<string,mixed> $receipt   Receipt payload returned by claim_receipt.
	 * @return string
	 */
	function dailyos_divergence_section_render_row( array $claim_ref, array $receipt ): string {
		$claim_id = isset( $claim_ref['claim_id'] ) ? (string) $claim_ref['claim_id'] : '';
		$trust_band = isset( $receipt['trustBand'] ) ? (string) $receipt['trustBand'] : ( isset( $receipt['trust_band'] ) ? (string) $receipt['trust_band'] : 'unscored' );
		return '<article class="wp-block-dailyos-divergence-section__row" data-claim-id="' . esc_attr( $claim_id ) . '" data-trust-band="' . esc_attr( $trust_band ) . '">'
			. '<span class="wp-block-dailyos-divergence-section__row-label">' . esc_html( $claim_id ) . '</span>'
			. '</article>';
	}
}
