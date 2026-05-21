<?php
/**
 * Meeting Prep Status inner-block server-side render (W2 §5.4 / DOS-752 / AC-MD.2).
 *
 * Projection: Health section (composed from `MeetingPrepStatus`) + direct
 * DOS-335 prep DTO read. Two-call composition per V1.2.1 §5.4 — the
 * envelope's Health composes with the DOS-335 read; both consumed via
 * the runtime client (no direct DB reads from PHP).
 *
 * Trust band source: `Health.aggregate_band`. Surfaces FolioBar readiness
 * signal via chrome.js (chrome lane primitive, not a W2-novel push).
 *
 * Writes go through `meeting_prep_status::write` via the ability runtime
 * per W1 architecture F2 split — never directly from this block.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_prep_status_render' ) ) {
	function dailyos_meeting_prep_status_render( array $attributes, $block = null ): string {
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
				. esc_html__( 'Prep status unavailable.', 'dailyos' )
				. '</span>';
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		// W1 producer: meeting_prep_status (DOS-335) direct read for prep DTO.
		$prep_response = $runtime_client->invoke_ability(
			'meeting_prep_status',
			[
				'meeting_id' => $meeting_id,
			],
			$scope_set
		);

		if ( is_wp_error( $prep_response ) ) {
			return '<span class="dailyos-empty-chip" data-empty-reason="prep_status_error">'
				. esc_html__( 'Prep status unavailable.', 'dailyos' )
				. '</span>';
		}

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'        => 'wp-block-dailyos-meeting-prep-status',
					'data-ds-tier' => 'primitive',
					'data-ds-name' => 'MeetingPrepStatus',
				]
			)
			: 'class="wp-block-dailyos-meeting-prep-status"';

		return '<section ' . $wrapper_attrs . ' data-empty-reason="">'
			. '<h2 class="dailyos-meeting-prep-status__title">'
			. esc_html__( 'Prep readiness', 'dailyos' )
			. '</h2>'
			. '</section>';
	}
}
