<?php
/**
 * Shared envelope resolver — per-request DOS-477 envelope cache shim.
 *
 * Per L0 packet W2 V1.2.1 §5.1 "envelopeHandle resolution contract":
 *
 *   1. The outer entity-detail block invokes get_entity_intelligence ONCE per
 *      render through the runtime client (the 3-arg invoke_ability signature
 *      enforced by check_w1_consumer_skeleton.sh).
 *   2. The producer-side DOS-477 envelope_cache keys the result by
 *      (envelope_render_id, actor_principal_id, surface).
 *   3. The outer block emits dailyos/envelopeHandle into block context.
 *   4. Inner blocks declare usesContext: ["dailyos/envelopeHandle"] and call
 *      dailyos_resolve_envelope(...) below, which short-circuits via the
 *      request-scoped cache to the response already paid for upstream.
 *
 * This file is the PHP-side request-scoped cache (lifetime = single PHP
 * request; evicts on response close). DOS-477 owns the producer-side cache.
 * The two together satisfy "envelope fetched once per request" per V1.2.1.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return;
}

if ( ! function_exists( 'dailyos_envelope_cache_get' ) ) {
	/**
	 * Read a cached envelope by handle. Returns null on miss.
	 *
	 * @param string $handle Envelope handle (server-emitted envelope_render_id).
	 * @return array<string,mixed>|null
	 */
	function dailyos_envelope_cache_get( string $handle ): ?array {
		if ( '' === $handle ) {
			return null;
		}
		static $cache = [];
		if ( isset( $cache[ $handle ] ) ) {
			return $cache[ $handle ];
		}
		$shared = dailyos_envelope_cache_storage();
		return $shared[ $handle ] ?? null;
	}
}

if ( ! function_exists( 'dailyos_envelope_cache_put' ) ) {
	/**
	 * Store an envelope by handle for the remainder of the request.
	 *
	 * @param string               $handle Envelope handle.
	 * @param array<string,mixed>  $envelope Envelope payload (full
	 *                                       EntityIntelligenceEnvelope shape).
	 */
	function dailyos_envelope_cache_put( string $handle, array $envelope ): void {
		if ( '' === $handle ) {
			return;
		}
		$shared             = &dailyos_envelope_cache_storage();
		$shared[ $handle ]  = $envelope;
	}
}

if ( ! function_exists( 'dailyos_envelope_cache_storage' ) ) {
	/**
	 * Backing storage for the request-scoped envelope cache. Returned by
	 * reference so put can mutate.
	 *
	 * @return array<string,array<string,mixed>>
	 */
	function &dailyos_envelope_cache_storage(): array {
		static $storage = [];
		return $storage;
	}
}

if ( ! function_exists( 'dailyos_envelope_handle_from_response' ) ) {
	/**
	 * Extract the envelope_render_id (or fall back to a deterministic hash)
	 * from a get_entity_intelligence response and cache the envelope under it.
	 *
	 * @param array<string,mixed> $response Runtime client response (decoded).
	 * @param string              $entity_type Entity kind (account/project/...).
	 * @param string              $entity_id   Entity id.
	 * @return string Envelope handle (empty string on degenerate input).
	 */
	function dailyos_envelope_handle_from_response( array $response, string $entity_type, string $entity_id ): string {
		// Runtime returns { ok, request_id, ability: { ability_name, data, ... } }
		// per src-tauri/src/bridges/types.rs::AbilityResponseJson — the actual
		// envelope lives at $response['ability']['data']. The 'envelope' and
		// 'data' top-level fallbacks remain for legacy callers that pre-unwrap.
		$ability  = $response['ability'] ?? null;
		$envelope = $response['envelope']
			?? $response['data']
			?? ( is_array( $ability ) ? ( $ability['data'] ?? null ) : null )
			?? $response;
		if ( ! is_array( $envelope ) ) {
			return '';
		}
		$handle = '';
		if ( isset( $envelope['envelopeRenderId'] ) && is_string( $envelope['envelopeRenderId'] ) ) {
			$handle = $envelope['envelopeRenderId'];
		} elseif ( isset( $envelope['envelope_render_id'] ) && is_string( $envelope['envelope_render_id'] ) ) {
			$handle = $envelope['envelope_render_id'];
		} else {
			// Fall back to a deterministic per-envelope handle so inner blocks
			// can still resolve via the request-scoped cache. Substrate carries
			// envelope_render_id once DOS-477 envelope-cache wire-up surfaces
			// it on the response; until then this hash keeps the consumer side
			// single-fetch.
			//
			// W1W2 L2 cycle-2 MEDIUM fix: the previous fallback used
			// `spl_object_hash( (object) $envelope )` which casts the array to
			// a FRESH stdClass on every call. PHP allocates a new object per
			// call, so the object-hash differs each invocation — the outer
			// block emitted one handle, the inner block (re-deriving from the
			// same envelope shape) got a DIFFERENT handle, and the cache
			// missed. Net effect: every inner block re-invoked the producer
			// instead of reusing the cached envelope, multiplying ability
			// invocations N-times per render.
			//
			// `md5( serialize( $envelope ) )` is deterministic on the
			// envelope's data shape (associative arrays serialize in insertion
			// order, identical envelopes hash identically), and the cost is
			// dominated by the serialize() of an already-in-memory array —
			// negligible compared to the saved ability round-trip.
			$handle = hash(
				'sha256',
				$entity_type . '|' . $entity_id . '|' . md5( serialize( $envelope ) )
			);
		}
		dailyos_envelope_cache_put( $handle, $envelope );
		return $handle;
	}
}

if ( ! function_exists( 'dailyos_resolve_envelope' ) ) {
	/**
	 * Resolve an envelope for an inner block. Hits the request-scoped cache
	 * by handle first; on miss invokes the producer with the 3-arg signature
	 * (which itself short-circuits via the DOS-477 producer cache when keyed
	 * the same way). Returns null when no envelope is reachable — inner
	 * blocks render empty-state chips per §10 invariant.
	 *
	 * @param string|null $handle     Envelope handle from block context.
	 * @param string      $entity_type Entity kind (account/project/person/meeting).
	 * @param string      $entity_id   Entity id from block context.
	 * @param array       $scope_set   Resolved scope set for the surface client.
	 * @return array<string,mixed>|null
	 */
	function dailyos_resolve_envelope( ?string $handle, string $entity_type, string $entity_id, array $scope_set ): ?array {
		if ( null !== $handle && '' !== $handle ) {
			$cached = dailyos_envelope_cache_get( $handle );
			if ( null !== $cached ) {
				return $cached;
			}
		}
		if ( '' === $entity_id ) {
			return null;
		}
		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return null;
		}
		$response = $runtime_client->invoke_ability(
			'get_entity_intelligence',
			[
				'entity_type' => $entity_type,
				'entity_id'   => $entity_id,
				'depth'       => 'Full',
				'sections'    => null,
			],
			$scope_set
		);
		if ( ! is_array( $response ) ) {
			return null;
		}
		$resolved_handle = dailyos_envelope_handle_from_response( $response, $entity_type, $entity_id );
		if ( '' === $resolved_handle ) {
			return null;
		}
		return dailyos_envelope_cache_get( $resolved_handle );
	}
}

if ( ! function_exists( 'dailyos_envelope_section' ) ) {
	/**
	 * Lookup a named section in the envelope's sections map, return a flat
	 * descriptor: [ 'present' => bool, 'item_count' => int, 'reason' => string ].
	 *
	 * @param array<string,mixed>|null $envelope Envelope payload.
	 * @param string                   $section Section key (snake_case).
	 * @return array{present:bool,item_count:int,reason:string}
	 */
	function dailyos_envelope_section( ?array $envelope, string $section ): array {
		$default = [
			'present'    => false,
			'item_count' => 0,
			'reason'     => 'not_available',
		];
		if ( null === $envelope ) {
			return $default;
		}
		$sections = $envelope['sections'] ?? [];
		if ( ! is_array( $sections ) ) {
			return $default;
		}
		$state = $sections[ $section ] ?? null;
		if ( ! is_array( $state ) ) {
			return $default;
		}
		$kind = $state['kind'] ?? '';
		if ( 'present' === $kind ) {
			return [
				'present'    => true,
				'item_count' => (int) ( $state['item_count'] ?? $state['itemCount'] ?? 0 ),
				'reason'     => '',
			];
		}
		$reason = '';
		if ( isset( $state['reason'] ) ) {
			$reason = is_array( $state['reason'] )
				? (string) ( $state['reason']['kind'] ?? 'unknown' )
				: (string) $state['reason'];
		}
		return [
			'present'    => false,
			'item_count' => 0,
			'reason'     => '' === $reason ? 'empty' : $reason,
		];
	}
}

if ( ! function_exists( 'dailyos_empty_chip' ) ) {
	/**
	 * Render the canonical empty-state chip mandated by V1.1 §10 invariant
	 * "every inner block renders empty as quiet chip with data-empty-reason;
	 * no silent hidden states; inherits envelope Empty { reason }".
	 *
	 * @param string $reason Empty-state reason (snake_case).
	 * @param string $label  Localized human label.
	 * @param string $block_class Block-scoped CSS class (e.g.,
	 *                            "dailyos-stakeholder-grid").
	 * @return string
	 */
	function dailyos_empty_chip( string $reason, string $label, string $block_class ): string {
		$safe_reason = esc_attr( '' === $reason ? 'empty' : $reason );
		$safe_label  = esc_html( $label );
		$safe_class  = esc_attr( $block_class );
		return '<div class="' . $safe_class . ' ' . $safe_class . '--empty">'
			. '<span class="dailyos-empty-chip" data-empty-reason="' . $safe_reason . '">'
			. $safe_label
			. '</span>'
			. '</div>';
	}
}

if ( ! function_exists( 'dailyos_inner_block_wrapper_attrs' ) ) {
	/**
	 * Build wrapper attrs for an inner block. Per V1.1 §10 invariant
	 * "check_no_inline_style_exception.sh", the only permitted inline-style
	 * value is a `--dailyos-*: var(--dailyos-*);` custom-property assignment.
	 * This helper returns either the standard wrapper attrs or, when the
	 * outer block context carries a CSS custom property (e.g.,
	 * --dailyos-project-tint), forwards it onto inner blocks via
	 * get_block_wrapper_attributes — never raw inline styles.
	 *
	 * @param string                 $block_class Block-scoped CSS class.
	 * @param array<string,string>   $custom_properties Optional inline-style
	 *                                                 custom-property pairs.
	 * @return string Rendered attribute string suitable for echo.
	 */
	function dailyos_inner_block_wrapper_attrs( string $block_class, array $custom_properties = [] ): string {
		$args = [
			'class' => $block_class,
		];
		if ( ! empty( $custom_properties ) ) {
			$style_segments = [];
			foreach ( $custom_properties as $key => $value ) {
				if ( 0 !== strpos( $key, '--dailyos-' ) ) {
					continue; // discipline per §10 inline-style allowlist
				}
				$style_segments[] = $key . ': ' . $value;
			}
			if ( ! empty( $style_segments ) ) {
				$args['style'] = implode( '; ', $style_segments );
			}
		}
		if ( function_exists( 'get_block_wrapper_attributes' ) ) {
			return get_block_wrapper_attributes( $args );
		}
		$out = 'class="' . esc_attr( $args['class'] ) . '"';
		if ( isset( $args['style'] ) ) {
			$out .= ' style="' . esc_attr( $args['style'] ) . '"';
		}
		return $out;
	}
}

if ( ! function_exists( 'dailyos_envelope_consume_claim' ) ) {
	/**
	 * Pass a claim reference through claim_receipt via the runtime client
	 * (AgentMcp audience filter is enforced server-side by
	 * build_receipt_for_audience per DOS-341). Returns the receipt payload
	 * or null on failure / unavailable runtime. Inner blocks invoke this
	 * for the rendered claim-row receipts called out in AC-462.3.
	 *
	 * @param array<string,mixed> $claim_ref { claim_id, audience_key, ... }.
	 * @param array<string|int,mixed> $scope_set Resolved scope set.
	 * @return array<string,mixed>|null
	 */
	function dailyos_envelope_consume_claim( array $claim_ref, array $scope_set ): ?array {
		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return null;
		}

		// Shape the claim_ref into the registered claim_receipt ability's
		// input contract: { schemaVersion, target: { kind, claimId, ... },
		// surface }. Without this shaping the ability decoder rejects the
		// payload as a contract violation rather than rendering the receipt.
		$claim_id = isset( $claim_ref['claim_id'] ) ? (string) $claim_ref['claim_id'] : '';
		if ( '' === $claim_id ) {
			return null;
		}
		$subject_ref = $claim_ref['subject_ref'] ?? null;
		if ( ! is_array( $subject_ref ) ) {
			return null;
		}
		$target = [
			'kind'    => 'claim',
			'claimId' => $claim_id,
			'subject' => $subject_ref,
		];
		if ( ! empty( $claim_ref['field_path'] ) ) {
			$target['fieldPath'] = (string) $claim_ref['field_path'];
		}
		// Envelope readers run inside the WP block render path, which
		// corresponds to entity_detail for the account-detail / project-detail
		// inner blocks unless the caller overrides via claim_ref['surface'].
		$surface = isset( $claim_ref['surface'] ) ? (string) $claim_ref['surface'] : 'entity_detail';
		$payload = [
			'schemaVersion' => 1,
			'target'        => $target,
			'surface'       => $surface,
		];

		$response = $runtime_client->invoke_ability( 'claim_receipt', $payload, $scope_set );
		return is_array( $response ) ? $response : null;
	}
}
