<?php
/**
 * MeetingAttendeesSection inner-block server-side render (W2 §5.4 / DOS-752).
 *
 * Projection: Facts section (attendees array). Per-attendee trust band\n * sourced from each attendee's claim.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_attendees_section_render' ) ) {
	function dailyos_meeting_attendees_section_render( array $attributes, $block = null ): string {
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
				. esc_html__( 'Attendees unavailable.', 'dailyos' )
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
				. esc_html__( 'Attendees unavailable.', 'dailyos' )
				. '</span>';
		}

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'        => 'wp-block-dailyos-meeting-attendees-section',
					'data-ds-tier' => 'primitive',
					'data-ds-name' => 'MeetingAttendeesSection',
				]
			)
			: 'class="wp-block-dailyos-meeting-attendees-section"';

		return '<section ' . $wrapper_attrs . ' data-empty-reason="">'
			. '<h2 class="wp-block-dailyos-meeting-attendees-section__title">'
			. esc_html__( 'Attendees', 'dailyos' )
			. '</h2>'
			. '</section>';
	}
}
