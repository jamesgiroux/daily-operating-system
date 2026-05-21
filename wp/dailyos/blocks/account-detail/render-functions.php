<?php
/**
 * Account detail outer-block server-side render (W2 L1 DOS-462).
 *
 * Per L0 packet `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md`
 * V1.2.1 §5.1: the outer dailyos/account-detail block invokes
 * `get_entity_intelligence` (entity_type=account) ONCE per render via the
 * paired DailyOS runtime client (3-arg invoke_ability signature enforced by
 * `src-tauri/scripts/check_w1_consumer_skeleton.sh`), caches the envelope
 * via the request-scoped DOS-477 envelope_cache shim, emits the
 * `dailyos/envelopeHandle` context value, and renders the 24-block
 * `<InnerBlocks />` slot via `do_blocks( $content )`.
 *
 * Inner blocks declare `usesContext: ["dailyos/envelopeHandle"]` and read
 * their slice via `dailyos_resolve_envelope( $handle, 'account', $id, $scope_set )`,
 * which short-circuits to the cached payload. AgentMcp audience filtering
 * is enforced server-side by `build_receipt_for_audience` (DOS-341).
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes (provided by core).
 * @var string               $content    Pre-rendered inner-block content from core.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_resolve_envelope' ) ) {
	require_once dirname( __DIR__ ) . '/_shared/envelope/envelope-resolver.php';
}

if ( ! function_exists( 'dailyos_account_detail_render' ) ) {
	/**
	 * Render the account-detail outer block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content    Pre-rendered inner-block content from core.
	 * @return string Rendered HTML.
	 */
	function dailyos_account_detail_render( array $attributes, string $content = '' ): string {
		$account_id = isset( $attributes['account_id'] ) ? (string) $attributes['account_id'] : '';

		// Auto-fill from post context when attribute is empty AND we're
		// rendering inside the matching CPT. L4 quick-setup path: create a
		// `dailyos_account` post, set the `dailyos_entity_id` post-meta (or
		// fall back to post slug), and the W2 surface composes automatically
		// without editor-side block-attribute wiring.
		if ( '' === $account_id && function_exists( 'get_the_ID' ) && function_exists( 'get_post_type' ) ) {
			$post_id = get_the_ID();
			if ( $post_id && 'dailyos_account' === get_post_type( $post_id ) ) {
				$meta_id = function_exists( 'get_post_meta' )
					? get_post_meta( $post_id, 'dailyos_entity_id', true )
					: '';
				if ( is_string( $meta_id ) && '' !== $meta_id ) {
					$account_id = $meta_id;
				} else {
					$post_obj = function_exists( 'get_post' ) ? get_post( $post_id ) : null;
					if ( $post_obj && is_object( $post_obj ) && isset( $post_obj->post_name ) ) {
						$account_id = (string) $post_obj->post_name;
					}
				}
			}
		}

		if ( '' === $account_id ) {
			return dailyos_empty_chip(
				'no_account_id',
				__( 'No account to show here.', 'dailyos' ),
				'wp-block-dailyos-account-detail'
			);
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! is_callable( [ $runtime_client, 'invoke_ability' ] ) ) {
			return dailyos_empty_chip(
				'runtime_unavailable',
				__( 'Runtime unavailable.', 'dailyos' ),
				'wp-block-dailyos-account-detail'
			);
		}

		// W1 producer: get_entity_intelligence (entity_type=account).
		// Runtime client signature (class-dailyos-runtime-client.php) requires
		// (name, payload, scope_set). The runtime authoritatively enforces
		// required scopes; passing the surface-resolved set scopes the
		// invocation to what this paired site has been granted.
		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}
		$response = $runtime_client->invoke_ability(
			'get_entity_intelligence',
			[
				'entity_type' => 'account',
				'entity_id'   => $account_id,
				'depth'       => 'Full',
				'sections'    => null,
			],
			$scope_set
		);

		if ( is_wp_error( $response ) || ! is_array( $response ) ) {
			return dailyos_empty_chip(
				'runtime_unavailable',
				__( 'Runtime unavailable.', 'dailyos' ),
				'wp-block-dailyos-account-detail'
			);
		}

		// Cache the envelope under its handle; inner blocks pick it up via
		// the dailyos/envelopeHandle context value.
		$handle = dailyos_envelope_handle_from_response( $response, 'account', $account_id );

		// Per §5.1: providesContext writes envelope_handle into block context.
		// Inner blocks resolve via dailyos_resolve_envelope( $handle, ... ).
		// Core block-context plumbing reads provided values from block
		// attributes at parse time; outer renderers communicate the handle
		// downstream by also setting it on a request-scoped global so inner
		// blocks rendered through do_blocks( $content ) can read it even
		// when block-context attribute wiring is editor-only.
		if ( '' !== $handle ) {
			$attributes['envelope_handle'] = $handle;
			$GLOBALS['dailyos_envelope_handle_for_request'] = $handle;
		}

		// Project tint comes through as a CSS custom property per §10
		// invariant (DOS-725) — only --dailyos-* names allowed. No other
		// inline-style values permitted per check_no_inline_style_exception.sh.
		$wrapper_args = [
			'class'                  => 'wp-block-dailyos-account-detail',
			'data-ds-tier'           => 'pattern',
			'data-ds-name'           => 'AccountDetail',
			'data-dailyos-surface'   => 'account_detail',
			'data-dailyos-entity-id' => $account_id,
		];
		// Optional tint from the envelope (subject's chrome). Only emits the
		// allowlisted --dailyos-* custom property; nothing else inline.
		$tint = '';
		$envelope = dailyos_envelope_cache_get( $handle );
		if ( is_array( $envelope ) ) {
			$subject = $envelope['subject'] ?? [];
			if ( is_array( $subject ) ) {
				$candidate = $subject['chromeTint'] ?? $subject['chrome_tint'] ?? '';
				if ( is_string( $candidate ) && '' !== $candidate ) {
					$tint = $candidate;
				}
			}
		}
		if ( '' !== $tint && 1 === preg_match( '/^var\(--dailyos-[a-z-]+\)$/', $tint ) ) {
			$wrapper_args['style'] = '--dailyos-account-tint: ' . $tint;
		}

		// In test envs without `get_block_wrapper_attributes`, flatten the
		// wrapper args ourselves so `data-ds-*`, `data-dailyos-entity-id`,
		// and the allowlisted `--dailyos-account-tint` custom property
		// still reach the rendered output. AccountDetailBlockTest's
		// `test_render_emits_allowlisted_custom_property_tint` exercises
		// this path; without the manual flatten it asserted against a
		// stripped-down "class + data-dailyos-surface" stub.
		if ( function_exists( 'get_block_wrapper_attributes' ) ) {
			$wrapper_attrs = get_block_wrapper_attributes( $wrapper_args );
		} else {
			$pieces = [];
			foreach ( $wrapper_args as $attr_key => $attr_value ) {
				if ( ! is_string( $attr_value ) ) {
					continue;
				}
				$pieces[] = esc_attr( $attr_key ) . '="' . esc_attr( $attr_value ) . '"';
			}
			$wrapper_attrs = implode( ' ', $pieces );
		}

		$out  = '<section ' . $wrapper_attrs . ' data-dailyos-envelope-handle="' . esc_attr( $handle ) . '">';
		// Inner blocks projection: 24 typed inner blocks. core emits
		// $content from the InnerBlocks parse; we route through do_blocks()
		// to ensure dynamic inner blocks re-render with current context.
		$out .= '<div class="dailyos-inner-blocks-slot">';
		$out .= '' !== $content ? do_blocks( $content ) : '';
		$out .= '</div>';
		$out .= '</section>';

		return $out;
	}

	/**
	 * Inner-block READ helper (W1W2 L2 cycle-2 split): claim-row inner blocks
	 * invoke `claim_receipt` (audience-keyed receipt builder via
	 * build_receipt_for_audience per DOS-341) through this hook to fetch
	 * the audience-scoped receipt for one claim_ref during render.
	 *
	 * **Render-path only.** This helper MUST NOT mutate state. The previous
	 * `..._claim_inner_consumer` shape unconditionally also fired
	 * `record_claim_feedback`, which is a write surface. Reads-on-render
	 * cannot emit writes; the write path is now `_claim_inner_write` below
	 * and is called explicitly from feedback affordance handlers.
	 *
	 * AgentMcp audience filter is enforced inside build_receipt_for_audience
	 * server-side; consumers pass the claim_ref unmodified.
	 *
	 * @param array<string, mixed> $claim_ref Claim reference from projection
	 *                                         ({ claim_id, audience_key, ... }).
	 * @return array<string, mixed> Receipt envelope (raw runtime response;
	 *                              typed shaping at inner block).
	 */
	function dailyos_account_detail_claim_inner_read( array $claim_ref ): array {
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

		return [
			'receipt' => $receipt_response,
		];
	}

	/**
	 * Inner-block WRITE helper (W1W2 L2 cycle-2 split): explicit feedback
	 * write surface for the account-detail subtree. Called from feedback
	 * affordance handlers (accept/reject/correct/dismiss buttons,
	 * MergeIntent submitters) — NEVER from the render path. The render
	 * path uses `_claim_inner_read` above.
	 *
	 * @param array<string, mixed> $claim_ref Claim reference
	 *                                        ({ claim_id, audience_key, ... }).
	 * @param string               $action    Feedback action variant
	 *                                        (e.g., 'confirm_current').
	 * @param array<string, mixed> $metadata  Optional metadata (payload_json
	 *                                        body — empty {} is valid).
	 * @return array<string, mixed> Feedback runtime response.
	 */
	function dailyos_account_detail_claim_inner_write(
		array $claim_ref,
		string $action,
		array $metadata = []
	): array {
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

		// W1 producer: record_claim_feedback — explicit feedback write. The
		// action + metadata are carried alongside the claim_ref so the
		// runtime persists the typed feedback row.
		$payload = $claim_ref;
		$payload['action']   = $action;
		$payload['metadata'] = $metadata;

		$feedback_response = $runtime_client->invoke_ability(
			'record_claim_feedback',
			$payload,
			$scope_set
		);

		return [
			'feedback' => $feedback_response,
		];
	}
}
