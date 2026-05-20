<?php
/**
 * Account detail outer-block server-side render (W2 sub-L0 SKELETON).
 *
 * W2 consumer-skeleton wiring per AC-W1.2 / AC-W1.9: the outer
 * account-detail block invokes the W1 producer `get_entity_intelligence`
 * (entity_type=account) via the paired DailyOS runtime, then emits the
 * outer wrapper and an <InnerBlocks /> placeholder so inner blocks can
 * project named slices of the composed envelope (ADR-0130 §4, V1.1 §13
 * "Entity-detail composites are 1 outer + N inner blocks"). Full inner
 * shape lands in W2 sub-L0; this stub is the lint-anchor for the
 * consumer-skeleton CI gate.
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

if ( ! function_exists( 'dailyos_account_detail_render' ) ) {
	/**
	 * Render the account-detail outer block. Invokes `get_entity_intelligence`
	 * via the runtime client (no direct DB reads from PHP), then emits the
	 * outer wrapper + inner-blocks slot. Errors render minimal notices; the
	 * full notice taxonomy is borrowed from account-overview in W2 sub-L0.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content    Pre-rendered inner-block content from core.
	 * @return string Rendered HTML.
	 */
	function dailyos_account_detail_render( array $attributes, string $content = '' ): string {
		$account_id = isset( $attributes['account_id'] ) ? (string) $attributes['account_id'] : '';

		if ( '' === $account_id ) {
			return '<div class="wp-block-dailyos-account-detail is-empty">'
				. esc_html__( 'No account to show here.', 'dailyos' )
				. '</div>';
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return '<div class="wp-block-dailyos-account-detail is-unavailable">'
				. esc_html__( 'Runtime unavailable.', 'dailyos' )
				. '</div>';
		}

		// W1 producer: get_entity_intelligence (entity_type=account).
		$response = $runtime_client->invoke_ability(
			'get_entity_intelligence',
			[
				'entity_type' => 'account',
				'entity_id'   => $account_id,
			]
		);

		if ( is_wp_error( $response ) ) {
			return '<div class="wp-block-dailyos-account-detail is-unavailable">'
				. esc_html__( 'Runtime unavailable.', 'dailyos' )
				. '</div>';
		}

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'                => 'wp-block-dailyos-account-detail',
					'data-ds-tier'         => 'pattern',
					'data-ds-name'         => 'AccountDetail',
					'data-dailyos-surface' => 'account_detail',
				]
			)
			: 'class="wp-block-dailyos-account-detail" data-dailyos-surface="account_detail"';

		$out  = '<section ' . $wrapper_attrs . '>';
		// Inner-blocks slot: W2 sub-L0 lands typed inner blocks. The W1
		// producers consumed via the inner-blocks projection chain are:
		//   - claim_receipt (audience-keyed receipt rows per claim_ref)
		//   - record_claim_feedback (per-claim feedback affordance mount)
		// Until W2 sub-L0 lands typed inner blocks, render the inner-block
		// content emitted by core's block parser.
		$out .= '<div class="dailyos-inner-blocks-slot">' . $content . '</div>';
		$out .= '</section>';

		return $out;
	}

	/**
	 * Inner-block consumer hook (W2 sub-L0): claim-row inner blocks invoke
	 * `claim_receipt` and `record_claim_feedback` through this hook so the
	 * outer block is the wiring authority. Defined here for the consumer-
	 * skeleton lint anchor; bodies land in W2 sub-L0.
	 *
	 * @param array<string, mixed> $claim_ref Claim reference from projection.
	 * @return array<string, mixed> Receipt + feedback affordance shape.
	 */
	function dailyos_account_detail_claim_inner_consumer( array $claim_ref ): array {
		// W2 sub-L0: invoke claim_receipt to build the audience-keyed receipt;
		// pair with record_claim_feedback for the feedback affordance.
		unset( $claim_ref );
		return [];
	}
}
