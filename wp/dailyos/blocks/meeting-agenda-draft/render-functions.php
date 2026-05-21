<?php
/**
 * Meeting Agenda Draft inner-block server-side render (W2 §5.4 / DOS-752).
 *
 * Projection: Facts (agenda narrative) + OpenLoops (agenda-bound action
 * items). Composes 2 sections from the meeting envelope. Per-item trust
 * band sourced from each open-loop / agenda item's `TrustBand`.
 *
 * Claim-fanout: for any agenda item carrying a `claim_ref`, renders the
 * receipt + feedback affordance via the outer block's
 * `dailyos_meeting_detail_claim_inner_consumer` hook (single wiring
 * authority for `claim_receipt` + `record_claim_feedback`).
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_agenda_draft_render' ) ) {
	function dailyos_meeting_agenda_draft_render( array $attributes, $block = null ): string {
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
				. esc_html__( 'Agenda unavailable.', 'dailyos' )
				. '</span>';
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		// W1 producer: get_entity_intelligence (entity_type=meeting) — Facts + OpenLoops.
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
				. esc_html__( 'Agenda unavailable.', 'dailyos' )
				. '</span>';
		}

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'        => 'wp-block-dailyos-meeting-agenda-draft',
					'data-ds-tier' => 'primitive',
					'data-ds-name' => 'MeetingAgendaDraft',
				]
			)
			: 'class="wp-block-dailyos-meeting-agenda-draft"';

		return '<section ' . $wrapper_attrs . ' data-empty-reason="">'
			. '<h2 class="dailyos-meeting-agenda-draft__title">'
			. esc_html__( 'Agenda', 'dailyos' )
			. '</h2>'
			. '</section>';
	}
}
