<?php
/**
 * Meeting Post-Meeting Capture inner-block server-side render
 * (W2 §5.4 / DOS-752 / AC-MD.3).
 *
 * Projection: Record section (transcript / notes) from the meeting
 * envelope. Read path consumes `get_entity_intelligence`
 * (entity_type=meeting).
 *
 * Write path goes through `process_paste_transcript` ability per AC-MD.3 —
 * NO direct DB writes from PHP/JS. The block emits a paste affordance
 * that hands the transcript text to the ability through the runtime
 * client; the ability owns claim emission and any downstream invalidation.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_post_meeting_capture_render' ) ) {
	function dailyos_meeting_post_meeting_capture_render( array $attributes, $block = null ): string {
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
				. esc_html__( 'Post-meeting capture unavailable.', 'dailyos' )
				. '</span>';
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		// Read path: get_entity_intelligence (entity_type=meeting) — Record section.
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
				. esc_html__( 'Post-meeting capture unavailable.', 'dailyos' )
				. '</span>';
		}

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'                       => 'wp-block-dailyos-meeting-post-meeting-capture',
					'data-ds-tier'                => 'primitive',
					'data-ds-name'                => 'MeetingPostMeetingCapture',
					'data-dailyos-write-ability'  => 'process_paste_transcript',
				]
			)
			: 'class="wp-block-dailyos-meeting-post-meeting-capture" data-dailyos-write-ability="process_paste_transcript"';

		return '<section ' . $wrapper_attrs . ' data-empty-reason="">'
			. '<h2 class="wp-block-dailyos-meeting-post-meeting-capture__title">'
			. esc_html__( 'Post-meeting capture', 'dailyos' )
			. '</h2>'
			. '</section>';
	}
}
