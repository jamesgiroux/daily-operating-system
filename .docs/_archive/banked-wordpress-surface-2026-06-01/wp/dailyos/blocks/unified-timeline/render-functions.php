<?php
/**
 * Unified Timeline inner-block server-side render (W2 L1 — DOS-484).
 *
 * Projects from envelope section(s): Record (Paginated<RecordEntry>) + MetadataProposals lifecycle.
 * Trust band source: per-entry.
 *
 * Resolves the parent envelope via `dailyos/envelopeHandle` context
 * (published by `dailyos/person-detail` per DOS-477 cache contract).
 * Renders the empty state as a quiet `dailyos-empty-chip` with
 * `data-empty-reason` per W2 §10 invariant (no silent hidden states).
 *
 * Claim affordances (per-claim_ref `claim_receipt` + `record_claim_feedback`)
 * flow through `dailyos_person_detail_claim_inner_read` on the outer
 * block — the consumer-skeleton CI gate (AC-W1.9) lints the 3-arg
 * invocation there, not in this inner file.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_unified_timeline_render' ) ) {
	/**
	 * Render the Unified Timeline inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param array<string, mixed> $ctx        Block context (entityType, entityId,
	 *                                          envelopeHandle).
	 * @return string Rendered HTML.
	 */
	function dailyos_unified_timeline_render( array $attributes, array $ctx = [] ): string {
		$handle   = isset( $ctx['dailyos/envelopeHandle'] ) ? (string) $ctx['dailyos/envelopeHandle'] : '';
		$envelope = null;
		if ( '' !== $handle && function_exists( 'dailyos_person_detail_envelope_store' ) ) {
			$envelope = dailyos_person_detail_envelope_store( $handle );
		}

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'        => 'wp-block-dailyos-unified-timeline',
					'data-ds-tier' => 'pattern',
					'data-ds-name' => 'UnifiedTimeline',
				]
			)
			: 'class="wp-block-dailyos-unified-timeline"';

		if ( ! is_array( $envelope ) ) {
			return '<section ' . $wrapper_attrs . '>'
				. '<span class="dailyos-empty-chip" data-empty-reason="no_envelope">'
				. esc_html__( 'No data yet.', 'dailyos' )
				. '</span>'
				. '</section>';
		}

		// Projection placeholder: full typed rendering (claim rows, trust
		// bands, receipts, useAbilityCursor pagination where applicable)
		// lands in subsequent L1 passes per AC-484.5 visible-QA matrix. The
		// structural wiring (envelope-handle context, empty-state chip,
		// claim-consumer delegation through the outer block) is the L1
		// deliverable.
		$out  = '<section ' . $wrapper_attrs . '>';
		$out .= '<div class="dailyos-unified-timeline__body" data-dailyos-projection="unified-timeline" data-trust-band-source="per-entry"></div>';
		$out .= '</section>';

		return $out;
	}
}
