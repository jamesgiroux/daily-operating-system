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
		// Runtime client signature (class-dailyos-runtime-client.php:85) requires
		// (name, payload, scope_set). Scope set resolves from the surface client's
		// granted scopes via the canonical filter (see class-dailyos-plugin.php:1421
		// + class-dailyos-ability-registry.php:185). The runtime authoritatively
		// enforces required scopes; passing the surface-resolved set scopes the
		// invocation to what this paired site has been granted.
		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}
		$response  = $runtime_client->invoke_ability(
			'get_entity_intelligence',
			[
				'entity_type' => 'account',
				'entity_id'   => $account_id,
			],
			$scope_set
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
	 * `claim_receipt` (audience-keyed receipt builder) and
	 * `record_claim_feedback` (feedback affordance) through this hook so the
	 * outer block remains the single wiring authority.
	 *
	 * Cycle-2 fix for codex-challenge F5: previous body unset the input and
	 * returned [] — decorative, not a real consumer. Per F5 patch, the inner
	 * consumer must invoke the W1 producers via the runtime client with the
	 * full 3-arg signature so the consumer-skeleton CI gate has a real signal.
	 * The full receipt-rendering UX (typed inner blocks, audience-keyed
	 * rendering, feedback affordance UI) lands in W2 sub-L0.
	 *
	 * @param array<string, mixed> $claim_ref Claim reference from projection
	 *                                         ({ claim_id, audience_key, ... }).
	 * @return array<string, mixed> Receipt + feedback affordance envelope (raw
	 *                              runtime response; typed shaping in W2).
	 */
	function dailyos_account_detail_claim_inner_consumer( array $claim_ref ): array {
		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return [
				'ok'    => false,
				'error' => [
					'code'    => 'runtime_unavailable',
					'message' => 'DailyOS runtime client not bound for inner consumer.',
				],
			];
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		// W1 producer: claim_receipt — audience-keyed receipt for one claim_ref.
		$receipt_response = $runtime_client->invoke_ability(
			'claim_receipt',
			$claim_ref,
			$scope_set
		);

		// W1 producer: record_claim_feedback — feedback affordance mount for the
		// same claim_ref. Returning the affordance descriptor lets the W2 inner
		// block render the corrected / dismissed / corroborated controls.
		$feedback_response = $runtime_client->invoke_ability(
			'record_claim_feedback',
			$claim_ref,
			$scope_set
		);

		return [
			'receipt'  => $receipt_response,
			'feedback' => $feedback_response,
		];
	}
}
