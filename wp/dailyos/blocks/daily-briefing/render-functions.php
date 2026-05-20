<?php
/**
 * Daily briefing outer-block server-side render (W2 sub-L0 SKELETON).
 *
 * W2 consumer-skeleton wiring per AC-W1.2 / AC-W1.9: the outer
 * daily-briefing block invokes the W1 producer `get_daily_briefing` via
 * the paired DailyOS runtime, then emits the outer wrapper and an
 * <InnerBlocks /> placeholder so inner blocks can project named slices
 * of the composed BriefingState envelope (DOS-507 / V1.1 §13).
 * Full inner shape lands in W2 sub-L0; this stub is the lint-anchor for
 * the consumer-skeleton CI gate.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes (provided by core).
 * @var string               $content    Inner content rendered upstream by core.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_daily_briefing_render' ) ) {
	/**
	 * Render the daily-briefing outer block. Invokes `get_daily_briefing`
	 * via the runtime client (no direct DB reads from PHP), then emits the
	 * outer wrapper + inner-blocks slot.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content    Pre-rendered inner-block content from core.
	 * @return string Rendered HTML.
	 */
	function dailyos_daily_briefing_render( array $attributes, string $content = '' ): string {
		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return '<div class="wp-block-dailyos-daily-briefing is-unavailable">'
				. esc_html__( 'Runtime unavailable.', 'dailyos' )
				. '</div>';
		}

		// W1 producer: get_daily_briefing (DOS-507 BriefingState envelope).
		$args = [];
		if ( isset( $attributes['date'] ) && is_string( $attributes['date'] ) && '' !== $attributes['date'] ) {
			$args['date'] = (string) $attributes['date'];
		}
		$response = $runtime_client->invoke_ability( 'get_daily_briefing', $args );

		if ( is_wp_error( $response ) ) {
			return '<div class="wp-block-dailyos-daily-briefing is-unavailable">'
				. esc_html__( 'Runtime unavailable.', 'dailyos' )
				. '</div>';
		}

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'                => 'wp-block-dailyos-daily-briefing',
					'data-ds-tier'         => 'pattern',
					'data-ds-name'         => 'DailyBriefing',
					'data-dailyos-surface' => 'daily_briefing',
				]
			)
			: 'class="wp-block-dailyos-daily-briefing" data-dailyos-surface="daily_briefing"';

		$out  = '<section ' . $wrapper_attrs . '>';
		// Inner-blocks slot: W2 sub-L0 lands meeting-prep inner blocks that
		// invoke meeting_prep_status via the consumer hook below.
		$out .= '<div class="dailyos-inner-blocks-slot">' . $content . '</div>';
		$out .= '</section>';

		return $out;
	}

	/**
	 * Inner-block consumer hook (W2 sub-L0): meeting-prep inner blocks call
	 * `meeting_prep_status` through this hook so the outer block remains the
	 * single wiring authority. Defined here for the consumer-skeleton lint
	 * anchor; body lands in W2 sub-L0.
	 *
	 * @param string $meeting_id Meeting identifier.
	 * @return array<string, mixed> Prep status envelope.
	 */
	function dailyos_daily_briefing_meeting_prep_inner_consumer( string $meeting_id ): array {
		// W2 sub-L0: invoke meeting_prep_status state machine.
		unset( $meeting_id );
		return [];
	}
}
