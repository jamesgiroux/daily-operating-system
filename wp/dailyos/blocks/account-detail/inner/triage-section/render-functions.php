<?php
/**
 * Triage Section (triage-section) — W2 L1 inner block render-functions.
 *
 * Projection rule: Health.factors (hasTriageContent derived).
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

if ( ! function_exists( 'dailyos_triage_section_render' ) ) {
	/**
	 * Render the triage-section inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content    Inner content (empty).
	 * @param \WP_Block|null       $block      Parsed block carrying usesContext.
	 * @return string
	 */
	function dailyos_triage_section_render( array $attributes, string $content = '', $block = null ): string {
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
		$projected_sections = [ 'facts' ];
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
		$out  = '<section id="needs-attention" class="entity-detail_chapterSection" data-ds-tier="pattern" data-ds-name="TriageSection" data-ds-spec="patterns/TriageSection.md">';
		$out .= '<section class="health_blockHeader" data-dailyos-projection="triage-section" data-dailyos-envelope-sections="' . esc_attr( implode( ',', [ 'facts' ] ) ) . '">';
		$out .= '<hr class="health_blockHeaderRule" />';
		$out .= '<div class="health_blockHeaderTitleRow">';
		$out .= '<h2 class="health_blockHeaderTitle">' . esc_html__( 'Needs attention', 'dailyos' ) . '</h2>';
		$out .= '<span class="health_blockHeaderCount health_blockHeaderCountTerracotta">' . esc_html__( 'Account intelligence', 'dailyos' ) . '</span>';
		$out .= '</div>';
		$out .= '</section>';
		if ( ! $any_present ) {
			$reason = '' !== $first_empty_reason ? $first_empty_reason : 'no_triage_content';
			$out .= dailyos_empty_chip(
				$reason,
				__( 'No triage signals on file', 'dailyos' ),
				'health_triageCard'
			);
			$out .= '</section>';
			return $out;
		}

		$out .= '<div>';
		// Per AC-462.3 + DOS-341: every claim-bearing inner block routes
		// receipts through build_receipt_for_audience server-side. The
		// dailyos_envelope_consume_claim helper invokes claim_receipt via the
		// runtime client with the resolved scope set; AgentMcp audience filter
		// is applied inside the producer (DOS-341 boundary).
		$projected_claim_refs = dailyos_triage_section_select_claim_refs( $envelope );
		$rows = '';
		foreach ( $projected_claim_refs as $claim_ref ) {
			$receipt = dailyos_envelope_consume_claim( $claim_ref, $scope_set );
			if ( null === $receipt ) {
				continue;
			}
			$rows .= dailyos_triage_section_render_row( $claim_ref, $receipt );
		}
		$out .= '' !== $rows
			? $rows
			: dailyos_empty_chip( 'no_triage_content', __( 'No triage signals on file', 'dailyos' ), 'health_triageCard' );
		$out .= '</div>';
		$out .= '</section>';
		return $out;
	}
}


if ( ! function_exists( 'dailyos_triage_section_select_claim_refs' ) ) {
	/**
	 * Select claim references from the envelope for the triage-section projection.
	 * Pure projection — does not invoke any abilities; receipts fan out in
	 * the renderer via dailyos_envelope_consume_claim().
	 *
	 * Projection rule: Health.factors (hasTriageContent derived).
	 *
	 * @param array<string,mixed>|null $envelope Envelope payload.
	 * @return array<int,array<string,mixed>>
	 */
	function dailyos_triage_section_select_claim_refs( ?array $envelope ): array {
		return dailyos_envelope_collect_claim_refs(
			$envelope,
			[ 'facts' ],
			[
				'claim_types'         => [ 'entity_risk' ],
				'field_path_prefixes' => [ 'risks[', 'blockers[' ],
			]
		);
	}
}

if ( ! function_exists( 'dailyos_triage_section_render_row' ) ) {
	/**
	 * Render a single claim row inside the triage-section projection.
	 * Receipt was already resolved server-side through claim_receipt; this
	 * function shapes the typed display (trust-band, sensitivity, freshness).
	 *
	 * @param array<string,mixed> $claim_ref The claim_ref passed to the consumer.
	 * @param array<string,mixed> $receipt   Receipt payload returned by claim_receipt.
	 * @return string
	 */
	function dailyos_triage_section_render_row( array $claim_ref, array $receipt ): string {
		$claim_id = isset( $claim_ref['claim_id'] ) ? (string) $claim_ref['claim_id'] : '';
		$trust_band = dailyos_receipt_trust_band( $receipt );
		return '<article class="health_triageCard" data-claim-id="' . esc_attr( $claim_id ) . '" data-trust-band="' . esc_attr( $trust_band ) . '">'
			. '<div class="health_triageSpine health_spineUrgent" aria-hidden="true"></div>'
			. '<div class="health_triageBody">'
			. '<span class="health_triageKind health_kindUrgent">' . esc_html__( 'Account signal', 'dailyos' ) . '</span>'
			. '<div class="health_triageHeadline">' . esc_html( dailyos_receipt_rendered_text( $receipt, $claim_id ) ) . '</div>'
			. '<div class="health_triageSources"><span class="health_triageCitationPlain">' . esc_html( $trust_band ) . '</span></div>'
			. '</div>'
			. '</article>';
	}
}
