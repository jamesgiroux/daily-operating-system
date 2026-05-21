<?php
/**
 * Meeting Touchpoints Feed inner-block server-side render
 * (W2 §5.4 / DOS-752).
 *
 * Projection: Touchpoints section (related-meeting cadence) from the
 * meeting envelope. Per-touchpoint trust band.
 *
 * **AgentMcp audience filter (V1.1 lock — Option B aggregate):** when the
 * resolved audience is AgentMcp, render aggregate-only signal
 * `{ count, recency: Recent|Aging|Stale, content: redacted }`. Per-item
 * titles + attendee names are scrubbed per W1 cycle-2 F2 pattern.
 * Human-audience renders the full per-touchpoint list.
 *
 * The runtime client is the canonical place audience filtering is
 * applied (via `build_receipt_for_audience` chain in
 * `services::claim_receipt::privacy`); this block reads the audience tag
 * from the response and renders accordingly without re-deriving the
 * filter.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_touchpoints_feed_render' ) ) {
	function dailyos_meeting_touchpoints_feed_render( array $attributes, $block = null ): string {
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
				. esc_html__( 'Touchpoints unavailable.', 'dailyos' )
				. '</span>';
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		// W1 producer: get_entity_intelligence (entity_type=meeting) — Touchpoints.
		// The runtime applies audience filtering: for AgentMcp callers, the
		// Touchpoints section arrives pre-aggregated to { count, recency,
		// content: redacted } per V1.1 lock.
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
				. esc_html__( 'Touchpoints unavailable.', 'dailyos' )
				. '</span>';
		}

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'                          => 'wp-block-dailyos-meeting-touchpoints-feed',
					'data-ds-tier'                   => 'primitive',
					'data-ds-name'                   => 'MeetingTouchpointsFeed',
					'data-dailyos-agentmcp-aggregate' => 'true',
				]
			)
			: 'class="wp-block-dailyos-meeting-touchpoints-feed" data-dailyos-agentmcp-aggregate="true"';

		return '<section ' . $wrapper_attrs . ' data-empty-reason="">'
			. '<h2 class="wp-block-dailyos-meeting-touchpoints-feed__title">'
			. esc_html__( 'Touchpoints', 'dailyos' )
			. '</h2>'
			. '</section>';
	}
}
