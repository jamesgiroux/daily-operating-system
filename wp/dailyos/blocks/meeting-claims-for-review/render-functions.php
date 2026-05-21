<?php
/**
 * MeetingClaimsForReview inner-block server-side render (W2 §5.4 / DOS-752).
 *
 * Projection: MetadataProposals section (meeting-bound proposals,\n * DOS-328). Per-proposal trust band; claim-fanout via outer block's\n * `dailyos_meeting_detail_claim_inner_consumer` hook for\n * claim_receipt + record_claim_feedback wiring.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_claims_for_review_render' ) ) {
	function dailyos_meeting_claims_for_review_render( array $attributes, $block = null ): string {
		$meeting_id = '';
		if ( is_object( $block ) && isset( $block->context ) && is_array( $block->context ) ) {
			$meeting_id = isset( $block->context['dailyos/entityId'] )
				? (string) $block->context['dailyos/entityId']
				: '';
		}

		if ( '' === $meeting_id ) {
			return '<span class="dailyos-empty-chip" data-empty-reason="missing_meeting_context">'
				. esc_html__( 'No meeting context.', 'dailyos' )
				. '</span>';
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return '<span class="dailyos-empty-chip" data-empty-reason="runtime_unavailable">'
				. esc_html__( 'Claims for review unavailable.', 'dailyos' )
				. '</span>';
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		// W1 producer: get_entity_intelligence (entity_type=meeting).
		$response = $runtime_client->invoke_ability(
			'get_entity_intelligence',
			[
				'entity_type' => 'meeting',
				'entity_id'   => $meeting_id,
			],
			$scope_set
		);

		if ( is_wp_error( $response ) ) {
			return '<span class="dailyos-empty-chip" data-empty-reason="envelope_error">'
				. esc_html__( 'Claims for review unavailable.', 'dailyos' )
				. '</span>';
		}

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'        => 'wp-block-dailyos-meeting-claims-for-review',
					'data-ds-tier' => 'primitive',
					'data-ds-name' => 'MeetingClaimsForReview',
				]
			)
			: 'class="wp-block-dailyos-meeting-claims-for-review"';

		return '<section ' . $wrapper_attrs . ' data-empty-reason="">'
			. '<h2 class="wp-block-dailyos-meeting-claims-for-review__title">'
			. esc_html__( 'Claims for review', 'dailyos' )
			. '</h2>'
			. '</section>';
	}
}
