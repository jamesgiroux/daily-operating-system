<?php
/**
 * Meeting Header inner-block server-side render (W2 §5.4 / DOS-752).
 *
 * Projection: Facts section (title, time, organizer) from the meeting
 * envelope produced by `get_entity_intelligence` (entity_type=meeting,
 * W1 substrate at `87df7cf6`). Operational shell — no trust band.
 *
 * Reads `dailyos/entityId` from block context (provided by outer
 * `dailyos/meeting-detail`). For Day-1 the inner block re-invokes
 * `get_entity_intelligence` and the DOS-477 envelope cache de-duplicates
 * the call within a single request.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_header_render' ) ) {
	/**
	 * Render the meeting-header inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param mixed                $block      WP_Block instance (provides context).
	 * @return string Rendered HTML.
	 */
	function dailyos_meeting_header_render( array $attributes, $block = null ): string {
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
				. esc_html__( 'Runtime unavailable.', 'dailyos' )
				. '</span>';
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

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
				. esc_html__( 'Header unavailable.', 'dailyos' )
				. '</span>';
		}

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'        => 'wp-block-dailyos-meeting-header',
					'data-ds-tier' => 'primitive',
					'data-ds-name' => 'MeetingHeader',
				]
			)
			: 'class="wp-block-dailyos-meeting-header"';

		return '<header ' . $wrapper_attrs . ' data-empty-reason="">'
			. '<h1 class="dailyos-meeting-header__title">'
			. esc_html__( 'Meeting', 'dailyos' )
			. '</h1>'
			. '</header>';
	}
}
